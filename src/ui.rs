use std::sync::mpsc::{self, Receiver};

use eframe::egui::{self, ColorImage, TextEdit, TextureHandle};
use tokio::runtime::Runtime;

use crate::{
    api::BoomlingsApi,
    icon_renderer,
    models::{CreatedLevel, SearchEntry, SearchType},
    storage,
};

struct SearchOutput {
    results: String,
    created_levels: Vec<CreatedLevel>,
    icon_image: Option<ColorImage>,
    icon_error: Option<String>,
}

pub struct GdInfoApp {
    query: String,
    search_type: SearchType,
    results: String,
    history: Vec<SearchEntry>,
    created_levels: Vec<CreatedLevel>,
    player_icon: Option<TextureHandle>,
    player_icon_error: Option<String>,
    comment_page: u32,
    runtime: Option<Runtime>,
    api: Option<BoomlingsApi>,
    pending: Option<Receiver<SearchOutput>>,
    searching: bool,
}

impl GdInfoApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_pixels_per_point(1.0);

        let (runtime, api, results) = match (Runtime::new(), BoomlingsApi::new()) {
            (Ok(runtime), Ok(api)) => (Some(runtime), Some(api), String::new()),
            (runtime, api) => {
                let mut errors = Vec::new();
                if let Err(error) = runtime {
                    errors.push(format!("Async runtime failed: {error}"));
                }
                if let Err(error) = api {
                    errors.push(format!("HTTP client failed: {error}"));
                }
                (None, None, errors.join("\n"))
            }
        };

        Self {
            query: String::new(),
            search_type: SearchType::Player,
            results,
            history: storage::load_history(),
            created_levels: Vec::new(),
            player_icon: None,
            player_icon_error: None,
            comment_page: 0,
            runtime,
            api,
            pending: None,
            searching: false,
        }
    }

    fn start_search(&mut self, ctx: &egui::Context) {
        let query = self.query.trim().to_owned();
        if query.is_empty() {
            self.results = "Enter a search term.".to_owned();
            return;
        }

        let Some(runtime) = &self.runtime else {
            self.results = "Search unavailable: async runtime failed to start.".to_owned();
            return;
        };
        let Some(api) = self.api.clone() else {
            self.results = "Search unavailable: HTTP client failed to start.".to_owned();
            return;
        };

        let (sender, receiver) = mpsc::channel();
        let search_type = self.search_type;
        let comment_page = if search_type == SearchType::Level {
            self.comment_page
        } else {
            0
        };
        let repaint_ctx = ctx.clone();

        self.pending = Some(receiver);
        self.searching = true;
        self.created_levels.clear();
        self.player_icon = None;
        self.player_icon_error = None;
        self.results = "Searching...".to_owned();
        storage::remember_search(
            &mut self.history,
            SearchEntry {
                query: query.clone(),
                search_type,
            },
        );

        runtime.spawn(async move {
            let output = match search_type {
                SearchType::Player => match api.search_player(&query).await {
                    Ok(profile) => {
                        let icon_result =
                            icon_renderer::load_icon_image(&profile.player.icon).await;
                        SearchOutput {
                            results: profile.to_result_text(),
                            created_levels: profile.created_levels,
                            icon_image: icon_result.as_ref().ok().cloned(),
                            icon_error: icon_result.err().map(|error| error.to_string()),
                        }
                    }
                    Err(error) => SearchOutput {
                        results: format!("Error: {error}"),
                        created_levels: Vec::new(),
                        icon_image: None,
                        icon_error: None,
                    },
                },
                SearchType::Level => match api.search_level(&query, comment_page).await {
                    Ok(level) => SearchOutput {
                        results: level.to_result_text(),
                        created_levels: Vec::new(),
                        icon_image: None,
                        icon_error: None,
                    },
                    Err(error) => SearchOutput {
                        results: format!("Error: {error}"),
                        created_levels: Vec::new(),
                        icon_image: None,
                        icon_error: None,
                    },
                },
            };

            let _ = sender.send(output);
            repaint_ctx.request_repaint();
        });
    }

    fn receive_pending(&mut self, ctx: &egui::Context) {
        if let Some(receiver) = &self.pending {
            if let Ok(output) = receiver.try_recv() {
                self.results = output.results;
                self.created_levels = output.created_levels;
                self.player_icon = output
                    .icon_image
                    .map(|image| icon_renderer::texture_from_image(ctx, image));
                self.player_icon_error = output.icon_error;
                self.searching = false;
                self.pending = None;
            }
        }
    }

    fn show_player_icon(&self, ui: &mut egui::Ui) {
        if self.player_icon.is_none() && self.player_icon_error.is_none() {
            return;
        }

        ui.vertical(|ui| {
            ui.label("Player Icon:");
            if let Some(texture) = &self.player_icon {
                ui.image((texture.id(), egui::vec2(72.0, 72.0)));
            } else {
                ui.label("Icon unavailable");
            }
        });
    }
}

