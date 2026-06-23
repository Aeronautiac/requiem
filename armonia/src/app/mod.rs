use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

use eframe::App;
use egui::{Align, ComboBox, Layout, Panel, Popup, PopupCloseBehavior, Ui, ViewportCommand};
use lawliet_types::{
    action::{Action, ActionActor, ActionRequest, AddPlayer},
    common::ActorKey,
    role::Role,
};
use strum::IntoEnumIterator;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::{AppExecResult, AppExecution};

struct PrivilegedPlayerInfo {
    role: Role,
    true_name: String,
}

struct PlayerIdentity {
    key: ActorKey,
    display_name: String,
}

struct Player {
    id: PlayerIdentity,
    privileged_info: Option<PrivilegedPlayerInfo>,
}

struct GameViewport {
    players: HashMap<ActorKey, Player>,
}

impl Default for GameViewport {
    fn default() -> Self {
        Self {
            players: HashMap::new(),
        }
    }
}

#[derive(Hash)]
enum Viewer {
    Admin,
    Player(ActorKey),
}

struct GameState {
    views: HashMap<Viewer, GameViewport>,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            views: HashMap::new(),
        }
    }
}

#[derive(Clone, Copy)]
enum UserInput {
    Exit,
    AddPlayer,
}

struct NewPlayerSettings {
    true_name: String,
    role: Role,
}

impl Default for NewPlayerSettings {
    fn default() -> Self {
        Self {
            true_name: String::new(),
            role: Role::Civilian,
        }
    }
}

struct UIState {
    input: Option<UserInput>,
    waiting_input: Option<UserInput>,
    selected_actor: ActionActor,
    new_player_state: NewPlayerSettings,
    show_add_players_popup: bool,
}

impl Default for UIState {
    fn default() -> Self {
        Self {
            input: None,
            waiting_input: None,
            selected_actor: ActionActor::Admin,
            new_player_state: NewPlayerSettings::default(),
            show_add_players_popup: false,
        }
    }
}

pub struct Application {
    input_wrt: UnboundedSender<ActionRequest>,
    output_rcv: UnboundedReceiver<AppExecution>,
    ui_state: UIState,
    game_state: GameState,
}

impl Application {
    pub fn new(
        input_wrt: UnboundedSender<ActionRequest>,
        output_rcv: UnboundedReceiver<AppExecution>,
    ) -> Self {
        Application {
            input_wrt,
            output_rcv,
            ui_state: UIState::default(),
            game_state: GameState::default(),
        }
    }

    fn exit(&mut self) {
        dbg!("exiting");
    }

    fn update(&mut self, ctx: &egui::Context) {
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();

        // dont process inputs until the current one has been responded to
        if let Some(input) = self.ui_state.input
            && self.ui_state.waiting_input.is_none()
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
                            true_name: self.ui_state.new_player_state.true_name.clone(),
                            starting_role: self.ui_state.new_player_state.role,
                        }),
                    })
                    .unwrap(),
            }
            self.ui_state.waiting_input = Some(input);
        }

        // try to get input response
        if let Ok(execution) = self.output_rcv.try_recv() {
            dbg!(&execution);

            match execution.exec_result {
                AppExecResult::Standard(result) => {
                    if let Ok(response) = result {
                        if matches!(execution.action_req.payload, Action::AddPlayer(_)) {
                            dbg!("received response to add player");
                        }
                    } else {
                        dbg!("error");
                    }
                }
                _ => {
                    dbg!("crashed");
                }
            }

            self.ui_state.waiting_input = None;
        }
    }

    // UI HELPERS
    fn player_selection(&mut self, ui: &mut Ui) {
        ComboBox::from_label("Player Selection")
            .selected_text(format!("{:?}", self.ui_state.selected_actor))
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut self.ui_state.selected_actor,
                    ActionActor::Admin,
                    "Admin",
                );
            });

        let button_response = ui.button("Add Players");
        if button_response.clicked() {
            self.ui_state.show_add_players_popup = true;
        }
        Popup::from_response(&button_response)
            .open_bool(&mut self.ui_state.show_add_players_popup)
            .close_behavior(PopupCloseBehavior::IgnoreClicks) // <- no more auto-close on any click
            .show(|ui| {
                ui.horizontal(|ui| {
                    ui.label("True Name");
                    ui.text_edit_singleline(&mut self.ui_state.new_player_state.true_name);
                });

                ComboBox::from_label("Role")
                    .selected_text(format!("{:?}", self.ui_state.new_player_state.role))
                    .show_ui(ui, |ui| {
                        for role in Role::iter() {
                            ui.selectable_value(
                                &mut self.ui_state.new_player_state.role,
                                role,
                                format!("{role:?}"),
                            );
                        }
                    });

                ui.horizontal(|ui| {
                    if ui.button("Add").clicked() {
                        self.ui_state.input = Some(UserInput::AddPlayer);
                    }
                    if ui.button("Close").clicked() {
                        ui.close();
                    }
                });
            });
    }
}

impl App for Application {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.ui_state.input = None;

        Panel::bottom("bottom bar")
            .resizable(false)
            .show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    self.player_selection(ui);

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("Quit").clicked() {
                            self.ui_state.input = Some(UserInput::Exit)
                        }
                    });
                });
            });
    }
}
