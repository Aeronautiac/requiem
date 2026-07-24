pub const TICKET_LIMIT: usize = 5; // tickets per key
pub const TICKET_TIMEOUT: u64 = 15; // seconds
pub const OUTBOX_BUF_SIZE: usize = 50;
pub const HEARTBEAT_TIMEOUT: u64 = 5; // seconds; recv task reaps if no frame arrives within this
pub const HEARTBEAT_INTERVAL: u64 = 2; // seconds; send task pings this often -- must be < HEARTBEAT_TIMEOUT
