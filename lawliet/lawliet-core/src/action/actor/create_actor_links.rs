/*
* SYSTEM ACTION
* Create links between all actors
*/

use crate::{
    action::{
        ActionContext, ActionInterface, ActionResult, ActionActor, ActionResponse,
    },
    actor::{ActorLink, ActorLinkType, ActorType},
    common::ActorKey,
    helpers::{get_actor_mut, get_role_config},
};

struct LinkDescriptor {
    pub from_dest: ActorKey,
    pub to_dest: ActorKey,
    pub link_type: ActorLinkType,
}

pub use crate::action::{CreateActorLinks, CreateActorLinksResponse};

impl ActionInterface for CreateActorLinks {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        let mut links_to_create: Vec<LinkDescriptor> = vec![];
        for (id, _) in eng.world.actors.iter() {
            // player (role based) links
            let mut found_role = None;
            if let Some(player) = eng.world.get_player(id) {
                found_role = Some(player.role);
            }
            if let Some(role) = found_role {
                let role_config = get_role_config(eng, role)?;
                let role_links = role_config.actor_links.clone();
                for link in role_links {
                    for (id_other, other_actor) in eng.world.actors.iter() {
                        if id == id_other {
                            continue;
                        }
                        if let ActorType::Player(other_player) = &other_actor.actor_type
                            && other_player.role == link.role
                        {
                            links_to_create.push(LinkDescriptor {
                                from_dest: id,
                                to_dest: id_other,
                                link_type: link.link_type,
                            });
                        }
                    }
                }
            }
        }

        if mutate {
            for link in links_to_create {
                let target = get_actor_mut(eng, link.from_dest)?;
                target.add_link(ActorLink {
                    link_type: link.link_type,
                    link_dest: link.to_dest,
                });
            }
        }

        Ok(ActionResponse::CreateActorLinks(
            CreateActorLinksResponse {},
        ))
    }
}
