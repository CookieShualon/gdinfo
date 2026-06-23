use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum SearchType {
    Player,
    Level,
}

impl SearchType {
    pub fn label(self) -> &'static str {
        match self {
            Self::Player => "Player",
            Self::Level => "Level",
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct SearchEntry {
    pub query: String,
    pub search_type: SearchType,
}

impl SearchEntry {
    pub fn label(&self) -> String {
        format!("{}: {}", self.search_type.label(), self.query)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ThemeMode {
    System,
    Light,
    Dark,
}

impl ThemeMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Light => "Light",
            Self::Dark => "Dark",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppSettings {
    pub theme: ThemeMode,
    pub history_limit: usize,
    pub result_font_size: f32,
    pub request_timeout_secs: u64,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: ThemeMode::System,
            history_limit: 25,
            result_font_size: 13.0,
            request_timeout_secs: 10,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AppData {
    pub history: Vec<SearchEntry>,
    pub favorites: Vec<SearchEntry>,
    pub settings: AppSettings,
}

#[derive(Clone, Debug, Default)]
pub struct PlayerInfo {
    pub username: String,
    pub account_id: String,
    pub user_id: String,
    pub stars: String,
    pub diamonds: String,
    pub secret_coins: String,
    pub user_coins: String,
    pub moons: String,
    pub demons: String,
    pub creator_points: String,
    pub global_rank: String,
    pub mod_status: String,
    pub cube_icon: String,
    pub ship_icon: String,
    pub ball_icon: String,
    pub ufo_icon: String,
    pub wave_icon: String,
    pub robot_icon: String,
    pub spider_icon: String,
    pub swing_icon: String,
    pub primary_color: String,
    pub secondary_color: String,
    pub glow: String,
    pub message_privacy: String,
    pub friend_privacy: String,
    pub comment_history_privacy: String,
    pub youtube: String,
    pub twitter: String,
    pub twitch: String,
}

impl PlayerInfo {
    pub fn to_result_text(&self) -> String {
        let mut lines = Vec::new();
        push_field(&mut lines, "Username", &self.username);
        push_field(&mut lines, "Account ID", &self.account_id);
        push_field(&mut lines, "User ID", &self.user_id);
        push_field(&mut lines, "Stars", &self.stars);
        push_field(&mut lines, "Diamonds", &self.diamonds);
        push_field(&mut lines, "Secret Coins", &self.secret_coins);
        push_field(&mut lines, "User Coins", &self.user_coins);
        push_field(&mut lines, "Moons", &self.moons);
        push_field(&mut lines, "Demons", &self.demons);
        push_field(&mut lines, "Creator Points", &self.creator_points);
        push_field(&mut lines, "Global Rank", &self.global_rank);
        push_field(&mut lines, "Mod Status", &self.mod_status);
        push_field(&mut lines, "Cube", &self.cube_icon);
        push_field(&mut lines, "Ship", &self.ship_icon);
        push_field(&mut lines, "Ball", &self.ball_icon);
        push_field(&mut lines, "UFO", &self.ufo_icon);
        push_field(&mut lines, "Wave", &self.wave_icon);
        push_field(&mut lines, "Robot", &self.robot_icon);
        push_field(&mut lines, "Spider", &self.spider_icon);
        push_field(&mut lines, "Swing", &self.swing_icon);
        push_field(&mut lines, "Primary Color", &self.primary_color);
        push_field(&mut lines, "Secondary Color", &self.secondary_color);
        push_field(&mut lines, "Glow", &self.glow);
        push_field(&mut lines, "Messages", &self.message_privacy);
        push_field(&mut lines, "Friend Requests", &self.friend_privacy);
        push_field(&mut lines, "Comment History", &self.comment_history_privacy);
        push_field(&mut lines, "YouTube", &self.youtube);
        push_field(&mut lines, "Twitter/X", &self.twitter);
        push_field(&mut lines, "Twitch", &self.twitch);
        lines.join("\n")
    }
}

#[derive(Clone, Debug, Default)]
pub struct PlayerProfile {
    pub player: PlayerInfo,
    pub created_levels: Vec<CreatedLevel>,
}

impl PlayerProfile {
    pub fn to_result_text(&self) -> String {
        let mut text = self.player.to_result_text();
        text.push_str("\n\nCreated Levels:\n");

        if self.created_levels.is_empty() {
            text.push_str("No created levels found.");
        } else {
            for level in &self.created_levels {
                text.push_str(&format!(
                    "{} ({}) - Downloads: {}, Likes: {}, Difficulty: {}, Stars: {}\n",
                    display(&level.name),
                    display(&level.id),
                    display(&level.downloads),
                    display(&level.likes),
                    display(&level.difficulty),
                    display(&level.stars),
                ));
            }
        }

        text.trim_end().to_owned()
    }
}

#[derive(Clone, Debug, Default)]
pub struct CreatedLevel {
    pub name: String,
    pub id: String,
    pub downloads: String,
    pub likes: String,
    pub difficulty: String,
    pub stars: String,
    pub length: String,
}

#[derive(Clone, Debug, Default)]
pub struct LevelInfo {
    pub name: String,
    pub id: String,
    pub creator: String,
    pub creator_id: String,
    pub difficulty: String,
    pub rate_status: String,
    pub downloads: String,
    pub likes: String,
    pub length: String,
    pub stars: String,
    pub coins: String,
    pub verified_coins: String,
    pub object_count: String,
    pub version: String,
    pub game_version: String,
    pub password: String,
    pub original_id: String,
    pub two_player: String,
    pub song_id: String,
    pub song_name: String,
    pub song_artist: String,
    pub song_size: String,
    pub description: String,
    pub comments: Vec<LevelComment>,
    pub comments_error: Option<String>,
}

impl LevelInfo {
    pub fn to_result_text(&self) -> String {
        let mut lines = Vec::new();
        push_field(&mut lines, "Level Name", &self.name);
        push_field(&mut lines, "Level ID", &self.id);
        push_field(&mut lines, "Creator", &self.creator);
        push_field(&mut lines, "Creator ID", &self.creator_id);
        push_field(&mut lines, "Difficulty", &self.difficulty);
        push_field(&mut lines, "Rate Status", &self.rate_status);
        push_field(&mut lines, "Stars", &self.stars);
        push_field(&mut lines, "Downloads", &self.downloads);
        push_field(&mut lines, "Likes", &self.likes);
        push_field(&mut lines, "Length", &self.length);
        push_field(&mut lines, "Coins", &self.coins);
        push_field(&mut lines, "Verified Coins", &self.verified_coins);
        push_field(&mut lines, "Object Count", &self.object_count);
        push_field(&mut lines, "Version", &self.version);
        push_field(&mut lines, "Game Version", &self.game_version);
        push_field(&mut lines, "Password", &self.password);
        push_field(&mut lines, "Original ID", &self.original_id);
        push_field(&mut lines, "Two Player", &self.two_player);
        push_field(&mut lines, "Song ID", &self.song_id);
        push_field(&mut lines, "Song Name", &self.song_name);
        push_field(&mut lines, "Song Artist", &self.song_artist);
        push_field(&mut lines, "Song Size", &self.song_size);

        if !self.description.trim().is_empty() {
            push_field(&mut lines, "Description", self.description.trim());
        }

        lines.push(String::new());
        lines.push("Comments:".to_owned());
        if let Some(error) = &self.comments_error {
            lines.push(format!("Could not load comments: {error}"));
        } else if self.comments.is_empty() {
            lines.push("No comments found.".to_owned());
        } else {
            for comment in &self.comments {
                let percent = if comment.percent.trim().is_empty() {
                    String::new()
                } else {
                    format!(" [{}%]", comment.percent)
                };
                lines.push(format!(
                    "{}{} - {} likes - {}\n{}",
                    display(&comment.username),
                    percent,
                    display(&comment.likes),
                    display(&comment.age),
                    display(&comment.text),
                ));
            }
        }

        lines.join("\n").trim_end().to_owned()
    }
}

#[derive(Clone, Debug, Default)]
pub struct LevelComment {
    pub username: String,
    pub text: String,
    pub likes: String,
    pub age: String,
    pub percent: String,
}

#[derive(Clone, Debug)]
pub enum SearchResult {
    Player(PlayerProfile),
    Level(LevelInfo),
}

pub fn display(value: &str) -> &str {
    if value.trim().is_empty() {
        "N/A"
    } else {
        value
    }
}

fn push_field(lines: &mut Vec<String>, label: &str, value: &str) {
    lines.push(format!("{label}: {}", display(value)));
}
