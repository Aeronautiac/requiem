use std::time::{SystemTime, UNIX_EPOCH};

use eframe::App;
use egui::ViewportCommand;
use lawliet_types::{
    action::{Action, ActionActor, ActionRequest, AddPlayer},
    role::Role,
};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::AppExecResult;

#[derive(Clone, Copy)]
enum UserInput {
    Exit,
    AddPlayer,
}

struct GameViewport {}

pub struct Application {
    input_wrt: UnboundedSender<ActionRequest>,
    output_rcv: UnboundedReceiver<AppExecResult>,
    input: Option<UserInput>,
    waiting_input: Option<UserInput>,
}

impl Application {
    pub fn new(
        input_wrt: UnboundedSender<ActionRequest>,
        output_rcv: UnboundedReceiver<AppExecResult>,
    ) -> Self {
        Application {
            input_wrt,
            output_rcv,
            input: None,
            waiting_input: None,
        }
    }

    fn exit(&mut self) {}

    fn update(&mut self, ctx: &egui::Context) {
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();

        // dont process inputs until the current one has been responded to
        if let Some(input) = self.input
            && self.waiting_input.is_none()
        {
            match input {
                UserInput::Exit => {
                    self.exit();
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
                UserInput::AddPlayer => self
                    .input_wrt
                    .send(ActionRequest {
                        timestamp: time,
                        actor: ActionActor::Admin,
                        payload: Action::AddPlayer(AddPlayer {
                            true_name: "John Pork".into(),
                            starting_role: Role::Civilian,
                        }),
                    })
                    .unwrap(),
            }
            self.waiting_input = Some(input);
        }

        // try to get input response
        if let Ok(response_data) = self.output_rcv.try_recv() {
            self.waiting_input = None;
            dbg!(response_data);
        }
    }
}

impl App for Application {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.input = None;

        ui.label("Hello world!");

        if ui.button("Add player").clicked() {
            self.input = Some(UserInput::AddPlayer)
        }

        if ui.button("Quit").clicked() {
            self.input = Some(UserInput::Exit)
        }
    }
}
