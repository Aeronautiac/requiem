// Peer into a contact channel's record by guessing which one.
//
// Contact channels are numbered in one strictly incrementing sequence, and that number is the only
// handle on one — you tap what you can work out from what you already know. Any contact channel is
// fair game, your own included.
//
// Usable by an org as well as a player (KK holds the full-history variant behind a vote), so the
// result goes wherever a view of that actor's things goes: to a player, or to an org's channel.
//
// Every outcome reports back, including the misses: learning that an id is unused is a real result,
// which is why a wrong guess is a Failure rather than an error. Failures draw a small pool, and that
// cap is the whole reason misses are rationed — without it the sequence could be walked from the top
// down until something answered, which would give away how many contacts exist.
//
// The tapped channel is told it was read, and never by whom. That gap is a move in itself: tapping a
// line you are on yourself makes the other side believe an outsider is listening.

use lawliet_types::{
    ability::{AbilityName, TapIn},
    command::{Command, TapInOutcome},
};

use crate::{
    ability::AbilityInterface,
    helpers::{
        actor_id, cmd_channel, get_ability, get_channel, get_gc, get_lounge, owner_view_recipient,
    },
    world::ContactChannel,
};

impl AbilityInterface for TapIn {
    fn ability_name(&self) -> AbilityName {
        AbilityName::TapIn
    }

    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &crate::action::ActionActor,
        ability: crate::AbilityKey,
        _version: u8,
        _mutate: bool,
    ) -> super::AbilityResult {
        let user_id = actor_id(actor).expect("expected valid actor to use TapIn");
        let recipient = owner_view_recipient(eng, user_id);

        // Variant 0 reads the whole record; variant 1 only what falls inside a window ending now.
        // The AbilityKey is what makes reading the variant possible at all.
        let range = match get_ability(eng, ability)?.variant {
            0 => None,
            _ => Some(eng.config.defaults.tap_in_window),
        };

        let channel_id = match eng.world.contact_channels.get(&self.contact_id) {
            Some(ContactChannel::Lounge(id)) => get_lounge(eng, *id)?.channel_id,
            Some(ContactChannel::Gc(id)) => get_gc(eng, *id)?.channel_id,
            None => {
                ctx.push_cmd(
                    Command::TapInResult {
                        contact_id: self.contact_id,
                        outcome: TapInOutcome::NoSuchContact,
                    },
                    recipient,
                    eng.time,
                );
                return Ok(super::AbilityStatus::Failure);
            }
        };

        // A dark channel is told apart from a missing one on purpose. A contact channel is loggable
        // unless an admin deliberately turned it off, so "nothing was ever written down here" is a
        // rare and meaningful answer rather than a way of hiding whether the guess was right.
        if !get_channel(eng, channel_id)?.loggable {
            ctx.push_cmd(
                Command::TapInResult {
                    contact_id: self.contact_id,
                    outcome: TapInOutcome::NotLoggable,
                },
                recipient,
                eng.time,
            );
            return Ok(super::AbilityStatus::Failure);
        }

        // The room learns first, so the record handed over already contains this tap. A later tap on
        // the same channel then shows that it is being watched.
        cmd_channel(
            eng,
            ctx,
            Command::ChannelTapped { channel_id },
            channel_id,
            true,
            Some(user_id),
        );

        let log = get_channel(eng, channel_id)?.log;
        ctx.push_cmd(
            Command::TapInResult {
                contact_id: self.contact_id,
                outcome: TapInOutcome::Found { log, range },
            },
            recipient,
            eng.time,
        );

        Ok(super::AbilityStatus::Success)
    }
}

#[cfg(test)]
mod tests {
    use lawliet_types::{
        ability::{AbilityBehaviour, AbilityName, TapIn},
        action::CreateAndGiveAbility,
        command::{Command, CommandRecipient, TapInOutcome},
    };

    use crate::{
        action::{Action, ActionActor, ActionContext, ActionRequest, CreateLounge, SetLoggable},
        common::{ActorKey, ID},
        config::role::Role,
        engine::Engine,
        helpers::{get_channel, get_lounge},
        lounge::LoungeVariant,
        test_helpers::{add_player, init_engine, quick_ability, send_message, use_ability},
    };

    // A tapper holding the ability at `variant`, two other players, and one Basic lounge between
    // them. The lounge is the thing being guessed at; the tapper is not in it.
    fn world(eng: &mut Engine, variant: u8) -> (ActorKey, crate::AbilityKey, ID) {
        init_engine(eng);
        let tapper = add_player(eng, 0, Role::Civilian, "tapper");
        let a = add_player(eng, 0, Role::Civilian, "alice");
        let b = add_player(eng, 0, Role::Civilian, "bob");

        eng.execute(ActionRequest {
            actor: ActionActor::System,
            timestamp: 0,
            payload: Action::CreateLounge(CreateLounge {
                variant: LoungeVariant::Basic {
                    contactor_id: a,
                    contacted_id: b,
                },
            }),
        })
        .unwrap();

        let ability = quick_ability(
            eng,
            0,
            CreateAndGiveAbility {
                ability_name: AbilityName::TapIn,
                variant,
                actor_id: tapper,
                volatile: false,
                transferrable: false,
            },
        );
        // One lounge exists, so it holds the only registered contact id.
        let contact_id = *eng.world.contact_channels.keys().next().unwrap();
        (tapper, ability, contact_id)
    }

