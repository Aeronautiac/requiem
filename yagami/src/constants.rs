pub const TICKET_LIMIT: usize = 5; // tickets per key
pub const TICKET_TIMEOUT: u64 = 15; // seconds
pub const OUTBOX_BUF_SIZE: usize = 50;
pub const HEARTBEAT_TIMEOUT: u64 = 15; // seconds; recv task reaps if nothing arrives within this
// time period
pub const HEARTBEAT_INTERVAL: u64 = 5;
pub const ENGINE_TIMEOUT: u64 = 5;
// seconds between null ticks.
pub const NULL_TICK_INTERVAL: u64 = 5;
// boot retry: base delay in ms for the exponential backoff between failed spawn/replay attempts.
pub const BOOT_RETRY_BASE_MS: u64 = 500;
// after this many consecutive failed boots a game gives up and tears itself down (a fresh game is
// reported as failed and never written to the DB) rather than retrying forever.
pub const BOOT_MAX_RETRIES: u32 = 10;
// the maximum number of commands in a batch before splitting
pub const BATCH_SIZE: usize = 32;
