// yagami-runtime: the simulation child process. A thin layer over the lawliet engine that also
// owns the server's sim state (keys, profiles) -- what used to be yagami's "shell". Hosting both in
// one deterministic process means a rebuild (re-feeding the accepted input stream) reconstructs
// engine state, sim state, AND the output stream identically, so yagami needs to persist only the
// input log.
//
// The runtime knows nothing of time travel: yagami orchestrates a rewind by truncating the accepted
// stream and re-feeding it to a fresh runtime. It authorizes every input it sees (it holds the
// keys); the one thing it never sees is a meta control (GoToTime), which yagami handles itself.

use std::collections::HashMap;

use lawliet::engine::Engine;
use lawliet_types::{
    action::{Action, ActionActor, ActionRequest, AddPlayer, SetTrueName},
    command::{Command, CommandPayload, CommandRecipient},
    common::{ActorKey, Time, Version, ViewportKey},
};
use rand_pcg::Pcg64;
use rand_pcg::rand_core::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use yagami_wire::{
    ActionOutcome, AdminControl, Capability, ControlError, ControlOutcome, ControlResponse,
    ExecOutcome, Key, Output, OutputData, Privileges, Profile, Recipient, ResponsePair,
    ServerInput, SimControl, SimControlData, SimOutput,
};

// The names the runtime draws display names from. First and last are kept apart so N of each yields
// N*N people. ASCII letters only: a true name is compared for equality and typed from memory.
const FIRST: &str = include_str!("../../names/first.txt");
const LAST: &str = include_str!("../../names/last.txt");

// How many random pairs to try before giving up on luck and sweeping. Far more than a game with a
// full lobby will ever need.
const RANDOM_ATTEMPTS: usize = 32;

fn parse(list: &str) -> Vec<&str> {
    list.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
}

// The reservoir the runtime draws names from. First and last are kept apart so N of each yields
// N*N people. ASCII letters only: a true name is compared for equality and typed from memory. The
// pool owns only the list of taken names; the RNG is the simulation's single seeded stream, shared
// with key generation, so the whole sim is deterministic from the engine seed.
struct NamePool {
    first: Vec<&'static str>,
    last: Vec<&'static str>,
    taken: Vec<String>,
}

impl NamePool {
    fn new() -> Self {
        let pool = NamePool {
            first: parse(FIRST),
            last: parse(LAST),
            taken: Vec::new(),
        };
        assert!(
            !pool.first.is_empty() && !pool.last.is_empty(),
            "name reservoir is empty: names/first.txt and names/last.txt must each hold at least one name"
        );
        pool
    }

    // draw a previously-unhanded name, consuming from the shared sim RNG. returns None when every
    // pair in the reservoir is spoken for.
    fn draw(&mut self, rng: &mut Pcg64) -> Option<String> {
        for _ in 0..RANDOM_ATTEMPTS {
            let f = (rng.next_u64() % self.first.len() as u64) as usize;
            let l = (rng.next_u64() % self.last.len() as u64) as usize;
            let candidate = format!("{} {}", self.first[f], self.last[l]);
            if !self.taken.contains(&candidate) {
                self.taken.push(candidate.clone());
                return Some(candidate);
            }
        }
        for f in &self.first {
            for l in &self.last {
                let candidate = format!("{f} {l}");
                if !self.taken.contains(&candidate) {
                    self.taken.push(candidate.clone());
                    return Some(candidate);
                }
            }
        }
        None
    }

    // record a name the world already holds, so a later draw never hands it out again.
    fn mark_taken(&mut self, name: &str) {
        if !self.taken.iter().any(|n| n == name) {
            self.taken.push(name.to_string());
        }
    }

    // reset for a rebuild replay: the sim RNG is re-seeded, so draws will reproduce, and the taken
    // set is rebuilt from the replayed commands.
    fn reset(&mut self) {
        self.taken.clear();
    }
}

