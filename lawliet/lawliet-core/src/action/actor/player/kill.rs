/*
* SYSTEM ACTION
* Kill a player and handle side effects
*/

use smallvec::{SmallVec, smallvec};

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionInterface, ActionResponse, ActionResult,
        AddState, GiveAbility, GiveNotebook, GivePassive, SetBooksDormant, SetBorrowersToOwners,
        SeverLinks, TakeNotebook,
    },
    actor::{ActorLinkType, ActorType, modifier::Modifier, state::State},
    command::Command,
    common::Version,
    engine::Engine,
    helpers::{
        cmd_all_deferred, get_ability, get_actor, get_actor_mut, get_notebook, get_passive,
        require_alive,
    },
};

pub use crate::action::{Kill, KillResponse};

impl ActionInterface for Kill {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;
        require_alive(eng, self.target_id)?;

        let target = get_actor(eng, self.target_id)?;
        let ActorType::Player(target_data) = &target.actor_type else {
            unreachable!()
        };
        let true_name = target_data.true_name.clone();
        let role = target_data.role;

        Action::AddState(AddState {
            actor_id: self.target_id,
            state: State::Dead,
        })
        .handle(eng, ctx, actor, version, mutate)?;

        let mut notebook_transferred = false;
        let mut ability_transferred = false;
        let mut next_actions: SmallVec<[Action; 8]> = smallvec![Action::AddState(AddState {
            actor_id: self.target_id,
            state: State::Dead
        })];
        if let Some(killer_id) = self.killer_id {
            let killer = get_actor_mut(eng, killer_id)?;

            if mutate {
                killer.kills.push(self.target_id);
            }

            let target = get_actor(eng, self.target_id)?;

            if killer_id != self.target_id {
                // ability transfers
                for id in target.abilities.iter() {
                    let ability = get_ability(eng, *id)?;
                    if ability.ownership_struct.transferrable {
                        ability_transferred = true;
                        next_actions.push(Action::GiveAbility(GiveAbility {
                            volatile: false,
                            ability_id: *id,
                            actor_id: killer_id,
                        }));
                    }
                }

                // passive transfers
                for id in target.passives.iter() {
                    let passive = get_passive(eng, *id)?;
                    if passive.ownership_struct.transferrable {
                        ability_transferred = true;
                        next_actions.push(Action::GivePassive(GivePassive {
                            volatile: false,
                            passive_id: *id,
                            actor_id: killer_id,
                        }));
                    }
                }
            }
        }

        // A notebook changes ownership if the true owner is not the killer
        // the notebook should still be given back if its not being held by the true owner.
        // A notebook is transferred if the person holding the notebook changes
        // If a person who is borrowing a notebook dies without a killer, the notebook loses its owner
        let target = get_actor(eng, self.target_id)?;
        for id in target.notebooks.iter() {
            let notebook = get_notebook(eng, *id)?;
            // it should be impossible
            // for a notebook to have a current owner and no true owner
            let true_owner = notebook.get_true_owner().unwrap();
            if let Some(killer_id) = self.killer_id {
                if (true_owner != killer_id) || (self.target_id != true_owner) {
                    next_actions.push(Action::GiveNotebook(GiveNotebook {
                        notebook_id: *id,
                        actor_id: killer_id,
                        volatile: false,
                    }));

                    if self.target_id != killer_id {
                        notebook_transferred = true;
                    }
                }
            } else if notebook.get_true_owner().unwrap() != self.target_id {
                next_actions.push(Action::TakeNotebook(TakeNotebook { notebook_id: *id }));
            };
        }

        for mut action in next_actions {
            action.handle(eng, ctx, actor, version, mutate)?;
        }

        if self.allow_link_chaining {
            let target = get_actor(eng, self.target_id)?;

            // life links
            let links = target.actor_links.clone();
            for link in links {
                if link.link_type != ActorLinkType::Life {
                    continue;
                }
                let linked_actor = get_actor(eng, link.link_dest).unwrap();
                if !linked_actor.states.contains(State::Dead) {
                    Action::Kill(Kill {
                        target_id: link.link_dest,
                        silent: self.silent,
                        death_message: Some(eng.config.defaults.life_link_death_message.clone()),
                        allow_link_chaining: true,
                        sever_links: false,
                        set_books_dormant: false,
                        killer_id: self.killer_id,
                    })
                    .handle(eng, ctx, actor, version, mutate)?;
                }
            }
        }

        if self.sever_links {
            Action::SeverLinks(SeverLinks {
                actor_id: self.target_id,
            })
            .handle(eng, ctx, actor, version, mutate)?;
        }

        if self.set_books_dormant {
            Action::SetBooksDormant(SetBooksDormant {
                actor_id: self.target_id,
            })
            .handle(eng, ctx, actor, version, mutate)?;
        }

        Action::SetBorrowersToOwners(SetBorrowersToOwners {
            actor_id: self.target_id,
        })
        .handle(eng, ctx, actor, version, mutate)?;

        if !self.silent {
            cmd_all_deferred(
                eng,
                ctx,
                Command::Death {
                    target_id: self.target_id,
                    true_name: String::from(&*true_name),
                    death_message: if let Some(msg) = &self.death_message {
                        msg.clone()
                    } else {
                        eng.config.defaults.death_message.clone()
                    },
                    role,
                    notebook_transferred,
                    ability_transferred,
                },
                Modifier::NoPresence.into(),
                true,
            );
        }

        Ok(ActionResponse::Kill(KillResponse {}))
    }
}
