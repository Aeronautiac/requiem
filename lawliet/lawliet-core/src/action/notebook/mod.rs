pub mod add_notebook;
pub mod destroy_notebook;
pub mod create_and_give_notebook;
pub mod give_notebook;
pub mod lend_notebook;
pub mod notebook_scheduled_kill;
pub mod return_dormant_books;
pub mod set_books_dormant;
pub mod set_borrowers_to_owners;
pub mod set_notebook_possession;
pub mod take_notebook;
pub mod write_name;

#[cfg(test)]
mod notebook_tests {
    use crate::{
        actor::state::State,
        config::role::Role,
        engine::Engine,
        helpers::{get_actor, get_notebook},
        test_helpers::*,
    };

    // a fake notebook should not kill someone
    #[test]
    fn fake_write_delayed() {
        let mut eng = Engine::new();
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
        let mut eng = Engine::new();
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
        let mut eng = Engine::new();
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
        let mut eng = Engine::new();
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
        let mut eng = Engine::new();
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
        let mut eng = Engine::new();
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
        let mut eng = Engine::new();
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
        let mut eng = Engine::new();
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
        let mut eng = Engine::new();
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
        let mut eng = Engine::new();
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
        let mut eng = Engine::new();
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
        let mut eng = Engine::new();
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
        let mut eng = Engine::new();
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
        let mut eng = Engine::new();
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
        let mut eng = Engine::new();
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
}