// What yagami feeds the runtime over the pipe: the input, the engine version that must interpret
// it, and the caller's key (None for a server-issued input or a rebuild replay, which are trusted).
// Meta controls never appear here.
#[derive(Serialize, Deserialize, Clone)]
pub struct PipeFrame {
    pub input: RuntimeInput,
    pub version: Version,
    pub caller: Option<Key>,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum RuntimeInput {
    Action(ActionRequest),
    Sim(SimControl),
}

// Convert a ServerInput into the RuntimeInput the runtime actually processes, if it is one the
// runtime sees. Meta controls are yagami's concern (time travel) and never reach the runtime.
pub fn runtime_input(input: &ServerInput) -> Option<RuntimeInput> {
    match input {
        ServerInput::Action(a) => Some(RuntimeInput::Action(a.clone())),
        ServerInput::Control(AdminControl::Sim(sim)) => Some(RuntimeInput::Sim(sim.clone())),
        // meta controls are yagami's concern; they never reach the runtime.
        ServerInput::Control(AdminControl::Meta(_)) => None,
    }
}

// What the runtime hands back per input: the Outputs to append to history (engine commands and sim
// rosters, each already addressed via `recipients`), and -- for the originating connection -- the
// reply that settles its positional queue.
#[derive(Serialize, Deserialize)]
pub struct RuntimeOutput {
    pub outputs: Vec<Output>,
    pub reply: Option<Reply>,
}

#[derive(Serialize, Deserialize)]
pub struct Reply {
    pub outcome: ExecOutcome,
    pub input: ServerInput,
}

impl Reply {
    // convert to yagami's ResponsePair (same shape, different name).
    pub fn into_pair(self) -> ResponsePair {
        ResponsePair {
            response: self.outcome,
            input: self.input,
        }
    }
}

pub struct Simulation {
    engine: Engine,
    keys: HashMap<Key, Privileges>,
    profiles: HashMap<ActorKey, Profile>,
    data_viewport: Option<ViewportKey>,
    // the simulation's seeded RNG: drives true-name draws, display-name draws, and key
    // generation. one stream, consumed in stream order, so a rebuild that re-feeds the same inputs
    // in the same order reproduces every draw and every key. the game task rotates it (a ReSeed
    // control) after any name leaks, so names drawn after a leak are independent of those before.
    sim_rng: Pcg64,
    display_names: NamePool,
    true_names: NamePool,
}

impl Simulation {
    pub fn new() -> Self {
        // seeded afresh by the InitializeEngine action; the sim RNG is re-seeded from the same seed
        // when that action runs, so a rebuild reproduces draws + keys.
        Simulation {
            engine: Engine::new(),
            keys: HashMap::new(),
            profiles: HashMap::new(),
            data_viewport: None,
            sim_rng: Pcg64::seed_from_u64(0),
            display_names: NamePool::new(),
            true_names: NamePool::new(),
        }
    }

    pub fn keys(&self) -> &HashMap<Key, Privileges> {
        &self.keys
    }

    // re-seed the simulation RNG from the engine seed and reset the name pools, so a rebuild
    // reproduces the same draws. called when InitializeEngine runs.
    pub fn reseed_sim(&mut self, seed: u64) {
        self.sim_rng = Pcg64::seed_from_u64(seed);
        self.display_names.reset();
        self.true_names.reset();
    }

    // rotate the simulation RNG to a fresh seed, on a ReSeed control. the taken sets are left
    // alone: rotations happen mid-game and must keep avoiding names already in play, and on a
    // rebuild replay they are repopulated from the replayed draws.
    pub fn reseed_rng(&mut self, seed: u64) {
        self.sim_rng = Pcg64::seed_from_u64(seed);
    }

    // process one input. `caller` is the connection's key (None for a server-issued input or a
    // rebuild replay). `version` is the engine version the input was recorded under, threaded into
    // the engine so an action executes under its own semantics. the returned outputs are gated
    // ServerOutputs ready to append to history; `reply` carries the outcome for the originating
    // connection.
    pub fn process(
        &mut self,
        input: &RuntimeInput,
        caller: Option<&Key>,
        version: Version,
    ) -> RuntimeOutput {
        match input {
            RuntimeInput::Action(request) => {
                let mut out = self.process_action(request, caller, version);
                if request.actor != ActionActor::System {
                    out.outputs.push(Output {
                        time: self.engine.time,
                        recipients: vec![Recipient::Admin],
                        data: OutputData::Sim(SimOutput::LogAction {
                            action: request.clone(),
                        }),
                    });
                }
                out
            }
            RuntimeInput::Sim(control) => self.process_sim(control, caller),
        }
    }

