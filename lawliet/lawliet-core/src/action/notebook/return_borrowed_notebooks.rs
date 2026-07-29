use lawliet_types::{
    action::{
        Action, ActionActor, ReturnBorrowedNotebooks, ReturnBorrowedNotebooksResponse,
        SetNotebookPossession,
    },
    common::NotebookKey,
};
use smallvec::SmallVec;

use crate::{
    action::{ActionInterface, ActionResponse},
    helpers::get_notebook_mut,
};

impl ActionInterface for ReturnBorrowedNotebooks {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut lawliet_types::action::ActionContext,
        actor: &lawliet_types::action::ActionActor,
        version: lawliet_types::common::Version,
        mutate: bool,
    ) -> crate::action::ActionResult {
        actor.admin_or_system()?;

        let borrowed: SmallVec<[NotebookKey; 8]> = eng
            .world
            .notebooks
            .iter()
            .filter(|(_, book)| book.borrowed.is_some())
            .map(|(key, _)| key)
            .collect();

        if mutate {
            for key in borrowed {
                let book =
                    get_notebook_mut(eng, key).expect("notebook key was found in engine already");
                let old = book.owner;
                book.return_borrowed();
                let new = book.owner;
                Action::SetNotebookPossession(SetNotebookPossession {
                    notebook_id: key,
                    from: old,
                    to: new,
                })
                .handle(eng, ctx, &ActionActor::System, version, mutate)?;
            }
        }

        Ok(ActionResponse::ReturnBorrowedNotebooks(
            ReturnBorrowedNotebooksResponse {},
        ))
    }
}
