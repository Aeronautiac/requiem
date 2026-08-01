/*
* SYSTEM / ADMIN ACTION
* Change whether a notebook is fake — a fake book's writes cannot kill — and restate the status to
* its original owner (mirrored to System). The flag is otherwise fixed at creation; this exists so
* a host can turn a decoy into a real book or back, with the owner learning of the change.
*
* Emits nothing beyond the status itself, and only to the original owner, exactly as when the book
* was first handed to them. Whoever merely holds or inherited it still learns nothing here.
*/

use crate::{
    action::{ActionActor, ActionContext, ActionInterface, ActionResponse, ActionResult},
    common::Version,
    engine::Engine,
    helpers::{cmd_notebook_fake_status, get_notebook, get_notebook_mut},
};

pub use crate::action::{SetNotebookFake, SetNotebookFakeResponse};

impl ActionInterface for SetNotebookFake {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        _version: Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;
        get_notebook(eng, self.notebook_id)?;

        if mutate {
            get_notebook_mut(eng, self.notebook_id)?.fake = self.fake;
        }

        cmd_notebook_fake_status(eng, ctx, self.notebook_id);

        Ok(ActionResponse::SetNotebookFake(SetNotebookFakeResponse {}))
    }
}