    fn process_action(
        &mut self,
        request: &ActionRequest,
        caller: Option<&Key>,
        version: Version,
    ) -> RuntimeOutput {
        // authorize: a server-issued input (None) always passes; a connection's input must hold the
        // actor under its key.
        if !self.authorize_action(caller, &request.actor) {
            return RuntimeOutput {
                outputs: vec![],
                reply: Some(Reply {
                    outcome: ExecOutcome::Action(ActionOutcome::Denied),
                    input: ServerInput::Action(request.clone()),
                }),
            };
        }

        // the server names people, not their clients. an unnamed AddPlayer/SetTrueName gets a fresh
        // true-name draw before the engine sees it; a TrueNameReroll ALWAYS replaces whatever the
        // client sent. deterministic from the engine seed, so a rebuild reproduces the same names.
        let mut request = request.clone();
        self.assign_true_name(&mut request);

        let result = self.engine.execute(request.clone(), version);
        let (outcome, context) = match result {
            Ok((response, ctx)) => (ActionOutcome::Ok(response), ctx),
            Err((error, ctx)) => (ActionOutcome::Err(error), ctx),
        };

        // learn the world-data viewport from its (once-per-boot) announcement.
        self.record_data_viewport(&context.commands);
        // tell the true-name pool about every name the world now holds (admin-set or drawn), so a
        // later draw never hands out one already in play.
        self.record_true_names(&context.commands);
        // a freshly-mapped PLAYER slot draws a display name before the roster is broadcast.
        let profiles_changed = self.assign_display_names(&context.commands);

        // engine commands become addressed Outputs, in order.
        let mut outputs: Vec<Output> = context.commands.iter().map(engine_to_output).collect();

        if profiles_changed && let Some(out) = self.profile_roster_output(request.timestamp) {
            outputs.push(out);
        }

        RuntimeOutput {
            outputs,
            reply: Some(Reply {
                outcome: ExecOutcome::Action(outcome),
                input: ServerInput::Action(request.clone()),
            }),
        }
    }

    fn process_sim(&mut self, control: &SimControl, caller: Option<&Key>) -> RuntimeOutput {
        let time = control.time;
        let result = match &control.data {
            SimControlData::CreateKey {
                actors,
                capabilities,
            } => {
                let caps = yagami_wire::to_flags(capabilities);
                // a trusted server-issued input (bootstrap, caller None) may grant Supervise; a live
                // connection may only if it already supervises.
                if caps.contains(Capability::Supervise)
                    && caller.is_some()
                    && !self.supervises(caller)
                {
                    return self.sim_denied(control, ControlError::CannotGrantSupervise, time);
                }
                // the key is generated from the sim RNG -- deterministic on replay, never on the
                // wire.
                let key = self.generate_key();
                self.keys.insert(
                    key.clone(),
                    Privileges {
                        actors: actors.clone(),
                        capabilities: caps,
                    },
                );
                Ok(ControlResponse::KeyCreated { key })
            }
            SimControlData::RevokeKey { key } => {
                if let Err(e) = self.may_manage(caller, key) {
                    return self.sim_denied(control, e, time);
                }
                self.keys.remove(key);
                Ok(ControlResponse::KeyRevoked)
            }
            SimControlData::SetCapabilities { key, capabilities } => {
                if let Err(e) = self.may_manage(caller, key) {
                    return self.sim_denied(control, e, time);
                }
                let caps = yagami_wire::to_flags(capabilities);
                if caps.contains(Capability::Supervise)
                    && caller.is_some()
                    && !self.supervises(caller)
                {
                    return self.sim_denied(control, ControlError::CannotGrantSupervise, time);
                }
                if let Some(p) = self.keys.get_mut(key) {
                    p.capabilities = caps;
                }
                Ok(ControlResponse::CapabilitiesSet)
            }
            SimControlData::SetActorScope { key, actors } => {
                if let Err(e) = self.may_manage(caller, key) {
                    return self.sim_denied(control, e, time);
                }
                if let Some(p) = self.keys.get_mut(key) {
                    p.actors = actors.clone();
                }
                Ok(ControlResponse::ActorScopeSet)
            }
            SimControlData::SetProfile { actor, profile } => {
                self.profiles.insert(*actor, profile.clone());
                Ok(ControlResponse::ProfileSet)
            }
            SimControlData::ReSeed { seed } => {
                self.reseed_rng(*seed);
                Ok(ControlResponse::ReSeed)
            }
            SimControlData::GetVersion => Ok(ControlResponse::EngineVersion(Engine::version())),
        };

        let (outcome, outputs) = match result {
            Ok(response) => {
                // the roster(s) the change rewrites, gated, become the input's outputs.
                let mut outputs = Vec::new();
                let is_key_change = matches!(
                    control.data,
                    SimControlData::CreateKey { .. }
                        | SimControlData::RevokeKey { .. }
                        | SimControlData::SetCapabilities { .. }
                        | SimControlData::SetActorScope { .. }
                );
                if is_key_change {
                    outputs.push(self.key_roster_output(time));
                }
                if matches!(control.data, SimControlData::SetProfile { .. }) {
                    if let Some(out) = self.profile_roster_output(time) {
                        outputs.push(out);
                    }
                }
                (ControlOutcome::Ok(response), outputs)
            }
            Err(e) => (ControlOutcome::Err(e), Vec::new()),
        };

        RuntimeOutput {
            outputs,
            reply: Some(Reply {
                outcome: ExecOutcome::Control(outcome),
                input: ServerInput::Control(AdminControl::Sim(control.clone())),
            }),
        }
    }

