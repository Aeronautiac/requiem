use serde::Serialize;
use crate::action::{ActionContext, ActionError, ActionResponse};

pub type ExecutionResult = Result<(ActionResponse, ActionContext), ActionError>;

// IPC-safe version of ExecutionResult. std::result::Result generates incorrect
// specta types, so we convert to this before sending across the Tauri boundary.
#[derive(Debug, Serialize)]
pub enum IpcExecutionResult {
    Ok((ActionResponse, ActionContext)),
    Err(ActionError),
}

impl From<ExecutionResult> for IpcExecutionResult {
    fn from(r: ExecutionResult) -> Self {
        match r {
            Ok(v) => IpcExecutionResult::Ok(v),
            Err(e) => IpcExecutionResult::Err(e),
        }
    }
}
