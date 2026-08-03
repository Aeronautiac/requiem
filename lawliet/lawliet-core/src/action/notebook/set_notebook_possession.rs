/*
* SYSTEM ACTION
* Transfer notebook possession: update actor caches and channel permissions atomically.
* Does not modify the notebook's ownership fields — callers handle that themselves.
*/

use lawliet_types::{
    channel::{AlivePolicy, PermUpdatePolicy},
    command::CommandRecipient,
};

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionInterface, ActionResponse, ActionResult,
        CreateAndGiveProfile, RemoveFromChannel,
    },
    actor::ActorDisplay,
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

        // Callers finalize the notebook's ownership fields before this action, so the borrow flag
        // reflects the post-transfer state.
        //
        // The holder is told "the book in your hands is not yours"; System mirrors it for the admin
        // overview, exactly as the fake status does. Nobody else — it is a fact about one person,
        // and no part of what the channel carried, so it never reaches the record.
        let borrowed = notebook.borrowed.is_some();
        if let Some(holder) = self.to {
            for recipient in [CommandRecipient::Actor(holder), CommandRecipient::System] {
                ctx.push_cmd(
                    Command::NotebookBorrowingStatus {
                        notebook_id: self.notebook_id,
                        borrowed,
                    },
                    recipient,
                    eng.time,
                );
            }
        }

        if let Some(from) = self.from {
            if mutate {
                if let Ok(a) = get_actor_mut(eng, from) {
                    a.remove_notebook(self.notebook_id);
                }
            }
            Action::RemoveFromChannel(RemoveFromChannel {
                channel_id,
                player_id: from,
            })
            .handle(eng, ctx, &ActionActor::System, version, mutate)?;
        }

        if let Some(to) = self.to {
            if mutate {
                get_actor_mut(eng, to)?.add_notebook(self.notebook_id);
            }
            // The book is yours to read and write while you are alive to do it. What is done with
            // it beyond that — passing it on, using it — is the notebook's own business and gated
            // elsewhere.
            Action::CreateAndGiveProfile(CreateAndGiveProfile {
                channel_id,
                player_id: to,
                display: ActorDisplay::Raw(to),
                visible: true,
                shared: false,
                transferrable: false,
                perm_policy: PermUpdatePolicy::Alive(AlivePolicy {}),
            })
            .handle(eng, ctx, &ActionActor::System, version, mutate)?;
        }

        Ok(ActionResponse::SetNotebookPossession(
            SetNotebookPossessionResponse {},
        ))
    }
}
