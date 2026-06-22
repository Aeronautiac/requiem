pub use lawliet_types::command::{Command, CommandPayload};

use crate::actor::modifier::Modifiers;

#[derive(Clone)]
pub struct DeferredCommand {
    pub payload: CommandPayload,
    pub blocking_modifiers: Modifiers, // when the target has none of these modifiers, they may
                                       // receive the command
}