    // The channel behind a contact id, which every test needs and no test is about.
    fn channel_of(eng: &Engine, contact_id: ID) -> crate::common::ChannelKey {
        match eng.world.contact_channels.get(&contact_id).unwrap() {
            crate::world::ContactChannel::Lounge(id) => get_lounge(eng, *id).unwrap().channel_id,
            crate::world::ContactChannel::Gc(id) => {
                crate::helpers::get_gc(eng, *id).unwrap().channel_id
            }
        }
    }

    fn outcome(ctx: &ActionContext) -> TapInOutcome {
        ctx.commands
            .iter()
            .find_map(|p| match &p.cmd {
                Command::TapInResult { outcome, .. } => Some(*outcome),
                _ => None,
            })
            .expect("a tap-in always reports back")
    }

    fn tap(
        eng: &mut Engine,
        time: crate::Time,
        tapper: ActorKey,
        ability: crate::AbilityKey,
        contact_id: ID,
    ) -> ActionContext {
        use_ability(
            eng,
            time,
            tapper,
            ability,
            AbilityBehaviour::TapIn(TapIn { contact_id }),
        )
        .unwrap()
        .1
    }

    #[test]
    fn the_full_variant_reads_the_whole_record() {
        let mut eng = Engine::new();
        let (tapper, ability, contact_id) = world(&mut eng, 0);

        let ctx = tap(&mut eng, 1, tapper, ability, contact_id);

        let log = get_channel(&eng, channel_of(&eng, contact_id)).unwrap().log;
        assert_eq!(outcome(&ctx), TapInOutcome::Found { log, range: None });
    }

    // The nerfed variant differs from the full one in exactly one way, and this is it.
    #[test]
    fn the_nerfed_variant_reads_a_window() {
        let mut eng = Engine::new();
        let (tapper, ability, contact_id) = world(&mut eng, 1);

        let ctx = tap(&mut eng, 1, tapper, ability, contact_id);

        let window = eng.config.defaults.tap_in_window;
        assert!(matches!(
            outcome(&ctx),
            TapInOutcome::Found { range: Some(r), .. } if r == window
        ));
    }

    // A miss is a result, not an error: the action succeeds and the guess is answered. Only a
    // reported miss can draw the failure pool that rations guessing.
    #[test]
    fn an_unused_id_reports_back_rather_than_erroring() {
        let mut eng = Engine::new();
        let (tapper, ability, contact_id) = world(&mut eng, 0);

        let ctx = tap(&mut eng, 1, tapper, ability, contact_id + 100);

        assert_eq!(outcome(&ctx), TapInOutcome::NoSuchContact);
    }

    // Told apart from a missing id on purpose — a contact channel is loggable unless an admin
    // turned it off, so players are entitled to know which of the two they hit.
    #[test]
    fn a_dark_channel_reads_differently_from_a_missing_one() {
        let mut eng = Engine::new();
        let (tapper, ability, contact_id) = world(&mut eng, 0);
        let channel_id = channel_of(&eng, contact_id);

        eng.execute(ActionRequest {
            actor: ActionActor::System,
            timestamp: 1,
            payload: Action::SetLoggable(SetLoggable {
                channel_id,
                loggable: false,
            }),
        })
        .unwrap();

        let ctx = tap(&mut eng, 2, tapper, ability, contact_id);

        assert_eq!(outcome(&ctx), TapInOutcome::NotLoggable);
    }

    // The gap between "you were read" and "by whom" is the whole play.
    #[test]
    fn the_tapped_channel_is_told_without_being_told_who() {
        let mut eng = Engine::new();
        let (tapper, ability, contact_id) = world(&mut eng, 0);
        let channel_id = channel_of(&eng, contact_id);
        let viewport = get_channel(&eng, channel_id).unwrap().viewport;

        let ctx = tap(&mut eng, 1, tapper, ability, contact_id);

        assert!(ctx.commands.iter().any(|p| {
            p.recipient == CommandRecipient::Viewport(viewport)
                && matches!(&p.cmd, Command::ChannelTapped { .. })
        }));
        // Nothing addressed to the room names the tapper.
        assert!(!ctx.commands.iter().any(|p| {
            p.recipient == CommandRecipient::Viewport(viewport)
                && matches!(&p.cmd, Command::TapInResult { .. })
        }));
    }

    // A tap is itself witnessed, so it lands on the record a later tap reads back.
    #[test]
    fn a_tap_is_on_the_record() {
        let mut eng = Engine::new();
        let (tapper, ability, contact_id) = world(&mut eng, 0);
        let channel_id = channel_of(&eng, contact_id);
        let log = get_channel(&eng, channel_id).unwrap().log;

        let ctx = tap(&mut eng, 1, tapper, ability, contact_id);

        assert!(ctx.commands.iter().any(|p| {
            p.recipient == CommandRecipient::Log(log)
                && matches!(&p.cmd, Command::ChannelTapped { .. })
        }));
    }

    // What a tap actually buys: the messages in the room reached the record, so there is something
    // for the reveal to point at.
    #[test]
    fn messages_in_a_tapped_lounge_are_on_its_record() {
        let mut eng = Engine::new();
        let (_, _, contact_id) = world(&mut eng, 0);
        let channel_id = channel_of(&eng, contact_id);
        let channel = get_channel(&eng, channel_id).unwrap();
        let log = channel.log;
        let speaker = *channel.members.keys().next().unwrap();
        let profile = channel.accessible_profiles(speaker)[0].profile_id;

        let (_, ctx) = send_message(
            &mut eng,
            1,
            speaker,
            channel_id,
            profile,
            "something worth tapping for",
        )
        .unwrap();

        assert!(ctx.commands.iter().any(|p| {
            p.recipient == CommandRecipient::Log(log)
                && matches!(&p.cmd, Command::AddMessage { .. })
        }));
    }
}
