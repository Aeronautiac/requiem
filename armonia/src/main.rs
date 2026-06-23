/*
* Armonia is a dev tool ui that directly hosts a lawliet instance with zero middleware. It is designed for
* developers and allows for things like switching player perspectives, rewinding, etc... all on the
* host PC.
* It communicates with a lawliet runtime via Unix pipes.
*/

pub mod app;

use eframe::NativeOptions;
use lawliet_types::{action::ActionRequest, engine::ExecutionResult};
use std::{env::current_exe, process::Stdio};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines},
    process::{ChildStdin, ChildStdout},
    runtime::Runtime,
    sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
};

use crate::app::Application;

// What needs to be done?
// A child process must be spawned, input must be fed into it, and output must be taken out of it
// Input sequences must be saved.
// Crashes must be detected and must trigger a reboot with a resaturation.
//
// The process supervisor thread should solely focus on rebooting and sending out file descriptors.
// It also sends out crash notifications.
// The reader just reads from the process output pipe and feeds it back to the application
// The writer reads from the application input channel and sends it into the child process via pipe.
// It also saves an input sequence of successful actions which it sends into the child whenever the
// process restarts. If a pipe write fails (due to the child crashing), it does not write the action
// into the input buffer.
//
// The reader/writer handles everything else

async fn supervisor_loop(fd_wrt: UnboundedSender<(ChildStdin, ChildStdout)>) {
    loop {
        let mut child = tokio::process::Command::new(
            current_exe()
                .expect("failed to get current exe")
                .parent()
                .expect("failed to get parent path")
                .join("lawliet-runtime"),
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .expect("failed to boot lawliet runtime");

        if fd_wrt
            .send((child.stdin.take().unwrap(), child.stdout.take().unwrap()))
            .is_err()
        {
            return;
        }

        dbg!("child booted, now waiting");

        let _ = child.wait().await;

        dbg!("child terminated");
    }
}

async fn coordinator_loop(
    mut fd_rcv: UnboundedReceiver<(ChildStdin, ChildStdout)>,
    output_wrt: UnboundedSender<AppExecution>,
    mut input_rcv: UnboundedReceiver<ActionRequest>,
) {
    let mut stdin: Option<ChildStdin> = None;
    let mut stdout: Option<Lines<BufReader<ChildStdout>>> = None;
    let mut awaiting: Option<ActionRequest> = None;
    let mut valid_inputs: Vec<ActionRequest> = vec![];
    let mut to_discard: usize = 0;

    loop {
        tokio::select! {
            // on crash or initial boot
            Some((new_in, new_out)) = fd_rcv.recv() => {
                dbg!("received fd pair");

                let action_req = awaiting.take();
                if action_req.is_some() {
                    output_wrt.send(AppExecution {
                        exec_result: AppExecResult::Crashed,
                        action_req: action_req.unwrap(),
                    }).ok();
                }

                // update fds
                stdin = Some(new_in);
                stdout = Some(BufReader::new(new_out).lines());

                // resaturate
                for input in &valid_inputs {
                    let line = serde_json::to_string(input).unwrap() + "\n";
                    if stdin.as_mut().unwrap().write_all(line.as_bytes()).await.is_err() {
                        stdin = None; // died during replay, require a reboot
                        break;
                    }
                }
                to_discard = valid_inputs.len();
            }

            // app input — only take it if there's a live pipe and nothing in flight
            Some(action) = input_rcv.recv(), if stdin.is_some() && awaiting.is_none() => {
                let line = serde_json::to_string(&action).unwrap() + "\n";            // app input — only take it if there's a live pipe and nothing in flight
                dbg!("received app input: ", &line);
                awaiting = Some(action);
                let _ = stdin.as_mut().unwrap().write_all(line.as_bytes()).await;
                dbg!("sent app input");
            }

            // child output
            line = async { stdout.as_mut().unwrap().next_line().await }, if stdout.is_some() => {
                dbg!("received response");
                match line {
                    Ok(Some(text)) => {
                        let result: ExecutionResult = serde_json::from_str(&text).unwrap();
                        let action_req = awaiting.take();
                        if let Some(action) = &action_req && result.is_ok() {
                            valid_inputs.push(action.clone());
                        }
                        if to_discard == 0 {
                            output_wrt.send(AppExecution {
                                exec_result: AppExecResult::Standard(result),
                                action_req: action_req.unwrap(),
                            }).ok();
                        } else {
                            to_discard -= 1;
                        }
                    }
                    _ => stdout = None,
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum AppExecResult {
    Standard(ExecutionResult),
    Crashed,
}

#[derive(Debug)]
pub struct AppExecution {
    action_req: ActionRequest,
    exec_result: AppExecResult,
}

fn main() {
    // thread comms
    let (input_wrt, input_rcv) = unbounded_channel::<ActionRequest>();
    let (output_wrt, output_rcv) = unbounded_channel::<AppExecution>();

    std::thread::spawn(move || {
        let rt = Runtime::new().unwrap();
        rt.block_on(async move {
            let (fd_wrt, fd_rcv) = unbounded_channel::<(ChildStdin, ChildStdout)>();

            let supervisor = tokio::spawn(supervisor_loop(fd_wrt));
            let coordinator = tokio::spawn(coordinator_loop(fd_rcv, output_wrt, input_rcv));

            let _ = tokio::join!(supervisor, coordinator);
        });
    });

    // app
    let native_options = NativeOptions::default();

    eframe::run_native(
        "Armonia",
        native_options,
        Box::new(|_cc| Ok(Box::new(Application::new(input_wrt, output_rcv)))),
    )
    .unwrap();
}
