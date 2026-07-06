/*
* SYSTEM ACTION
* Create a lounge and return the lounge id and channel id.
* Add the lounge to all involved player caches and add players to the channels.
* Creating a lounge will update a player's lounges immediately after (their channel permissions will
* be modified based on current state).
* Channel permissions are set to none in the creation stage. They are only applied after a lounge update.
*/

use indexmap::{IndexSet, indexset};
use lawliet_types::{command::CommandRecipient, lounge::AnonymousLoungeRoleDisplay};
use smallvec::{SmallVec, smallvec};

use crate::{
    action::{
        Action, ActionInterface, ActionResponse, CreateChannel, SetMember, UpdateContactChannels,
    },
    actor::ActorDisplay,
    channel::{ChannelMember, ChannelPermissions},
    command::Command,
    common::{ActorKey, LoungeKey},
    helpers::{get_player, get_player_mut},
    lounge::{Lounge, LoungeVariant},
    world::ContactChannel,
};

struct Participant {
    pub displays: IndexSet<ActorDisplay>,
    pub id: ActorKey,
}

use crate::action::ActionActor;
pub use crate::action::{CreateLounge, CreateLoungeResponse};

impl ActionInterface for CreateLounge {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> crate::action::ActionResult {
        actor.admin_or_system()?;

        let mut participants: SmallVec<[Participant; 8]> = smallvec![];
        match &self.variant {
            LoungeVariant::Fake {
                creator_id,
                contacted_id,
                contactor_id,
            } => {
                participants.push(Participant {
                    id: *creator_id,
                    displays: indexset![
                        ActorDisplay::Raw(*contacted_id),
                        ActorDisplay::Raw(*contactor_id),
                    ],
                });
            }
            LoungeVariant::Basic {
                contacted_id,
                contactor_id,
            } => {
                participants.push(Participant {
                    id: *contactor_id,
                    displays: indexset![ActorDisplay::Raw(*contactor_id),],
                });
                participants.push(Participant {
                    id: *contacted_id,
                    displays: indexset![ActorDisplay::Raw(*contacted_id),],
                });
            }
            LoungeVariant::Anonymous {
                contacted_id,
                contactor_id,
                role_display,
            } => {
                match role_display {
                    AnonymousLoungeRoleDisplay::Dynamic => {
                        // TODO:
                        // need to add a new mechanism for updating the role displayed here
                        // for now, unimplemented!(). this will rarely happen (only when someone's
                        // role changes AFTER they have already anonymous contacted someone). so it
                        // is probably fine to leave the behaviour alone for now. abilities should
                        // use static roles.
                        unimplemented!()
                    }
                    AnonymousLoungeRoleDisplay::Static(role) => {
                        participants.push(Participant {
                            id: *contactor_id,
                            displays: indexset![ActorDisplay::Role(*role),],
                        });
                    }
                }
                participants.push(Participant {
                    id: *contacted_id,
                    displays: indexset![ActorDisplay::Raw(*contacted_id),],
                });
            }
        };
        for participant in &participants {
            get_player(eng, participant.id)?;
        }

        let channel_response = Action::CreateChannel(CreateChannel { loggable: true })
            .handle(eng, ctx, actor, version, mutate)?;
        let ActionResponse::CreateChannel(data) = channel_response else {
            unreachable!();
        };
        let channel_id = data.id;

        let lounge_id = if mutate {
            let lounge = Lounge {
                channel_id,
                variant: self.variant.clone(),
            };

            let lounge_id: LoungeKey = eng.world.add_lounge(lounge);
            eng.world
                .register_contact_channel(ContactChannel::Lounge(lounge_id));

            for participant in participants {
                Action::SetMember(SetMember {
                    channel_id,
                    player_id: participant.id,
                    settings: Some(ChannelMember {
                        perms: ChannelPermissions::EMPTY,
                        displays: participant.displays,
                    }),
                })
                .handle(eng, ctx, actor, version, mutate)?;

                let player_data = get_player_mut(eng, participant.id)
                    .expect("expected lounge participant to be a valid player");
                player_data.add_lounge(lounge_id);

                Action::UpdateContactChannels(UpdateContactChannels {
                    player_id: participant.id,
                })
                .handle(eng, ctx, actor, version, mutate)?;

                ctx.push_cmd(
                    Command::MapLounge {
                        lounge_id,
                        channel_id,
                    },
                    CommandRecipient::System,
                    eng.time,
                );
            }

            lounge_id
        } else {
            LoungeKey::default()
        };

        Ok(ActionResponse::CreateLounge(CreateLoungeResponse {
            lounge_id,
            channel_id,
        }))
    }
}
