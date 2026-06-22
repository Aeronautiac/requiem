use std::io;
use std::io::Write;

use lawliet::engine::Engine;
use lawliet_types::action::ActionRequest;

// listen for actions through stdin, deserialize, send them into the engine, and send
// out their responses via stdout

fn main() {
    let mut eng = Engine::new();

    let stdin = io::stdin();
    let stdout = io::stdout();
    let stdin_lock = stdin.lock();
    let mut stdout_lock = stdout.lock();

    let stream = serde_json::Deserializer::from_reader(stdin_lock).into_iter::<ActionRequest>();
    for input in stream {
        let rqst = input.unwrap();
        let res = eng.execute(rqst);
        serde_json::to_writer(&mut stdout_lock, &res).unwrap();
        stdout_lock.write_all(b"\n").unwrap();
        stdout_lock.flush().unwrap();
    }
}