impl eframe::App for GdInfoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.receive_pending(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);

                ui.label("Search:");
                let response = ui.add(
                    TextEdit::singleline(&mut self.query)
                        .desired_width(f32::INFINITY)
                        .hint_text("Username, level name, or level ID"),
                );
                if response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                    self.start_search(ctx);
                }

                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label("Search Type:");
                        ui.horizontal(|ui| {
                            ui.radio_value(&mut self.search_type, SearchType::Player, "Player");
                            ui.radio_value(&mut self.search_type, SearchType::Level, "Level");
                        });

                        if self.search_type == SearchType::Level {
                            ui.horizontal(|ui| {
                                ui.label("Comment Page:");

                                if ui
                                    .add_enabled(
                                        !self.searching && self.comment_page > 0,
                                        egui::Button::new("Prev"),
                                    )
                                    .clicked()
                                {
                                    self.comment_page -= 1;
                                    self.start_search(ctx);
                                }

                                ui.add(
                                    egui::DragValue::new(&mut self.comment_page)
                                        .speed(1)
                                        .range(0..=999),
                                );

                                if ui
                                    .add_enabled(!self.searching, egui::Button::new("Load Page"))
                                    .clicked()
                                {
                                    self.start_search(ctx);
                                }

                                if ui
                                    .add_enabled(!self.searching, egui::Button::new("Next"))
                                    .clicked()
                                {
                                    self.comment_page += 1;
                                    self.start_search(ctx);
                                }
                            });
                        }

                        ui.horizontal(|ui| {
                            if ui
                                .add_enabled(!self.searching, egui::Button::new("Search"))
                                .clicked()
                            {
                                self.start_search(ctx);
                            }

                            if ui.button("Copy Results").clicked() {
                                ctx.copy_text(self.results.clone());
                            }

                            if ui.button("Clear Results").clicked() {
                                self.results.clear();
                                self.player_icon = None;
                                self.player_icon_error = None;
                            }
                        });
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                        self.show_player_icon(ui);
                    });
                });

                if !self.history.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Recent:");
                        for entry in self.history.clone() {
                            if ui.small_button(entry.label()).clicked() {
                                self.query = entry.query;
                                self.search_type = entry.search_type;
                                self.comment_page = 0;
                            }
                        }
                    });
                }

                ui.separator();
                ui.label("Results:");
                let result_height = if self.created_levels.is_empty() {
                    320.0
                } else {
                    220.0
                };
                ui.add_sized(
                    [ui.available_width(), result_height],
                    TextEdit::multiline(&mut self.results)
                        .desired_width(f32::INFINITY)
                        .font(egui::TextStyle::Monospace)
                        .lock_focus(true),
                );

                if !self.created_levels.is_empty() {
                    ui.separator();
                    ui.label("Created Levels:");

                    for level in self.created_levels.clone() {
                        let label = format!(
                            "{} | ID {} | Downloads {} | Likes {} | {}",
                            level.name, level.id, level.downloads, level.likes, level.difficulty
                        );

                        if ui.button(label).clicked() {
                            self.query = level.id;
                            self.search_type = SearchType::Level;
                            self.comment_page = 0;
                            self.start_search(ctx);
                        }
                    }
                }
            });
        });
    }
}
