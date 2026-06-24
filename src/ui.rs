use std::{
    collections::HashMap,
    sync::mpsc::{self, Receiver},
};

use eframe::egui::{self, Color32, RichText, TextEdit};
use tokio::runtime::Runtime;

use crate::{
    api::BoomlingsApi,
    models::{
        AppData, CreatedLevel, LevelComment, LevelInfo, PlayerComment, PlayerInfo, PlayerProfile,
        SearchEntry, SearchResult, SearchType, ThemeMode, display,
    },
    storage,
};

struct SearchOutput {
    request_id: u64,
    entry: SearchEntry,
    result: Result<SearchResult, String>,
    keep_created_levels: bool,
}

struct CommentsOutput {
    request_id: u64,
    result: Result<Vec<LevelComment>, String>,
}

struct PlayerCommentsOutput {
    request_id: u64,
    result: Result<Vec<PlayerComment>, String>,
}

pub struct GdInfoApp {
    query: String,
    search_type: SearchType,
    data: AppData,
    result: Option<SearchResult>,
    status: String,
    created_levels: Vec<CreatedLevel>,
    comment_page: u32,
    created_levels_page: u32,
    created_filter: String,
    created_sort: CreatedSort,
    runtime: Option<Runtime>,
    api: Option<BoomlingsApi>,
    pending: Option<Receiver<SearchOutput>>,
    pending_comments: Option<Receiver<CommentsOutput>>,
    pending_player_comments: Option<Receiver<PlayerCommentsOutput>>,
    searching: bool,
    comments_loading: bool,
    player_comments_loading: bool,
    request_id: u64,
    comments_request_id: u64,
    player_comments_request_id: u64,
    cache: HashMap<SearchEntry, SearchResult>,
    show_settings: bool,
    show_player_comment_history: bool,
    player_comment_history_page: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CreatedSort {
    Default,
    Name,
    Downloads,
    Likes,
    Difficulty,
}

impl CreatedSort {
    fn label(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::Name => "Name",
            Self::Downloads => "Downloads",
            Self::Likes => "Likes",
            Self::Difficulty => "Difficulty",
        }
    }
}

