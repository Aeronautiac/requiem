pub mod add_notebook;
pub mod create_and_give_notebook;
pub mod destroy_notebook;
pub mod give_notebook;
pub mod lend_notebook;
pub mod notebook_scheduled_kill;
pub mod return_borrowed_notebooks;
pub mod return_dormant_books;
pub mod set_books_dormant;
pub mod set_borrowers_to_owners;
pub mod set_notebook_fake;
pub mod set_notebook_possession;
pub mod take_notebook;
pub mod write_name;

#[cfg(test)]
mod notebook_tests {
    use lawliet_types::command::{Command, CommandRecipient};

    use crate::{
        action::{
            Action, ActionActor, ActionRequest, CreateAndGiveNotebook, SetNotebookFake, WriteName,
        },
        actor::state::State,
        config::role::Role,
        helpers::{get_actor, get_notebook},
        test_helpers::*,
    };

    // Every fake-status statement in a context, as (recipient, fake) pairs.
    fn fake_status(ctx: &crate::action::ActionContext) -> Vec<(CommandRecipient, bool)> {
        ctx.commands
            .iter()
            .filter_map(|p| match &p.cmd {
                Command::NotebookFakeStatus { fake, .. } => Some((p.recipient.clone(), *fake)),
                _ => None,
            })
            .collect()
    }