    // authorize an action's actor against the caller's privileges. a server-issued input (None) and
    // System both pass -- yagami's own voice, never arriving on a connection.
    fn authorize_action(&self, caller: Option<&Key>, actor: &ActionActor) -> bool {
        match caller {
            None => true,
            Some(c) => match actor {
                ActionActor::System => true,
                _ => self.keys.get(c).is_some_and(|p| p.can_act_as(actor)),
            },
        }
    }

    // may the caller act on this target key? the single authority rule for every key-management
    // control: a key holding Administer is reachable only from a Supervise holder, and a Supervise
    // holder cannot reach its own key -- so the LAST Supervise holder can be neither revoked nor
    // demoted, by anyone.
    fn may_manage(&self, caller: Option<&Key>, target: &Key) -> Result<(), ControlError> {
        let Some(target_privs) = self.keys.get(target) else {
            return Err(ControlError::KeyNotFound);
        };
        let caller_key = match caller {
            Some(c) => c,
            None => return Ok(()), // server-issued, trusted
        };
        let supervises = self.supervises(caller);

        if target == caller_key {
            return if supervises {
                Err(ControlError::CannotActOnSelf)
            } else {
                Ok(())
            };
        }
        if target_privs.administers() && !supervises {
            return Err(ControlError::RequiresSupervise);
        }
        Ok(())
    }

    fn supervises(&self, caller: Option<&Key>) -> bool {
        caller
            .and_then(|c| self.keys.get(c))
            .is_some_and(|p| p.capabilities.contains(Capability::Supervise))
    }

    // generate a key from the sim RNG. 256 bits of entropy, hex-encoded -- same shape as
    // yagami_wire::generate_token, but drawn from the deterministic stream.
    fn generate_key(&mut self) -> Key {
        let mut bytes = [0u8; 32];
        for chunk in bytes.chunks_mut(8) {
            chunk.copy_from_slice(&self.sim_rng.next_u64().to_le_bytes());
        }
        let mut s = String::with_capacity(bytes.len() * 2);
        for b in bytes {
            use std::fmt::Write;
            write!(s, "{b:02x}").unwrap();
        }
        Key::from_token(s)
    }

    fn sim_denied(&self, control: &SimControl, error: ControlError, _time: Time) -> RuntimeOutput {
        RuntimeOutput {
            outputs: vec![],
            reply: Some(Reply {
                outcome: ExecOutcome::Control(ControlOutcome::Err(error)),
                input: ServerInput::Control(AdminControl::Sim(control.clone())),
            }),
        }
    }

    // ===== output construction ===== //

