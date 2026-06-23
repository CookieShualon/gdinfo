use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchEntry {
    pub query: String,
    pub search_type: SearchType,
}

impl SearchEntry {
    pub fn label(&self) -> String {
        format!("{}: {}", self.search_type.label(), self.query)
    }
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
}

impl PlayerInfo {
    pub fn to_result_text(&self) -> String {
        format!(
            "Username: {}\nAccount ID: {}\nUser ID: {}\nStars: {}\nDiamonds: {}\nSecret Coins: {}\nUser Coins: {}\nMoons: {}\nDemons: {}\nCreator Points: {}\nGlobal Rank: {}\nMod Status: {}",
            display(&self.username),
            display(&self.account_id),
            display(&self.user_id),
            display(&self.stars),
            display(&self.diamonds),
            display(&self.secret_coins),
            display(&self.user_coins),
            display(&self.moons),
            display(&self.demons),
            display(&self.creator_points),
            display(&self.global_rank),
            display(&self.mod_status),
        )
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
                    "{} ({}) - Downloads: {}, Likes: {}, Difficulty: {}\n",
                    display(&level.name),
                    display(&level.id),
                    display(&level.downloads),
                    display(&level.likes),
                    display(&level.difficulty),
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
}

#[derive(Clone, Debug, Default)]
pub struct LevelInfo {
    pub name: String,
    pub id: String,
    pub creator: String,
    pub difficulty: String,
    pub rate_status: String,
    pub downloads: String,
    pub likes: String,
    pub length: String,
    pub song_name: String,
    pub description: String,
    pub comments: Vec<LevelComment>,
    pub comments_error: Option<String>,
}

impl LevelInfo {
    pub fn to_result_text(&self) -> String {
        let mut text = format!(
            "Level Name: {}\nLevel ID: {}\nCreator: {}\nDifficulty: {}\nRate Status: {}\nDownloads: {}\nLikes: {}\nLength: {}\nSong Name: {}",
            display(&self.name),
            display(&self.id),
            display(&self.creator),
            display(&self.difficulty),
            display(&self.rate_status),
            display(&self.downloads),
            display(&self.likes),
            display(&self.length),
            display(&self.song_name),
        );

        if !self.description.trim().is_empty() {
            text.push_str("\nDescription: ");
            text.push_str(self.description.trim());
        }

        text.push_str("\n\nComments:\n");
        if let Some(error) = &self.comments_error {
            text.push_str("Could not load comments: ");
            text.push_str(error);
        } else if self.comments.is_empty() {
            text.push_str("No comments found.");
        } else {
            for comment in &self.comments {
                let percent = if comment.percent.trim().is_empty() {
                    String::new()
                } else {
                    format!(" [{}%]", comment.percent)
                };
                text.push_str(&format!(
                    "{}{} - {} likes - {}\n{}\n\n",
                    display(&comment.username),
                    percent,
                    display(&comment.likes),
                    display(&comment.age),
                    display(&comment.text),
                ));
            }
        }

        text.trim_end().to_owned()
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

fn display(value: &str) -> &str {
    if value.trim().is_empty() {
        "N/A"
    } else {
        value
    }
}
