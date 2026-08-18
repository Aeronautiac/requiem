// the runtime child process. reads RuntimeInputs from stdin, runs each through the Simulation, and
// writes RuntimeOutputs to stdout -- one JSON object per line, in order. yagami spawns this and
// speaks the pipe; the simulation is deterministic from its input stream, so a rebuild is just
// re-feeding that stream to a fresh child.

use std::io::{self, Write};

use lawliet_types::action::{Action, ActionRequest, InitializeEngine};
use yagami_runtime::{PipeFrame, RuntimeOutput, Simulation};

fn main() {
    let mut sim = Simulation::new();

    let stdin = io::stdin();
    let stdout = io::stdout();
    let stdin_lock = stdin.lock();
    let mut stdout_lock = stdout.lock();

    let stream = serde_json::Deserializer::from_reader(stdin_lock).into_iter::<PipeFrame>();

    for frame in stream {
        let frame = frame.unwrap();

        // the InitializeEngine action seeds the engine's RNG; re-seed the name pool from the same
        // seed so display-name draws reproduce across rebuilds. this is the one input that touches
        // determinism setup rather than advancing the simulation.
        if let yagami_runtime::RuntimeInput::Action(ActionRequest {
            payload: Action::InitializeEngine(InitializeEngine { seed }),
            ..
        }) = &frame.input
        {
            sim.reseed_sim(*seed as u64);
        }

        let output: RuntimeOutput = sim.process(&frame.input, frame.caller.as_ref(), frame.version);
        serde_json::to_writer(&mut stdout_lock, &output).unwrap();
        stdout_lock.write_all(b"\n").unwrap();
        stdout_lock.flush().unwrap();
    }
}