    // the server names people, not their clients. an unnamed AddPlayer/SetTrueName gets a fresh
    // draw; a TrueNameReroll ALWAYS replaces whatever the client sent (letting the user choose is
    // the whole thing being prevented). a name a request carries is kept as sent. an exhausted
    // reservoir leaves the request untouched, and the engine refuses it as a duplicate.
    fn assign_true_name(&mut self, request: &mut ActionRequest) {
        let unnamed = match &mut request.payload {
            Action::UseAbility(use_ability) => match &mut use_ability.ability_args {
                lawliet_types::ability::AbilityBehaviour::TrueNameReroll(reroll) => {
                    Some(&mut reroll.true_name)
                }
                _ => None,
            },
            Action::AddPlayer(AddPlayer { true_name, .. }) if true_name.is_empty() => {
                Some(true_name)
            }
            Action::SetTrueName(SetTrueName { true_name, .. }) if true_name.is_empty() => {
                Some(true_name)
            }
            _ => None,
        };
        if let Some(slot) = unnamed
            && let Some(name) = self.true_names.draw(&mut self.sim_rng)
        {
            *slot = name;
        }
    }

    // tell the true-name pool about every name the world now holds, so a later draw never hands out
    // one already in play -- whether it was drawn here or typed by an admin straight into the action.
    fn record_true_names(&mut self, commands: &[CommandPayload]) {
        for payload in commands {
            if let Command::TrueNameUpdate { true_name, .. } = &payload.cmd {
                self.true_names.mark_taken(true_name);
            }
        }
    }

    fn record_data_viewport(&mut self, commands: &[CommandPayload]) {
        for payload in commands {
            if let Command::MapViewport { viewport, kind } = &payload.cmd
                && *kind == lawliet_types::viewport::ViewportKind::WorldData
            {
                self.data_viewport = Some(*viewport);
            }
        }
    }

    // a freshly-mapped PLAYER slot with no profile yet draws a display name. cosmetic, drawn
    // independently of the true name. returns whether any profile changed.
    fn assign_display_names(&mut self, commands: &[CommandPayload]) -> bool {
        let mut changed = false;
        for payload in commands {
            let Command::MapActor {
                actor_id,
                kind: lawliet_types::actor::ActorKind::Player,
            } = &payload.cmd
            else {
                continue;
            };
            if self.profiles.contains_key(actor_id) {
                continue;
            }
            let Some(display_name) = self.display_names.draw(&mut self.sim_rng) else {
                continue;
            };
            self.profiles.insert(
                *actor_id,
                Profile {
                    display_name: Some(display_name),
                },
            );
            changed = true;
        }
        changed
    }

    fn key_roster_output(&self, time: Time) -> Output {
        let keys: Vec<(Key, yagami_wire::PrivilegeSet)> = self
            .keys
            .iter()
            .map(|(key, privileges)| (key.clone(), yagami_wire::privileges_to_wire(privileges)))
            .collect();
        Output {
            recipients: vec![Recipient::Admin],
            data: OutputData::Sim(SimOutput::KeyRoster { keys }),
            time,
        }
    }

    fn profile_roster_output(&self, time: Time) -> Option<Output> {
        let viewport = self.data_viewport?;
        let profiles: Vec<(ActorKey, Profile)> =
            self.profiles.iter().map(|(k, v)| (*k, v.clone())).collect();
        Some(Output {
            recipients: vec![Recipient::Viewport(viewport), Recipient::Admin],
            data: OutputData::Sim(SimOutput::ProfileRoster { profiles }),
            time,
        })
    }
}

// convert an engine payload to its stored Output. mirrors the engine's own recipient addressing:
// system -> admin, viewport -> viewport+admin, actor -> player, log -> log (indexed for dumps,
// delivered to nobody live).
fn engine_to_output(cmd: &CommandPayload) -> Output {
    let recipients = match &cmd.recipient {
        CommandRecipient::System => vec![Recipient::Admin],
        CommandRecipient::Viewport(viewport) => {
            vec![Recipient::Viewport(*viewport), Recipient::Admin]
        }
        CommandRecipient::Actor(id) => vec![Recipient::Player(*id)],
        CommandRecipient::Log(log) => vec![Recipient::Log(*log)],
    };
    Output {
        time: cmd.timestamp,
        recipients,
        data: OutputData::Engine(cmd.cmd.clone()),
    }
}
