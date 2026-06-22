use crate::action::{ActionContext, ActionError, ActionResponse};

pub type ExecutionResult = Result<(ActionResponse, ActionContext), ActionError>;
