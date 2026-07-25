pub const TICKET_LIMIT: usize = 5; // tickets per key
pub const TICKET_TIMEOUT: u64 = 15; // seconds
pub const OUTBOX_BUF_SIZE: usize = 50;
pub const HEARTBEAT_TIMEOUT: u64 = 5; // seconds; recv task reaps if no frame arrives within this
pub const HEARTBEAT_INTERVAL: u64 = 2; // seconds; send task pings this often -- must be < HEARTBEAT_TIMEOUT
// seconds the coordinator will wait on the engine for a line it is owed before declaring the child
// hung and having it killed. an action is single-digit milliseconds of work, so this only has to
// clear the worst plausible catchup burst -- not be a performance budget.
pub const ENGINE_TIMEOUT: u64 = 5;
