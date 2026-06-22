use indexmap::IndexMap;

use crate::actor::{
    modifier::{Modifier, Modifiers},
    state::State,
};

pub type StateModifierMap = IndexMap<State, Modifiers>;

pub fn default_state_modifiers() -> StateModifierMap {
    let mut map = IndexMap::new();

    map.insert(State::Dead, Modifiers::all()); // later restrict this to some set. there
    // may be positive modifiers in the future
    map.insert(
        State::Incarcerated,
        Modifier::NoPresence
            | Modifier::NoContact
            | Modifier::NoNotebookPassage
            | Modifier::NoNotebookUsage,
    );
    map.insert(
        State::Kidnapped,
        Modifier::NoPresence
            | Modifier::NoContact
            | Modifier::NoNotebookUsage
            | Modifier::NoNotebookPassage,
    );
    map.insert(
        State::Custody,
        Modifier::NoNotebookPassage | Modifier::NoNotebookUsage,
    );
    map.insert(
        State::Ipp,
        Modifier::StrengthenedPresence | Modifier::WriteImmunity,
    );

    map
}
