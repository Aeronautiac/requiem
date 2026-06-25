/*
* Armonia is a dev tool UI that directly hosts a lawliet instance with zero middleware. It is
* designed for developers and allows for things like switching player perspectives, rewinding, etc.
* It communicates with a lawliet runtime via Unix pipes.
*
* When networking middleware is added, the supervisor/coordinator here gets replaced by a network
* client, and the Tauri commands stay the same from the frontend's perspective.
*/

use std::{env::current_exe, process::Stdio};

use lawliet_types::{action::ActionRequest, engine::{ExecutionResult, IpcExecutionResult}};
use serde::Serialize;
use tauri::State;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines},
    process::{ChildStdin, ChildStdout},
    runtime::Runtime,
    sync::{
        mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
        oneshot,
    },
};

// ////////////////////////////////////////////////////////////
// PROCESS MANAGEMENT
// ////////////////////////////////////////////////////////////

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

        let _ = child.wait().await;
    }
}

// Each action from the frontend arrives with a oneshot sender to reply on.
// The coordinator processes one action at a time
// On crash, the in-flight action gets a Crashed result and replay resaturates the engine.
async fn coordinator_loop(
    mut fd_rcv: UnboundedReceiver<(ChildStdin, ChildStdout)>,
    mut action_rcv: UnboundedReceiver<(ActionRequest, oneshot::Sender<AppExecution>)>,
) {
    let mut stdin: Option<ChildStdin> = None;
    let mut stdout: Option<Lines<BufReader<ChildStdout>>> = None;
    let mut awaiting: Option<(ActionRequest, oneshot::Sender<AppExecution>)> = None;
    let mut valid_inputs: Vec<ActionRequest> = vec![];
    let mut to_discard: usize = 0;

    loop {
        tokio::select! {
            // crash or initial boot — resaturate the new child with previously valid inputs
            Some((new_in, new_out)) = fd_rcv.recv() => {
                if let Some((_, tx)) = awaiting.take() {
                    let _ = tx.send(AppExecution { exec_result: AppExecResult::Crashed });
                }

                stdin = Some(new_in);
                stdout = Some(BufReader::new(new_out).lines());

                for input in &valid_inputs {
                    let line = serde_json::to_string(input).unwrap() + "\n";
                    if stdin.as_mut().unwrap().write_all(line.as_bytes()).await.is_err() {
                        stdin = None;
                        break;
                    }
                }
                to_discard = valid_inputs.len();
            }

            // new action from frontend — only accept when pipe is live and nothing in flight
            Some((action, tx)) = action_rcv.recv(), if stdin.is_some() && awaiting.is_none() => {
                let line = serde_json::to_string(&action).unwrap() + "\n";
                awaiting = Some((action, tx));
                let _ = stdin.as_mut().unwrap().write_all(line.as_bytes()).await;
            }

            // response from child
            line = async { stdout.as_mut().unwrap().next_line().await }, if stdout.is_some() => {
                match line {
                    Ok(Some(text)) => {
                        let result: ExecutionResult = serde_json::from_str(&text).unwrap();

                        if to_discard > 0 {
                            // replay response — discard
                            to_discard -= 1;
                        } else if let Some((action, tx)) = awaiting.take() {
                            if result.is_ok() {
                                valid_inputs.push(action);
                            }
                            let _ = tx.send(AppExecution {
                                exec_result: AppExecResult::Standard(result.into()),
                            });
                        }
                    }
                    _ => stdout = None,
                }
            }
        }
    }
}

// ////////////////////////////////////////////////////////////
// IPC TYPES
// ////////////////////////////////////////////////////////////

#[derive(Debug, Serialize)]
pub enum AppExecResult {
    Standard(IpcExecutionResult),
    Crashed,
}

#[derive(Debug, Serialize)]
pub struct AppExecution {
    pub exec_result: AppExecResult,
}

// ////////////////////////////////////////////////////////////
// TAURI STATE & COMMANDS
// ////////////////////////////////////////////////////////////

struct AppState {
    action_tx: UnboundedSender<(ActionRequest, oneshot::Sender<AppExecution>)>,
}

#[tauri::command]
async fn send_action(
    action: ActionRequest,
    state: State<'_, AppState>,
) -> Result<AppExecution, String> {
    let (tx, rx) = oneshot::channel();
    state
        .action_tx
        .send((action, tx))
        .map_err(|e| e.to_string())?;
    rx.await.map_err(|e| e.to_string())
}

// ////////////////////////////////////////////////////////////
// ENTRY POINT
// ////////////////////////////////////////////////////////////

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let (action_tx, action_rx) = unbounded_channel();

    std::thread::spawn(move || {
        let rt = Runtime::new().unwrap();
        rt.block_on(async move {
            let (fd_tx, fd_rx) = unbounded_channel();
            let supervisor = tokio::spawn(supervisor_loop(fd_tx));
            let coordinator = tokio::spawn(coordinator_loop(fd_rx, action_rx));
            let _ = tokio::join!(supervisor, coordinator);
        });
    });

    tauri::Builder::default()
        .manage(AppState { action_tx })
        .invoke_handler(tauri::generate_handler![send_action])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