impl GdInfoApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_pixels_per_point(1.0);
        let data = storage::load_data();
        apply_theme(&cc.egui_ctx, data.settings.theme);

        let (runtime, api, status) = match (
            Runtime::new(),
            BoomlingsApi::with_timeout_secs(data.settings.request_timeout_secs),
        ) {
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
            data,
            result: None,
            status,
            created_levels: Vec::new(),
            comment_page: 0,
            created_levels_page: 0,
            created_filter: String::new(),
            created_sort: CreatedSort::Default,
            runtime,
            api,
            pending: None,
            pending_comments: None,
            pending_player_comments: None,
            searching: false,
            comments_loading: false,
            player_comments_loading: false,
            request_id: 0,
            comments_request_id: 0,
            player_comments_request_id: 0,
            cache: HashMap::new(),
            show_settings: false,
            show_player_comment_history: false,
            player_comment_history_page: 0,
        }
    }

    fn start_search(&mut self, ctx: &egui::Context) {
        self.start_search_with_options(ctx, false, true);
    }

    fn start_created_levels_page_search(&mut self, ctx: &egui::Context) {
        self.start_search_with_options(ctx, true, false);
    }

    fn start_comment_page_search(&mut self, ctx: &egui::Context) {
        let Some(SearchResult::Level(level)) = &self.result else {
            self.status = "Load a level before changing comment pages.".to_owned();
            return;
        };
        let level_id = level.id.clone();
        if level_id.trim().is_empty() {
            self.status = "Current level has no ID for comment lookup.".to_owned();
            return;
        }

        let Some(runtime) = &self.runtime else {
            self.status = "Comment lookup unavailable: async runtime failed to start.".to_owned();
            return;
        };
        let Some(api) = self.api.clone() else {
            self.status = "Comment lookup unavailable: HTTP client failed to start.".to_owned();
            return;
        };

        self.comments_request_id += 1;
        let request_id = self.comments_request_id;
        let page = self.comment_page;
        let (sender, receiver) = mpsc::channel();
        let repaint_ctx = ctx.clone();

        self.pending_comments = Some(receiver);
        self.comments_loading = true;
        self.status = format!("Loading comments page {page}...");

        runtime.spawn(async move {
            let result = api
                .level_comments(&level_id, page)
                .await
                .map_err(|error| error.to_string());

            let _ = sender.send(CommentsOutput { request_id, result });
            repaint_ctx.request_repaint();
        });
    }

    fn start_player_comment_history_search(&mut self, ctx: &egui::Context) {
        let Some(SearchResult::Player(profile)) = &self.result else {
            self.status = "Load a player before opening comment history.".to_owned();
            return;
        };
        let account_id = profile.player.account_id.clone();
        if account_id.trim().is_empty() {
            self.status = "Current player has no account ID for comment history lookup.".to_owned();
            return;
        }

        let Some(runtime) = &self.runtime else {
            self.status = "Comment history unavailable: async runtime failed to start.".to_owned();
            return;
        };
        let Some(api) = self.api.clone() else {
            self.status = "Comment history unavailable: HTTP client failed to start.".to_owned();
            return;
        };

        self.player_comments_request_id += 1;
        let request_id = self.player_comments_request_id;
        let page = self.player_comment_history_page;
        let (sender, receiver) = mpsc::channel();
        let repaint_ctx = ctx.clone();

        self.pending_player_comments = Some(receiver);
        self.player_comments_loading = true;
        self.status = format!("Loading comment history page {page}...");

        runtime.spawn(async move {
            let result = api
                .account_comments(&account_id, page)
                .await
                .map_err(|error| error.to_string());

            let _ = sender.send(PlayerCommentsOutput { request_id, result });
            repaint_ctx.request_repaint();
        });
    }

    fn start_search_with_options(
        &mut self,
        ctx: &egui::Context,
        keep_created_levels: bool,
        use_cache: bool,
    ) {
        let query = self.query.trim().to_owned();
        if query.is_empty() {
            self.status = "Enter a search term.".to_owned();
            return;
        }

        let entry = SearchEntry {
            query: query.clone(),
            search_type: self.search_type,
        };
        if use_cache && self.comment_page == 0 && self.created_levels_page == 0 {
            if let Some(result) = self.cache.get(&entry).cloned() {
                self.apply_result(result);
                self.status = "Loaded from local cache.".to_owned();
                self.remember(entry);
                return;
            }
        }

        self.remember(entry.clone());

        let Some(runtime) = &self.runtime else {
            self.status = "Search unavailable: async runtime failed to start.".to_owned();
            return;
        };
        let Some(api) = self.api.clone() else {
            self.status = "Search unavailable: HTTP client failed to start.".to_owned();
            return;
        };

        self.request_id += 1;
        let request_id = self.request_id;
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
        let repaint_ctx = ctx.clone();

        self.pending = Some(receiver);
        self.searching = true;
        self.status = "Searching Boomlings...".to_owned();
        if !keep_created_levels {
            self.result = None;
            self.created_levels.clear();
        }
        runtime.spawn(async move {
            let result = match search_type {
                SearchType::Player => api
                    .search_player(&query, created_levels_page)
                    .await
                    .map(SearchResult::Player)
                    .map_err(|error| error.to_string()),
                SearchType::Level => api
                    .search_level(&query, comment_page)
                    .await
                    .map(SearchResult::Level)
                    .map_err(|error| error.to_string()),
            };

            let _ = sender.send(SearchOutput {
                request_id,
                entry,
                result,
                keep_created_levels,
            });
            repaint_ctx.request_repaint();
        });
    }

    fn receive_pending(&mut self) {
        if let Some(receiver) = &self.pending {
            if let Ok(output) = receiver.try_recv() {
                if output.request_id == self.request_id {
                    match output.result {
                        Ok(result) => {
                            if !output.keep_created_levels
                                && self.comment_page == 0
                                && self.created_levels_page == 0
                            {
                                self.cache.insert(output.entry, result.clone());
                            }
                            self.apply_result(result);
                            self.status = "Loaded.".to_owned();
                        }
                        Err(error) => self.status = format!("Error: {error}"),
                    }
                    self.searching = false;
                    self.pending = None;
                }
            }
        }
    }

    fn receive_pending_comments(&mut self) {
        if let Some(receiver) = &self.pending_comments {
            if let Ok(output) = receiver.try_recv() {
                if output.request_id == self.comments_request_id {
                    if let Some(SearchResult::Level(level)) = &mut self.result {
                        match output.result {
                            Ok(comments) => {
                                level.comments = comments;
                                level.comments_error = None;
                                self.status =
                                    format!("Loaded comments page {}.", self.comment_page);
                            }
                            Err(error) => {
                                level.comments.clear();
                                level.comments_error = Some(error.clone());
                                self.status = format!("Error: {error}");
                            }
                        }
                    }
                    self.comments_loading = false;
                    self.pending_comments = None;
                }
            }
        }
    }

    fn receive_pending_player_comments(&mut self) {
        if let Some(receiver) = &self.pending_player_comments {
            if let Ok(output) = receiver.try_recv() {
                if output.request_id == self.player_comments_request_id {
                    if let Some(SearchResult::Player(profile)) = &mut self.result {
                        match output.result {
                            Ok(comments) => {
                                profile.comment_history = comments;
                                profile.comment_history_error = None;
                                self.status = format!(
                                    "Loaded comment history page {}.",
                                    self.player_comment_history_page
                                );
                            }
                            Err(error) => {
                                profile.comment_history.clear();
                                profile.comment_history_error = Some(error.clone());
                                self.status = format!("Error: {error}");
                            }
                        }
                    }
                    self.player_comments_loading = false;
                    self.pending_player_comments = None;
                }
            }
        }
    }

    fn apply_result(&mut self, result: SearchResult) {
        if let SearchResult::Player(profile) = &result {
            self.created_levels = profile.created_levels.clone();
        } else {
            self.created_levels.clear();
        }
        self.show_player_comment_history = false;
        self.player_comment_history_page = 0;
        self.player_comments_loading = false;
        self.pending_player_comments = None;
        self.result = Some(result);
    }

    fn remember(&mut self, entry: SearchEntry) {
        storage::remember_search(
            &mut self.data.history,
            entry,
            self.data.settings.history_limit,
        );
        storage::save_data(&self.data);
    }

    fn current_entry(&self) -> Option<SearchEntry> {
        let query = self.query.trim();
        (!query.is_empty()).then(|| SearchEntry {
            query: query.to_owned(),
            search_type: self.search_type,
        })
    }

    fn is_favorite(&self, entry: &SearchEntry) -> bool {
        self.data.favorites.iter().any(|item| item == entry)
    }

    fn toggle_current_favorite(&mut self) {
        if let Some(entry) = self.current_entry() {
            storage::toggle_favorite(&mut self.data.favorites, entry);
            storage::save_data(&self.data);
        }
    }
}

