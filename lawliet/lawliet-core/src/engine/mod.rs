use rand_pcg::Pcg32;
use rand_pcg::rand_core::SeedableRng;

use crate::Time;
use crate::action::{
    ActionContext, ActionError, ActionExt, ActionRequest, ActionResponse, ActionResult,
};
use crate::command::DeferredCommand;
use crate::config::Config;
use crate::engine::jobs::Jobs;
use crate::world::World;

pub mod jobs;

pub struct Engine {
    pub world: World,
    pub config: Config,
    pub time: Time,
    pub jobs: Jobs,
    pub deferred_commands: Vec<DeferredCommand>,
    pub rng_state: Pcg32,
    // set once InitializeEngine has run; guards against re-initialization.
    pub initialized: bool,
}

pub type ExecutionResult =
    Result<(ActionResponse, ActionContext), (ActionError, ActionContext)>;

impl Engine {
    pub fn new() -> Self {
        Engine {
            world: World::new(),
            config: Config::new(),
            jobs: Jobs::new(),
            deferred_commands: vec![],
            time: 0,
            rng_state: Pcg32::seed_from_u64(0),
            initialized: false,
        }
    }

    pub fn schedule(&mut self, request: ActionRequest) {
        self.jobs.push(request);
    }

    pub fn is_future_timestamp(&self, timestamp: Time) -> bool {
        timestamp >= self.time
    }

    pub fn defer_cmd(&mut self, payload: DeferredCommand) {
        self.deferred_commands.push(payload);
    }

    // attempt to execute an action atomically
    // first run a validation pass. this will propagate any sub action failures upwards without
    // modifying game state. after this, run the execution pass. this will crash on failure
    // (although this should never happen in practice due to the validation pass).
    // overflows are a non-issue. no action creates large enough of a chain to naturally overflow
    // the stack. if a stack overflow occurs it is due to an infinite recursion bug, and in this
    // case, a crash is necessary.
    fn execute_atomic(
        &mut self,
        ctx: &mut ActionContext,
        mut action: ActionRequest,
    ) -> ActionResult {
        let old_time = self.time;
        self.time = action.timestamp;
        // prevent validation mutation on action context
        let dry_result = action.payload.validate(self, ctx, &action.actor, 0);
        if dry_result.is_err() {
            self.time = old_time;
            return dry_result;
        }
        self.time = action.timestamp;
        ctx.commands.clear();
        let result = action.payload.execute(self, ctx, &action.actor, 0);
        result
            .as_ref()
            .expect("Validate and execute pass desync detected.");
        result
    }

    // store a command buffer
    // check action timestamp and execute any pending jobs that happen before/at the timestamp
    // execute the requested action
    // recursively execute sub-actions
    // return only top level result (with the combined command buffer)
    pub fn execute(&mut self, action: ActionRequest) -> ExecutionResult {
        let mut ctx = ActionContext { commands: vec![], mutate: false };

        if action.timestamp < self.time {
            return Err((ActionError::TimeAlreadyPassed, ctx));
        }

        // Commands are emitted in push order; local ordering needs (e.g. perms before
        // members) are handled with reversed scopes inside the relevant actions, so no
        // global reversal happens here. execute_atomic resets ctx between passes, so
        // drain each job's commands into a separate buffer as we go.
        let mut commands = Vec::new();
        loop {
            if self.jobs.is_empty() {
                break;
            }

            let job = self.jobs.peek().unwrap();
            if job.request.timestamp > action.timestamp {
                break;
            }

            // ignore the errors of scheduled jobs.
            let job = self.jobs.pop().unwrap();
            let _ = self.execute_atomic(&mut ctx, job.request);
            commands.append(&mut ctx.commands);
        }

        let result = self.execute_atomic(&mut ctx, action);
        commands.append(&mut ctx.commands);

        // Catchup commands first, then the target action's, in the order they occurred.
        ctx.commands = commands;

        // Return the accumulated context (catchup + target) whether or not the
        // requested action succeeds — only the Ok/Err payload differs.
        match result {
            Ok(main_response) => Ok((main_response, ctx)),
            Err(err) => Err((err, ctx)),
        }
    }

    // every update to any place in code after the engine is publicly usable requires the version number to be incremented by 1
    /// return the latest version of the engine
    pub fn version() -> u64 {
        0
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}
