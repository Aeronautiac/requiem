use crate::action::{ActionContext, ActionError, ActionResponse};

// The context (catchup commands accumulated from the job queue) is returned on
// both arms so the frontend can apply world progression even when the requested
// action itself fails.
pub type ExecutionResult =
    Result<(ActionResponse, ActionContext), (ActionError, ActionContext)>;