    // a fake notebook should not kill someone
    #[test]
    fn fake_write_delayed() {
        let mut eng = started_engine();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "Light Yagami");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "Quillsh Wammy");
        let notebook_id = quick_notebook(&mut eng, 0, p1, true);

        quick_write(&mut eng, p1, 0, notebook_id, "quillsh wammy", 40).unwrap();
        null_action(&mut eng, 39);

        let p1_actor = get_actor(&eng, p1).unwrap();
        let p2_actor = get_actor(&eng, p2).unwrap();
        assert!(!p1_actor.has_state(State::Dead));
        assert!(!p2_actor.has_state(State::Dead));

        null_action(&mut eng, 40);

        let p1_actor = get_actor(&eng, p1).unwrap();
        let p2_actor = get_actor(&eng, p2).unwrap();
        assert!(!p1_actor.has_state(State::Dead));
        assert!(!p2_actor.has_state(State::Dead));
    }

    #[test]
    fn fake_write_instant() {
        let mut eng = started_engine();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "Light Yagami");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "Quillsh Wammy");
        let notebook_id = quick_notebook(&mut eng, 0, p1, true);

        quick_write(&mut eng, p1, 0, notebook_id, "quillsh wammy", 0).unwrap();

        let p1_actor = get_actor(&eng, p1).unwrap();
        let p2_actor = get_actor(&eng, p2).unwrap();
        assert!(!p1_actor.has_state(State::Dead));
        assert!(!p2_actor.has_state(State::Dead));
    }

    #[test]
    fn write_delayed() {
        let mut eng = started_engine();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "Light Yagami");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "Quillsh Wammy");
        let notebook_id = quick_notebook(&mut eng, 0, p1, false);

        quick_write(&mut eng, p1, 0, notebook_id, "quillsh wammy", 40).unwrap();
        null_action(&mut eng, 39);

        let p1_actor = get_actor(&eng, p1).unwrap();
        let p2_actor = get_actor(&eng, p2).unwrap();
        assert!(!p1_actor.has_state(State::Dead));
        assert!(!p2_actor.has_state(State::Dead));

        null_action(&mut eng, 40);

        let p1_actor = get_actor(&eng, p1).unwrap();
        let p2_actor = get_actor(&eng, p2).unwrap();
        assert!(!p1_actor.has_state(State::Dead));
        assert!(p2_actor.has_state(State::Dead));
    }

    #[test]
    fn write_instant() {
        let mut eng = started_engine();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "Light Yagami");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "Quillsh Wammy");
        let notebook_id = quick_notebook(&mut eng, 0, p1, false);

        quick_write(&mut eng, p1, 0, notebook_id, "quillSh wammy", 0).unwrap();

        let p1_actor = get_actor(&eng, p1).unwrap();
        let p2_actor = get_actor(&eng, p2).unwrap();
        assert!(!p1_actor.has_state(State::Dead));
        assert!(p2_actor.has_state(State::Dead));
    }

    // if you kill someone who is holding a notebook, you should get that notebook
    #[test]
    fn kill_wielder() {
        let mut eng = started_engine();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");
        let p1_notebook_id = quick_notebook(&mut eng, 0, p1, false);
        let p2_notebook_id = quick_notebook(&mut eng, 0, p2, false);

        quick_write(&mut eng, p1, 0, p1_notebook_id, "p2", 0).unwrap();

        let p1_actor = get_actor(&eng, p1).unwrap();
        let p2_actor = get_actor(&eng, p2).unwrap();
        assert!(p1_actor.has_notebook(p2_notebook_id));
        assert!(!p2_actor.has_notebook(p2_notebook_id));
    }

    // what happens if you kill yourself while you are the true owner of a notebook?
    // - you should remain as the true owner, but the notebook should be unusable because you're dead
    // - the game should not announce a notebook transfer
    #[test]
    fn suicide() {
        let mut eng = started_engine();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "Light Yagami");
        let notebook_id = quick_notebook(&mut eng, 0, p1, false);

        quick_write(&mut eng, p1, 0, notebook_id, "light yagami", 121).unwrap();
        null_action(&mut eng, 122);

        let p1_actor = get_actor(&eng, p1).unwrap();
        assert!(p1_actor.has_notebook(notebook_id));
        assert!(p1_actor.has_state(State::Dead));
    }

    #[test]
    fn lend() {
        let mut eng = started_engine();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");
        let p1_notebook_id_1 = quick_notebook(&mut eng, 0, p1, false);

        quick_lend(&mut eng, 0, p1_notebook_id_1, p1, p2);

        let p1_actor = get_actor(&eng, p1).unwrap();
        let p2_actor = get_actor(&eng, p2).unwrap();
        assert!(!p1_actor.has_notebook(p1_notebook_id_1));
        assert!(p2_actor.has_notebook(p1_notebook_id_1));
    }

    // General rules:
    // - If you kill a notebook wielder, and you are not the true owner of that notebook, then the
    // notebook should be given to you. It doesn't matter if you killed yourself or not.
    // - Notebook transfers are only announced if a death resulted in the CURRENT owner of a death
    // note changing, not the true owner.

    // what happens if you kill someone you're lending to?
    // - should get back early
    #[test]
    fn kill_lent_to() {
        let mut eng = started_engine();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");
        let p1_notebook_id_1 = quick_notebook(&mut eng, 0, p1, false);
        let p1_notebook_id_2 = quick_notebook(&mut eng, 0, p1, false);

        quick_lend(&mut eng, 0, p1_notebook_id_2, p1, p2);
        quick_write(&mut eng, p1, 0, p1_notebook_id_1, "p2", 0).unwrap();

        let p1_actor = get_actor(&eng, p1).unwrap();
        let p2_actor = get_actor(&eng, p2).unwrap();
        assert!(p1_actor.has_notebook(p1_notebook_id_2));
        assert!(!p2_actor.has_notebook(p1_notebook_id_2));
    }

    // what happens if you kill yourself while being lended to?
    // - the notebook should become yours but should become unusable because you are dead
    // - do not announce notebook transfer
    #[test]
    fn borrowed_suicide() {
        let mut eng = started_engine();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");
        let notebook_id = quick_notebook(&mut eng, 0, p1, false);

        quick_lend(&mut eng, 0, notebook_id, p1, p2);
        quick_write(&mut eng, p2, 0, notebook_id, "p2", 0).unwrap();

        let p1_actor = get_actor(&eng, p1).unwrap();
        let p2_actor = get_actor(&eng, p2).unwrap();
        let notebook = get_notebook(&eng, notebook_id).unwrap();
        assert!(!p1_actor.has_notebook(notebook_id));
        assert!(p2_actor.has_notebook(notebook_id));
        assert!(notebook.get_true_owner().unwrap() == p2);
    }

    // what happens if you kill someone who is lending to you?
    // what happens if the owner dies while the notebook is being lent out to someone?
    // - the person who is currently holding the notebook becomes the true owner
    // - do not announce a transfer
    #[test]
    fn borrowed_true_owner_died() {
        let mut eng = started_engine();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");
        let notebook_id = quick_notebook(&mut eng, 0, p1, false);

        quick_lend(&mut eng, 0, notebook_id, p1, p2);
        quick_kill(&mut eng, 0, true, true, false, p1);

        let p1_actor = get_actor(&eng, p1).unwrap();
        let p2_actor = get_actor(&eng, p2).unwrap();
        let notebook = get_notebook(&eng, notebook_id).unwrap();
        assert!(!p1_actor.has_notebook(notebook_id));
        assert!(p2_actor.has_notebook(notebook_id));
        assert!(notebook.get_true_owner().unwrap() == p2);
    }

    // what happens if the person borrowing your book dies before it returns and isnt killed by anyone?
    // - the notebook is lost (it no longer has an owner)
    // - do not announce a transfer
    #[test]
    fn borrowed_die_no_killer() {
        let mut eng = started_engine();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");
        let notebook_id = quick_notebook(&mut eng, 0, p1, false);

        quick_lend(&mut eng, 0, notebook_id, p1, p2);
        quick_kill(&mut eng, 0, true, true, false, p2);

        let p1_actor = get_actor(&eng, p1).unwrap();
        let p2_actor = get_actor(&eng, p2).unwrap();
        let notebook = get_notebook(&eng, notebook_id).unwrap();
        assert!(!p1_actor.has_notebook(notebook_id));
        assert!(!p2_actor.has_notebook(notebook_id));
        assert!(notebook.get_true_owner().is_none());
    }

    // it is possible to die before your scheduled notebook death through things like being executed
    // - the scheduled death should fail with no side effects
    #[test]
    fn die_before_scheduled() {
        let mut eng = started_engine();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let notebook_id = quick_notebook(&mut eng, 0, p1, false);

        quick_write(&mut eng, p1, 0, notebook_id, "p1", 10).unwrap();
        quick_kill(&mut eng, 1, true, true, false, p1);
        null_action(&mut eng, 11);
    }

    // what happens if a dead player kills a living player who owns a notebook through a scheduled
    // kill?
    // - the notebook goes to the dead player, but the dead player cannot use the notebook due to
    // restrictions
    #[test]
    fn dead_kill_living() {
        let mut eng = started_engine();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");
        let p1_notebook_id = quick_notebook(&mut eng, 0, p1, false);
        let p2_notebook_id = quick_notebook(&mut eng, 0, p2, false);

        quick_write(&mut eng, p1, 0, p1_notebook_id, "p2", 40).unwrap();
        quick_write(&mut eng, p2, 0, p2_notebook_id, "p1", 0).unwrap();
        null_action(&mut eng, 50);

        let p1_actor = get_actor(&eng, p1).unwrap();
        let p2_actor = get_actor(&eng, p2).unwrap();
        assert!(p1_actor.has_notebook(p1_notebook_id));
        assert!(p1_actor.has_notebook(p2_notebook_id));
        assert!(!p2_actor.has_notebook(p1_notebook_id));
        assert!(!p2_actor.has_notebook(p2_notebook_id));
    }

    // what happens when someone writes a name that has already been scheduled in a notebook?
    // - the actions cancel each other out (scheduled death is removed, actor does not die)
    #[test]
    fn collisions() {
        let mut eng = started_engine();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let book_1_id = quick_notebook(&mut eng, 0, p1, false);
        let book_2_id = quick_notebook(&mut eng, 0, p1, false);
        let book_3_id = quick_notebook(&mut eng, 0, p1, false);

        quick_write(&mut eng, p1, 0, book_1_id, "p1", 99).unwrap();
        quick_write(&mut eng, p1, 0, book_2_id, "p1", 0).unwrap();

        let p1_actor = get_actor(&eng, p1).unwrap();
        assert!(!p1_actor.has_state(State::Dead));

        quick_write(&mut eng, p1, 0, book_3_id, "p1", 0).unwrap();

        let p1_actor = get_actor(&eng, p1).unwrap();
        assert!(p1_actor.has_state(State::Dead));
    }

    #[test]
    fn dormancy() {
        let mut eng = started_engine();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");
        let p3 = add_player(&mut eng, 0, Role::Civilian, "p3");
        let book_id = quick_notebook(&mut eng, 0, p1, false);

        quick_lend(&mut eng, 0, book_id, p1, p2);
        quick_kill(&mut eng, 0, true, true, true, p1);

        let notebook = get_notebook(&eng, book_id).unwrap();
        assert!(notebook.get_dormant_owner() == Some(p1));
        assert!(notebook.get_true_owner() == Some(p2));

        quick_lend(&mut eng, 0, book_id, p2, p3);

        let notebook = get_notebook(&eng, book_id).unwrap();
        assert!(notebook.owner == Some(p3));

        quick_revive(&mut eng, 0, false, p1);

        let notebook = get_notebook(&eng, book_id).unwrap();
        assert!(notebook.get_dormant_owner().is_none());
        assert!(notebook.get_true_owner() == Some(p1));
        assert!(notebook.owner == Some(p1));
    }

    // The original owner is told their book is a decoy the moment they receive it, and so is System
    // for the admin inspector — those two and nobody else.
    #[test]
    fn the_original_owner_is_told_the_book_is_fake() {
        let mut eng = started_engine();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");

        let (_, ctx) = eng
            .execute(ActionRequest {
                actor: ActionActor::System,
                timestamp: 0,
                payload: Action::CreateAndGiveNotebook(CreateAndGiveNotebook {
                    fake: true,
                    actor_id: p1,
                    volatile: false,
                }),
            })
            .unwrap();

        let recipients = fake_status(&ctx);
        assert_eq!(recipients.len(), 2);
        assert!(recipients.contains(&(CommandRecipient::Actor(p1), true)));
        assert!(recipients.contains(&(CommandRecipient::System, true)));
    }

    // Owning a book is not being told about it. Kill its original owner and the book falls to you,
    // but the fake status was theirs alone — you inherit the decoy without ever being told it is
    // one, left to deduce it from a write that fails to kill.
    #[test]
    fn an_inheritor_is_never_told_the_book_is_fake() {
        let mut eng = started_engine();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");
        let fake_book = quick_notebook(&mut eng, 0, p1, true);
        let p2_book = quick_notebook(&mut eng, 0, p2, false);

        // p2 writes p1's true name in their own real book: p1 dies and the fake book falls to p2.
        let (_, ctx) = eng
            .execute(ActionRequest {
                actor: ActionActor::Player(p2),
                timestamp: 0,
                payload: Action::WriteName(WriteName {
                    true_name: "p1".into(),
                    death_message: None,
                    notebook_id: p2_book,
                    delay: 0,
                }),
            })
            .unwrap();

        assert!(get_actor(&eng, p1).unwrap().has_state(State::Dead));
        assert!(get_actor(&eng, p2).unwrap().has_notebook(fake_book));
        assert!(fake_status(&ctx).is_empty());
    }

    // Flipping the flag restates it to the original owner (and System), so a book turned real is
    // known to be real by the one person entitled to know.
    #[test]
    fn changing_the_fake_status_restates_it() {
        let mut eng = started_engine();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let book_id = quick_notebook(&mut eng, 0, p1, true);

        let (_, ctx) = eng
            .execute(ActionRequest {
                actor: ActionActor::Admin,
                timestamp: 0,
                payload: Action::SetNotebookFake(SetNotebookFake {
                    notebook_id: book_id,
                    fake: false,
                }),
            })
            .unwrap();

        let recipients = fake_status(&ctx);
        assert_eq!(recipients.len(), 2);
        assert!(recipients.contains(&(CommandRecipient::Actor(p1), false)));
        assert!(recipients.contains(&(CommandRecipient::System, false)));

        assert!(!get_notebook(&eng, book_id).unwrap().fake);
    }
}