impl eframe::App for GdInfoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.receive_pending();
        self.receive_pending_comments();
        self.receive_pending_player_comments();

        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("GD Info");
                ui.label(
                    RichText::new("native Boomlings inspector")
                        .color(ui.visuals().weak_text_color()),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Settings").clicked() {
                        self.show_settings = !self.show_settings;
                    }
                    if ui.button("Clear").clicked() {
                        self.result = None;
                        self.status.clear();
                    }
                });
            });
        });

        egui::SidePanel::left("library")
            .resizable(true)
            .default_width(190.0)
            .show(ctx, |ui| {
                ui.heading("Library");
                ui.separator();
                self.render_entry_list(ui, ctx, "Favorites", self.data.favorites.clone(), true);
                ui.add_space(10.0);
                self.render_entry_list(
                    ui,
                    ctx,
                    "Recent searches",
                    self.data.history.clone(),
                    false,
                );
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
            self.render_search_bar(ui, ctx);

            if self.show_settings {
                self.render_settings(ui, ctx);
            }

            ui.separator();
            if !self.status.is_empty() {
                ui.label(
                    RichText::new(&self.status).color(if self.status.starts_with("Error") {
                        Color32::from_rgb(180, 60, 60)
                    } else {
                        ui.visuals().weak_text_color()
                    }),
                );
            }

            egui::ScrollArea::vertical().show(ui, |ui| match self.result.clone() {
                Some(SearchResult::Player(profile)) => self.render_player(ui, ctx, &profile),
                Some(SearchResult::Level(level)) => self.render_level(ui, ctx, &level),
                None => self.render_empty(ui),
            });
        });
    }
}

