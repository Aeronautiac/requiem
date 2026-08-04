/*
* SYSTEM ACTION
* Name the news anchor, or vacate the post with None.
*
* The anchor's kit lives on the world (see News). Naming an anchor hands that kit's ownership to the
* target — abilities and passives reassigned in place via Give*, so their charges carry across a
* change of anchor. Vacating takes the kit back to ownerless via Take*. Either way a NewsAnchor
* command states the new holder to the world.
*/

use lawliet_types::{
    action::{ActionError, ActionResponse, SetNewsAnchor, SetNewsAnchorResponse},
    command::{Command, CommandRecipient},
};

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionInterface, ActionResult, GiveAbility, GivePassive,
        TakeAbility, TakePassive,
    },
    helpers::get_player,
};

impl ActionInterface for SetNewsAnchor {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        if let Some(target_id) = self.target_id {
            get_player(eng, target_id)?;
        }

        // Covers both "already the anchor" and "already vacant" in one comparison.
        if eng.world.news.anchor == self.target_id {
            return Err(ActionError::AlreadyNewsAnchor);
        }

        let abilities: Vec<_> = eng.world.news.anchor_abilities.iter().copied().collect();
        let passives: Vec<_> = eng.world.news.anchor_passives.iter().copied().collect();

        match self.target_id {
            // Hand the whole kit to the new anchor. Give* removes each item from its previous holder
            // (the outgoing anchor, if any) as part of the transfer.
            Some(target_id) => {
                for ability_id in abilities {
                    Action::GiveAbility(GiveAbility {
                        ability_id,
                        actor_id: target_id,
                        volatile: false,
                    })
                    .handle(eng, ctx, actor, version, mutate)?;
                }
                for passive_id in passives {
                    Action::GivePassive(GivePassive {
                        passive_id,
                        actor_id: target_id,
                        volatile: false,
                    })
                    .handle(eng, ctx, actor, version, mutate)?;
                }
            }
            // Strip the kit back to ownerless.
            None => {
                for ability_id in abilities {
                    Action::TakeAbility(TakeAbility { ability_id })
                        .handle(eng, ctx, actor, version, mutate)?;
                }
                for passive_id in passives {
                    Action::TakePassive(TakePassive { passive_id })
                        .handle(eng, ctx, actor, version, mutate)?;
                }
            }
        }

        if mutate {
            eng.world.news.anchor = self.target_id;
        }

        ctx.push_cmd(
            Command::NewsAnchor {
                target_id: self.target_id,
            },
            CommandRecipient::Viewport(eng.world.events_viewport),
            eng.time,
        );

        Ok(ActionResponse::SetNewsAnchor(SetNewsAnchorResponse {}))
    }
}
