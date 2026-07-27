/*
* SYSTEM ACTION
* Create a lounge and return the lounge id and channel id.
* Add the lounge to all involved player caches and add players to the channels.
* Creating a lounge will update a player's lounges immediately after (their channel permissions will
* be modified based on current state).
* Channel permissions are set to none in the creation stage. They are only applied after a lounge update.
*/

use indexmap::{IndexSet, indexset};
use lawliet_types::lounge::AnonymousLoungeRoleDisplay;
use smallvec::{SmallVec, smallvec};

use crate::{
    action::{
        Action, ActionInterface, ActionResponse, CreateChannel, SetMember, UpdateContactChannels,
    },
    actor::ActorDisplay,
    channel::{ChannelMember, ChannelPermissions},
    command::Command,
    common::{ActorKey, LoungeKey},
    helpers::{cmd_channel, cmd_contact_log, get_player, get_player_mut},
    lounge::{Lounge, LoungeVariant},
    passive::{ContactEvent, ContactLog},
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
        // How the contact reads to a log watcher, and who actually made it. These come apart for a
        // Fake lounge: the displays name a pair that never spoke, while the creator is the only one
        // who did anything.
        let contact_displays: (ActorDisplay, ActorDisplay);
        let initiator: ActorKey;

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
                contact_displays = (
                    ActorDisplay::Raw(*contactor_id),
                    ActorDisplay::Raw(*contacted_id),
                );
                initiator = *creator_id;
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
                contact_displays = (
                    ActorDisplay::Raw(*contactor_id),
                    ActorDisplay::Raw(*contacted_id),
                );
                initiator = *contactor_id;
            }
            LoungeVariant::Anonymous {
                contacted_id,
                contactor_id,
                role_display,
            } => {
                // Both variants resolve to a role display; they differ only in where the role comes
                // from. Dynamic reads the contactor's role as it stands now.
                let role = match role_display {
                    // TODO:
                    // there is no mechanism for UPDATING the display once set, so a role change
                    // after the lounge exists leaves this showing the old one. Rare enough to leave
                    // alone — abilities should prefer static roles.
                    AnonymousLoungeRoleDisplay::Dynamic => get_player(eng, *contactor_id)?.role,
                    AnonymousLoungeRoleDisplay::Static(role) => *role,
                };
                participants.push(Participant {
                    id: *contactor_id,
                    displays: indexset![ActorDisplay::Role(role)],
                });
                participants.push(Participant {
                    id: *contacted_id,
                    displays: indexset![ActorDisplay::Raw(*contacted_id),],
                });
                contact_displays = (
                    ActorDisplay::Role(role),
                    ActorDisplay::Raw(*contacted_id),
                );
                initiator = *contactor_id;
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
            let contact_id = eng
                .world
                .register_contact_channel(ContactChannel::Lounge(lounge_id));

            cmd_contact_log(
                eng,
                ctx,
                Some(initiator),
                ContactLog {
                    contact_id,
                    contactor: contact_displays.0,
                    contacted: contact_displays.1,
                    event: ContactEvent::LoungeOpened,
                },
            );

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

                cmd_channel(
                    eng,
                    ctx,
                    Command::MapLounge {
                        lounge_id,
                        channel_id,
                        contact_id,
                    },
                    channel_id,
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