impl GdInfoApp {
    fn render_search_bar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.horizontal(|ui| {
            let response = ui.add_sized(
                [ui.available_width() - 260.0, 30.0],
                TextEdit::singleline(&mut self.query)
                    .hint_text("Username, level name, or level ID"),
            );
            if response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                self.start_search(ctx);
            }
            egui::ComboBox::from_id_salt("search_type")
                .selected_text(self.search_type.label())
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.search_type, SearchType::Player, "Player");
                    ui.selectable_value(&mut self.search_type, SearchType::Level, "Level");
                });
            if ui
                .add_enabled(!self.searching, egui::Button::new("Search"))
                .clicked()
            {
                self.start_search(ctx);
            }
            if ui
                .button(
                    if self
                        .current_entry()
                        .as_ref()
                        .is_some_and(|entry| self.is_favorite(entry))
                    {
                        "Unfavorite"
                    } else {
                        "Favorite"
                    },
                )
                .clicked()
            {
                self.toggle_current_favorite();
            }
        });
    }

    fn render_settings(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.heading("Settings");
            let mut changed = false;
            ui.horizontal(|ui| {
                ui.label("Theme");
                egui::ComboBox::from_id_salt("theme")
                    .selected_text(self.data.settings.theme.label())
                    .show_ui(ui, |ui| {
                        changed |= ui
                            .selectable_value(
                                &mut self.data.settings.theme,
                                ThemeMode::System,
                                "System",
                            )
                            .changed();
                        changed |= ui
                            .selectable_value(
                                &mut self.data.settings.theme,
                                ThemeMode::Light,
                                "Light",
                            )
                            .changed();
                        changed |= ui
                            .selectable_value(
                                &mut self.data.settings.theme,
                                ThemeMode::Dark,
                                "Dark",
                            )
                            .changed();
                    });
                ui.label("History limit");
                changed |= ui
                    .add(egui::DragValue::new(&mut self.data.settings.history_limit).range(1..=100))
                    .changed();
                ui.label("Font size");
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut self.data.settings.result_font_size)
                            .range(11.0..=20.0),
                    )
                    .changed();
                ui.label("Timeout");
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut self.data.settings.request_timeout_secs)
                            .range(1..=60)
                            .suffix("s"),
                    )
                    .changed();
            });
            ui.horizontal(|ui| {
                if ui.button("Clear history").clicked() {
                    self.data.history.clear();
                    changed = true;
                }
                if ui.button("Clear favorites").clicked() {
                    self.data.favorites.clear();
                    changed = true;
                }
                if ui.button("Clear cache").clicked() {
                    self.cache.clear();
                    self.status = "Cache cleared.".to_owned();
                }
            });
            if changed {
                self.data.history.truncate(self.data.settings.history_limit);
                apply_theme(ctx, self.data.settings.theme);
                self.api =
                    BoomlingsApi::with_timeout_secs(self.data.settings.request_timeout_secs).ok();
                storage::save_data(&self.data);
            }
        });
    }

    fn render_entry_list(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        title: &str,
        entries: Vec<SearchEntry>,
        removable: bool,
    ) {
        ui.label(RichText::new(title).strong());
        if entries.is_empty() {
            ui.label(RichText::new("None yet").color(ui.visuals().weak_text_color()));
            return;
        }
        for entry in entries {
            ui.horizontal(|ui| {
                if ui.small_button(entry.label()).clicked() {
                    self.query = entry.query.clone();
                    self.search_type = entry.search_type;
                    self.comment_page = 0;
                    self.created_levels_page = 0;
                    self.start_search(ctx);
                }
                if removable && ui.small_button("x").clicked() {
                    self.data.favorites.retain(|item| item != &entry);
                    storage::save_data(&self.data);
                }
            });
        }
    }

    fn render_empty(&self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(70.0);
            ui.heading("Search a player or level");
            ui.label("Results now appear as native sections instead of terminal text.");
        });
    }

    fn render_player(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, profile: &PlayerProfile) {
        section(ui, "Player profile", |ui| {
            ui.horizontal(|ui| {
                ui.heading(display(&profile.player.username));
                copy_button(ui, ctx, "Copy all", profile.to_result_text());
                copy_button(
                    ui,
                    ctx,
                    "Copy account ID",
                    profile.player.account_id.clone(),
                );
                copy_button(ui, ctx, "Copy user ID", profile.player.user_id.clone());
            });
            field_grid(
                ui,
                &[
                    ("Stars", &profile.player.stars),
                    ("Diamonds", &profile.player.diamonds),
                    ("Moons", &profile.player.moons),
                    ("Demons", &profile.player.demons),
                    ("Creator points", &profile.player.creator_points),
                    ("Global rank", &profile.player.global_rank),
                    ("Mod status", &profile.player.mod_status),
                ],
            );
        });
        section(ui, "Icons and colors", |ui| {
            render_player_icons(ui, &profile.player)
        });
        section(ui, "Privacy and links", |ui| {
            field_grid(
                ui,
                &[
                    ("Messages", &profile.player.message_privacy),
                    ("Friend requests", &profile.player.friend_privacy),
                    ("YouTube", &profile.player.youtube),
                    ("Twitter/X", &profile.player.twitter),
                    ("Twitch", &profile.player.twitch),
                ],
            );
            ui.horizontal(|ui| {
                ui.label(RichText::new("Comment history").color(ui.visuals().weak_text_color()));
                ui.label(
                    RichText::new(display(&profile.player.comment_history_privacy)).monospace(),
                );
                if profile.player.comment_history_privacy == "Visible" {
                    let label = if self.show_player_comment_history {
                        "Hide"
                    } else {
                        "Show"
                    };
                    if ui
                        .add_enabled(!self.player_comments_loading, egui::Button::new(label))
                        .clicked()
                    {
                        if self.show_player_comment_history {
                            self.show_player_comment_history = false;
                        } else {
                            self.show_player_comment_history = true;
                            self.player_comment_history_page = 0;
                            self.start_player_comment_history_search(ctx);
                        }
                    }
                }
            });
        });
        if self.show_player_comment_history {
            self.render_player_comment_history(ui, ctx, profile);
        }
        self.render_created_levels(ui, ctx);
    }

    fn render_player_comment_history(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        profile: &PlayerProfile,
    ) {
        section(ui, "Comment history", |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("Page {}", self.player_comment_history_page));
                if ui
                    .add_enabled(
                        !self.searching
                            && !self.player_comments_loading
                            && self.player_comment_history_page > 0,
                        egui::Button::new("Prev"),
                    )
                    .clicked()
                {
                    self.player_comment_history_page -= 1;
                    self.start_player_comment_history_search(ctx);
                }
                ui.add(
                    egui::DragValue::new(&mut self.player_comment_history_page)
                        .speed(1)
                        .range(0..=999),
                );
                if ui
                    .add_enabled(
                        !self.searching && !self.player_comments_loading,
                        egui::Button::new("Load page"),
                    )
                    .clicked()
                {
                    self.start_player_comment_history_search(ctx);
                }
                if ui
                    .add_enabled(
                        !self.searching && !self.player_comments_loading,
                        egui::Button::new("Next"),
                    )
                    .clicked()
                {
                    self.player_comment_history_page += 1;
                    self.start_player_comment_history_search(ctx);
                }
                if self.player_comments_loading {
                    ui.label(
                        RichText::new("Loading comment history...")
                            .color(ui.visuals().weak_text_color()),
                    );
                }
            });

            if let Some(error) = &profile.comment_history_error {
                ui.label(format!("Could not load comment history: {error}"));
                return;
            }
            if profile.comment_history.is_empty() {
                ui.label("No comment history found.");
                return;
            }
            for comment in &profile.comment_history {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(format!("{} likes", display(&comment.likes)));
                        ui.label(display(&comment.age));
                    });
                    ui.label(&comment.text);
                });
            }
        });
    }

    fn render_level(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, level: &LevelInfo) {
        section(ui, "Level", |ui| {
            ui.horizontal(|ui| {
                ui.heading(display(&level.name));
                copy_button(ui, ctx, "Copy all", level.to_result_text());
                copy_button(ui, ctx, "Copy ID", level.id.clone());
                copy_button(ui, ctx, "Copy name", level.name.clone());
            });
            field_grid(
                ui,
                &[
                    ("ID", &level.id),
                    ("Creator", &level.creator),
                    ("Difficulty", &level.difficulty),
                    ("Rate", &level.rate_status),
                    ("Stars", &level.stars),
                    ("Downloads", &level.downloads),
                    ("Likes", &level.likes),
                    ("Length", &level.length),
                ],
            );
            ui.horizontal(|ui| {
                if ui.button("Open creator").clicked() {
                    self.query = level.creator.clone();
                    self.search_type = SearchType::Player;
                    self.start_search(ctx);
                }
                if !level.original_id.trim().is_empty()
                    && level.original_id != "0"
                    && ui.button("Open original").clicked()
                {
                    self.query = level.original_id.clone();
                    self.search_type = SearchType::Level;
                    self.comment_page = 0;
                    self.start_search(ctx);
                }
            });
        });
        section(ui, "Build and song", |ui| {
            field_grid(
                ui,
                &[
                    ("Coins", &level.coins),
                    ("Verified coins", &level.verified_coins),
                    ("Objects", &level.object_count),
                    ("Version", &level.version),
                    ("Game version", &level.game_version),
                    ("Copy state", &level.password),
                    ("Copy password", &level.copy_password),
                    ("Original ID", &level.original_id),
                    ("Two player", &level.two_player),
                    ("Song ID", &level.song_id),
                    ("Song", &level.song_name),
                    ("Artist", &level.song_artist),
                    ("Size", &level.song_size),
                ],
            )
        });
        if !level.description.trim().is_empty() {
            section(ui, "Description", |ui| {
                ui.label(&level.description);
            });
        }
        self.render_comments(ui, ctx, level);
    }

    fn render_created_levels(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        section(ui, "Created levels", |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("Page {}", self.created_levels_page));
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
                ui.add(egui::DragValue::new(&mut self.created_levels_page).range(0..=999));
                if ui
                    .add_enabled(!self.searching, egui::Button::new("Load page"))
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
                ui.separator();
                ui.label("Filter");
                ui.text_edit_singleline(&mut self.created_filter);
                egui::ComboBox::from_id_salt("created_sort")
                    .selected_text(self.created_sort.label())
                    .show_ui(ui, |ui| {
                        for sort in [
                            CreatedSort::Default,
                            CreatedSort::Name,
                            CreatedSort::Downloads,
                            CreatedSort::Likes,
                            CreatedSort::Difficulty,
                        ] {
                            ui.selectable_value(&mut self.created_sort, sort, sort.label());
                        }
                    });
            });
            let mut levels = self.created_levels.clone();
            if !self.created_filter.trim().is_empty() {
                let needle = self.created_filter.to_lowercase();
                levels.retain(|level| {
                    level.name.to_lowercase().contains(&needle) || level.id.contains(&needle)
                });
            }
            match self.created_sort {
                CreatedSort::Name => levels.sort_by(|a, b| a.name.cmp(&b.name)),
                CreatedSort::Downloads => levels.sort_by_key(|level| {
                    std::cmp::Reverse(level.downloads.parse::<u64>().unwrap_or(0))
                }),
                CreatedSort::Likes => levels.sort_by_key(|level| {
                    std::cmp::Reverse(level.likes.parse::<i64>().unwrap_or(0))
                }),
                CreatedSort::Difficulty => levels.sort_by(|a, b| a.difficulty.cmp(&b.difficulty)),
                CreatedSort::Default => {}
            }
            if levels.is_empty() {
                ui.label("No created levels found.");
            }
            for level in levels {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(display(&level.name)).strong());
                    ui.label(format!("ID {}", display(&level.id)));
                    ui.label(format!("{} downloads", display(&level.downloads)));
                    ui.label(format!("{} likes", display(&level.likes)));
                    ui.label(display(&level.difficulty));
                    ui.label(display(&level.length));
                    if ui.small_button("Open").clicked() {
                        self.query = level.id.clone();
                        self.search_type = SearchType::Level;
                        self.comment_page = 0;
                        self.start_search(ctx);
                    }
                    copy_button(ui, ctx, "Copy ID", level.id.clone());
                });
            }
        });
    }

    fn render_comments(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, level: &LevelInfo) {
        section(ui, "Comments", |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("Page {}", self.comment_page));
                if ui
                    .add_enabled(
                        !self.searching && !self.comments_loading && self.comment_page > 0,
                        egui::Button::new("Prev"),
                    )
                    .clicked()
                {
                    self.comment_page -= 1;
                    self.start_comment_page_search(ctx);
                }
                ui.add(
                    egui::DragValue::new(&mut self.comment_page)
                        .speed(1)
                        .range(0..=999),
                );
                if ui
                    .add_enabled(
                        !self.searching && !self.comments_loading,
                        egui::Button::new("Load page"),
                    )
                    .clicked()
                {
                    self.start_comment_page_search(ctx);
                }
                if ui
                    .add_enabled(
                        !self.searching && !self.comments_loading,
                        egui::Button::new("Next"),
                    )
                    .clicked()
                {
                    self.comment_page += 1;
                    self.start_comment_page_search(ctx);
                }
                if self.comments_loading {
                    ui.label(
                        RichText::new("Loading comments...").color(ui.visuals().weak_text_color()),
                    );
                }
            });

            if let Some(error) = &level.comments_error {
                ui.label(format!("Could not load comments: {error}"));
                return;
            }
            if level.comments.is_empty() {
                ui.label("No comments found.");
                return;
            }
            for comment in &level.comments {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        if ui.button(display(&comment.username)).clicked() {
                            self.query = comment.username.clone();
                            self.search_type = SearchType::Player;
                            self.start_search(ctx);
                        }
                        ui.label(format!("{} likes", display(&comment.likes)));
                        if !comment.percent.trim().is_empty() {
                            ui.label(format!("{}%", comment.percent));
                        }
                        ui.label(display(&comment.age));
                    });
                    ui.label(&comment.text);
                });
            }
        });
    }
}

