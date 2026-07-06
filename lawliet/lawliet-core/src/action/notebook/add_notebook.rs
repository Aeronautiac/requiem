/*
* SYSTEM ACTION
* Add a notebook to the world state
*/

use lawliet_types::command::CommandRecipient;

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionInterface, ActionResponse, ActionResult,
        CreateChannel,
    },
    command::Command,
    common::{NotebookKey, Version},
    engine::Engine,
};

pub use crate::action::{AddNotebook, AddNotebookResponse};

impl ActionInterface for AddNotebook {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        let channel_response = Action::CreateChannel(CreateChannel { loggable: false })
            .handle(eng, ctx, actor, version, mutate)?;
        let ActionResponse::CreateChannel(data) = channel_response else {
            unreachable!();
        };
        let channel_id = data.id;

        let id = if mutate {
            eng.world.add_notebook(channel_id, self.fake)
        } else {
            NotebookKey::default()
        };

        ctx.push_cmd(
            Command::MapNotebook {
                notebook_id: id,
                channel_id,
            },
            CommandRecipient::System,
            eng.time,
        );

        Ok(ActionResponse::AddNotebook(AddNotebookResponse { id }))
    }
}
