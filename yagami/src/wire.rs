// The wire: what goes over the socket (server -> client) and over the runtime pipe. Defined in the
// shared `yagami-wire` crate so the runtime emits the same shapes yagami appends to history; this
// module re-exports them and keeps nothing local.

pub use yagami_wire::{
    ActionOutcome, AdminControl, Batch, BatchKind, ControlOutcome, ControlResponse, ExecOutcome,
    LogCommand, LogType, MetaControl, Output, OutputData, Recipient, ResponsePair, ServerCmd,
    ServerInput, SimControl, SimControlData, SimOutput, VersionedInput, privileges_to_wire,
};