fn render_player_icons(ui: &mut egui::Ui, player: &PlayerInfo) {
    field_grid(
        ui,
        &[
            ("Cube", &player.cube_icon),
            ("Ship", &player.ship_icon),
            ("Ball", &player.ball_icon),
            ("UFO", &player.ufo_icon),
            ("Wave", &player.wave_icon),
            ("Robot", &player.robot_icon),
            ("Spider", &player.spider_icon),
            ("Swing", &player.swing_icon),
            ("Primary color", &player.primary_color),
            ("Secondary color", &player.secondary_color),
            ("Glow", &player.glow),
        ],
    );
}

fn field_grid(ui: &mut egui::Ui, fields: &[(&str, &str)]) {
    egui::Grid::new(ui.next_auto_id())
        .num_columns(4)
        .spacing([18.0, 8.0])
        .show(ui, |ui| {
            for (index, (label, value)) in fields.iter().enumerate() {
                ui.label(RichText::new(*label).color(ui.visuals().weak_text_color()));
                ui.label(RichText::new(display(value)).monospace());
                if index % 2 == 1 {
                    ui.end_row();
                }
            }
        });
}

fn section(ui: &mut egui::Ui, title: &str, contents: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::group(ui.style())
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.label(RichText::new(title).strong().size(15.0));
            ui.add_space(4.0);
            contents(ui);
        });
    ui.add_space(8.0);
}

fn copy_button(ui: &mut egui::Ui, ctx: &egui::Context, label: &str, value: String) {
    if ui.small_button(label).clicked() {
        ctx.copy_text(value);
    }
}

fn apply_theme(ctx: &egui::Context, theme: ThemeMode) {
    match theme {
        ThemeMode::System => ctx.set_visuals(egui::Visuals::default()),
        ThemeMode::Light => ctx.set_visuals(egui::Visuals::light()),
        ThemeMode::Dark => ctx.set_visuals(egui::Visuals::dark()),
    }
}
