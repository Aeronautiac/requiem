/*
* SYSTEM ACTION
* Transfer notebook possession: update actor caches and channel permissions atomically.
* Does not modify the notebook's ownership fields — callers handle that themselves.
*/

use indexmap::indexset;
use lawliet_types::command::CommandRecipient;

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionInterface, ActionResponse, ActionResult,
        SetMember,
    },
    actor::ActorDisplay,
    channel::{ChannelMember, ChannelPermission},
    command::Command,
    helpers::{get_actor_mut, get_notebook},
};

pub use crate::action::{SetNotebookPossession, SetNotebookPossessionResponse};

impl ActionInterface for SetNotebookPossession {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut ActionContext,
        _actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> ActionResult {
        let notebook = get_notebook(eng, self.notebook_id)?;
        let channel_id = notebook.channel_id;

        // Callers finalize the notebook's ownership fields before this action, so the borrow
        // flag reflects the post-transfer state. Broadcast it so the notebook channel can show
        // whether the book is currently on loan.
        ctx.push_cmd(
            Command::NotebookBorrowingStatus {
                notebook_id: self.notebook_id,
                borrowed: notebook.borrowed.is_some(),
            },
            CommandRecipient::System,
            eng.time,
        );

        if let Some(from) = self.from {
            if mutate {
                if let Ok(a) = get_actor_mut(eng, from) {
                    a.remove_notebook(self.notebook_id);
                }
            }
            Action::SetMember(SetMember {
                player_id: from,
                channel_id,
                settings: None,
            })
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

        Ok(ActionResponse::SetNotebookPossession(
            SetNotebookPossessionResponse {},
        ))
    }
}
