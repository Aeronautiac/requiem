/*
* SYSTEM ACTION
* Transfer notebook possession: update actor caches and channel permissions atomically.
* Does not modify the notebook's ownership fields — callers handle that themselves.
*/

use indexmap::indexset;

use crate::{
    action::{
        Action, ActionContext, ActionInterface, ActionResult, ActionActor, ActionResponse, SetMember,
    },
    actor::ActorDisplay,
    channel::{ChannelMember, ChannelPermission},
    helpers::{get_actor_mut, get_notebook},
};

pub use crate::action::{SetNotebookPossession, SetNotebookPossessionResponse};

impl ActionInterface for SetNotebookPossession {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> ActionResult {
        let channel_id = get_notebook(eng, self.notebook_id)?.channel_id;

        if let Some(from) = self.from {
            if mutate {
                if let Ok(a) = get_actor_mut(eng, from) {
                    a.remove_notebook(self.notebook_id);
                }
            }
            Action::SetMember(SetMember { player_id: from, channel_id, settings: None })
                .handle(eng, ctx, &ActionActor::System, version, mutate)?;
        }

        if let Some(to) = self.to {
            if mutate {
                get_actor_mut(eng, to)?.add_notebook(self.notebook_id);
            }
            Action::SetMember(SetMember {
                player_id: to,
                channel_id,
                settings: Some(ChannelMember {
                    perms: ChannelPermission::Send | ChannelPermission::View,
                    displays: indexset![ActorDisplay::Raw(to)],
                }),
            })
            .handle(eng, ctx, &ActionActor::System, version, mutate)?;
        }

        Ok(ActionResponse::SetNotebookPossession(SetNotebookPossessionResponse {}))
    }
}
