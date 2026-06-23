use std::sync::mpsc::{self, Receiver};

use eframe::egui::{self, TextEdit};
use tokio::runtime::Runtime;

use crate::{
    api::BoomlingsApi,
    models::{CreatedLevel, SearchEntry, SearchType},
    storage,
};

struct SearchOutput {
    results: Option<String>,
    created_levels: Option<Vec<CreatedLevel>>,
    show_created_levels: bool,
}

pub struct GdInfoApp {
    query: String,
    search_type: SearchType,
    results: String,
    history: Vec<SearchEntry>,
    created_levels: Vec<CreatedLevel>,
    show_created_levels: bool,
    comment_page: u32,
    created_levels_page: u32,
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
            show_created_levels: false,
            comment_page: 0,
            created_levels_page: 0,
            runtime,
            api,
            pending: None,
            searching: false,
        }
    }

    fn start_search(&mut self, ctx: &egui::Context) {
        self.start_search_with_options(ctx, false);
    }

    fn start_created_levels_page_search(&mut self, ctx: &egui::Context) {
        self.start_search_with_options(ctx, true);
    }

    fn start_search_with_options(&mut self, ctx: &egui::Context, keep_created_levels: bool) {
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
        let created_levels_page = if search_type == SearchType::Player {
            self.created_levels_page
        } else {
            0
        };
        let update_results = !keep_created_levels;
        let repaint_ctx = ctx.clone();

        self.pending = Some(receiver);
        self.searching = true;
        if !keep_created_levels {
            self.created_levels.clear();
            self.show_created_levels = false;
            self.results = "Searching...".to_owned();
        }
        storage::remember_search(
            &mut self.history,
            SearchEntry {
                query: query.clone(),
                search_type,
            },
        );

        runtime.spawn(async move {
            let output = match search_type {
                SearchType::Player => match api.search_player(&query, created_levels_page).await {
                    Ok(profile) => SearchOutput {
                        results: update_results.then(|| profile.to_result_text()),
                        created_levels: Some(profile.created_levels),
                        show_created_levels: true,
                    },
                    Err(error) => SearchOutput {
                        results: Some(format!("Error: {error}")),
                        created_levels: (!keep_created_levels).then(Vec::new),
                        show_created_levels: keep_created_levels,
                    },
                },
                SearchType::Level => match api.search_level(&query, comment_page).await {
                    Ok(level) => SearchOutput {
                        results: Some(level.to_result_text()),
                        created_levels: Some(Vec::new()),
                        show_created_levels: false,
                    },
                    Err(error) => SearchOutput {
                        results: Some(format!("Error: {error}")),
                        created_levels: Some(Vec::new()),
                        show_created_levels: false,
                    },
                },
            };

            let _ = sender.send(output);
            repaint_ctx.request_repaint();
        });
    }

    fn receive_pending(&mut self) {
        if let Some(receiver) = &self.pending {
            if let Ok(output) = receiver.try_recv() {
                if let Some(results) = output.results {
                    self.results = results;
                }
                if let Some(created_levels) = output.created_levels {
                    self.created_levels = created_levels;
                }
                self.show_created_levels = output.show_created_levels;
                self.searching = false;
                self.pending = None;
            }
        }
    }
}

impl eframe::App for GdInfoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.receive_pending();

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
                            }
                        });
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
                                self.created_levels_page = 0;
                            }
                        }
                    });
                }

                ui.separator();
                ui.label("Results:");
                let result_height = if self.show_created_levels {
                    220.0
                } else {
                    320.0
                };
                ui.add_sized(
                    [ui.available_width(), result_height],
                    TextEdit::multiline(&mut self.results)
                        .desired_width(f32::INFINITY)
                        .font(egui::TextStyle::Monospace)
                        .lock_focus(true),
                );

                if self.show_created_levels {
                    ui.separator();
                    ui.label(format!(
                        "Created Levels (page {}):",
                        self.created_levels_page
                    ));

                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(
                                !self.searching && self.created_levels_page > 0,
                                egui::Button::new("Prev"),
                            )
                            .clicked()
                        {
                            self.created_levels_page -= 1;
                            self.start_created_levels_page_search(ctx);
                        }

                        ui.add(
                            egui::DragValue::new(&mut self.created_levels_page)
                                .speed(1)
                                .range(0..=999),
                        );

                        if ui
                            .add_enabled(!self.searching, egui::Button::new("Load Page"))
                            .clicked()
                        {
                            self.start_created_levels_page_search(ctx);
                        }

                        if ui
                            .add_enabled(!self.searching, egui::Button::new("Next"))
                            .clicked()
                        {
                            self.created_levels_page += 1;
                            self.start_created_levels_page_search(ctx);
                        }
                    });

                    ui.allocate_ui_with_layout(
                        egui::vec2(ui.available_width(), 260.0),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                if self.created_levels.is_empty() {
                                    ui.label("No created levels found on this page.");
                                }

                                for level in self.created_levels.clone() {
                                    let label = format!(
                                        "{} | ID {} | Downloads {} | Likes {} | {}",
                                        level.name,
                                        level.id,
                                        level.downloads,
                                        level.likes,
                                        level.difficulty
                                    );

                                    if ui.button(label).clicked() {
                                        self.query = level.id;
                                        self.search_type = SearchType::Level;
                                        self.comment_page = 0;
                                        self.start_search(ctx);
                                    }
                                }
                            });
                        },
                    );
                }
            });
        });
    }
}
