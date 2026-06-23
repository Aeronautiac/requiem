use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

use eframe::App;
use egui::{Align, ComboBox, Layout, Panel, Popup, PopupCloseBehavior, Ui, ViewportCommand};
use lawliet_types::{
    action::{Action, ActionActor, ActionRequest, ActionResponse, AddPlayer},
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

#[derive(Default)]
struct GameViewport {
    players: HashMap<ActorKey, Player>,
}

#[derive(Hash, PartialEq, Eq, Debug, Clone, Copy)]
enum Viewer {
    Admin,
    Player(ActorKey),
}

struct GameState {
    views: HashMap<Viewer, GameViewport>,
}

impl GameState {
    // add view for the player, add info to everyone else's views, only show privileged info to the
    // player and the admin
    fn add_player(&mut self, key: ActorKey, settings: &NewPlayerSettings) {
        // create view
        self.views
            .insert(Viewer::Player(key), GameViewport::default());

        // update views
        for (viewer, view) in self.views.iter_mut() {
            let mut privileged = None;
            if *viewer == Viewer::Admin || *viewer == Viewer::Player(key) {
                privileged = Some(PrivilegedPlayerInfo {
                    role: settings.role,
                    true_name: settings.true_name.clone(),
                });
            }

            let player = Player {
                id: PlayerIdentity {
                    display_name: settings.display_name.clone(),
                    key,
                },
                privileged_info: privileged,
            };

            view.players.insert(key, player);
        }
    }

    fn admin_view(&self) -> &GameViewport {
        &self.views[&Viewer::Admin]
    }
}

impl Default for GameState {
    fn default() -> Self {
        let mut views = HashMap::new();
        views.insert(Viewer::Admin, GameViewport::default());
        Self { views }
    }
}

#[derive(Clone)]
struct NewPlayerSettings {
    display_name: String,
    true_name: String,
    role: Role,
}

impl Default for NewPlayerSettings {
    fn default() -> Self {
        Self {
            display_name: String::new(),
            true_name: String::new(),
            role: Role::Civilian,
        }
    }
}

#[derive(Clone)]
enum UserInput {
    Exit,
    AddPlayer(NewPlayerSettings),
}

struct UIState {
    input: Option<UserInput>,
    waiting_input: Option<UserInput>,
    selected_viewer: Viewer,
    new_player_state: NewPlayerSettings,
    show_add_players_popup: bool,
}

impl Default for UIState {
    fn default() -> Self {
        Self {
            input: None,
            waiting_input: None,
            selected_viewer: Viewer::Admin,
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
        if let Some(input) = self.ui_state.input.clone()
            && self.ui_state.waiting_input.is_none()
        {
            match &input {
                UserInput::Exit => {
                    self.exit();
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
                UserInput::AddPlayer(_) => self
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
                        // if we get a response with no waiting input, something is wrong
                        match self
                            .ui_state
                            .waiting_input
                            .as_mut()
                            .expect("received a response with no waiting input")
                        {
                            UserInput::AddPlayer(settings) => {
                                let ActionResponse::AddPlayer(response_data) = response.0 else {
                                    unreachable!()
                                };
                                self.game_state.add_player(response_data.id, settings);
                            }
                            _ => {}
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
            .selected_text(format!("{:?}", self.ui_state.selected_viewer))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.ui_state.selected_viewer, Viewer::Admin, "Admin");

                for (key, player) in self.game_state.admin_view().players.iter() {
                    ui.selectable_value(
                        &mut self.ui_state.selected_viewer,
                        Viewer::Player(*key),
                        &player.id.display_name,
                    );
                }
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
                    ui.label("Display Name");
                    ui.text_edit_singleline(&mut self.ui_state.new_player_state.display_name);
                });

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
                        self.ui_state.input =
                            Some(UserInput::AddPlayer(self.ui_state.new_player_state.clone()));
                    }
                    if ui.button("Close").clicked() {
                        ui.close();
                    }
                });
            });
    }

    fn player(&self, ui: &mut Ui, player: &Player) {
        if ui.button(&player.id.display_name).clicked() {
            dbg!("player interaction");
        }
    }

    fn player_list(&mut self, ui: &mut Ui) {
        for (id, player) in self.game_state.views[&self.ui_state.selected_viewer]
            .players
            .iter()
        {
            self.player(ui, player);
        }
    }
}

impl App for Application {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.ui_state.input = None;

        Panel::top("top bar")
            .resizable(false)
            .show_inside(ui, |ui| {
                ui.with_layout(
                    Layout::centered_and_justified(egui::Direction::LeftToRight),
                    |ui| ui.label("Requiem"),
                );
            });

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

        Panel::right("right panel")
            .resizable(true)
            .show_inside(ui, |ui| {
                self.player_list(ui);
            });

        Panel::left("left panel")
            .resizable(true)
            .show_inside(ui, |ui| {});
    }
}
