use std::{collections::HashMap, time::Duration};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use reqwest::Client;

use crate::models::{CreatedLevel, LevelComment, LevelInfo, PlayerInfo, PlayerProfile};

const BASE_URL: &str = "https://www.boomlings.com/database";
const SECRET: &str = "Wmfd2893gb7";

#[derive(Clone)]
pub struct BoomlingsApi {
    client: Client,
}

impl BoomlingsApi {
    pub fn new() -> Result<Self, ApiError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("")
            .build()
            .map_err(ApiError::Http)?;

        Ok(Self { client })
    }

    pub async fn search_player(&self, username: &str) -> Result<PlayerProfile, ApiError> {
        let username = username.trim();
        if username.is_empty() {
            return Err(ApiError::Input("Enter a username.".to_owned()));
        }

        let response = self
            .post("getGJUsers20.php", &[("str", username), ("secret", SECRET)])
            .await?;

        if response == "-1" || response.trim().is_empty() {
            return Err(ApiError::NotFound("No player found.".to_owned()));
        }

        let first_match = response.split('#').next().unwrap_or(&response);
        let search_values = parse_pairs(first_match);
        let account_id = value(&search_values, "16");

        if account_id.is_empty() {
            let player = player_from_values(&search_values);
            let created_levels = self.created_levels(&player.user_id).await?;
            return Ok(PlayerProfile {
                player,
                created_levels,
            });
        }

        let details = self
            .post(
                "getGJUserInfo20.php",
                &[("targetAccountID", &account_id), ("secret", SECRET)],
            )
            .await?;

        if details == "-1" || details.trim().is_empty() {
            let player = player_from_values(&search_values);
            let created_levels = self.created_levels(&player.user_id).await?;
            return Ok(PlayerProfile {
                player,
                created_levels,
            });
        }

        let player = player_from_values(&parse_pairs(&details));
        let created_levels = self.created_levels(&player.user_id).await?;

        Ok(PlayerProfile {
            player,
            created_levels,
        })
    }

    pub async fn search_level(
        &self,
        query: &str,
        comment_page: u32,
    ) -> Result<LevelInfo, ApiError> {
        let query = query.trim();
        if query.is_empty() {
            return Err(ApiError::Input("Enter a level name or ID.".to_owned()));
        }

        let response = self
            .post(
                "getGJLevels21.php",
                &[("str", query), ("type", "0"), ("secret", SECRET)],
            )
            .await?;

        let response = if response == "-1" && query.chars().all(|ch| ch.is_ascii_digit()) {
            self.post(
                "getGJLevels21.php",
                &[("str", query), ("type", "10"), ("secret", SECRET)],
            )
            .await?
        } else {
            response
        };

        if response == "-1" || response.trim().is_empty() {
            return Err(ApiError::NotFound("No level found.".to_owned()));
        }

        let mut level = parse_level_response(&response)
            .ok_or_else(|| ApiError::Parse("Could not read level data.".to_owned()))?;

        match self.level_comments(&level.id, comment_page).await {
            Ok(comments) => level.comments = comments,
            Err(error) => level.comments_error = Some(error.to_string()),
        }

        Ok(level)
    }

    async fn created_levels(&self, user_id: &str) -> Result<Vec<CreatedLevel>, ApiError> {
        if user_id.trim().is_empty() {
            return Ok(Vec::new());
        }

        let response = self
            .post(
                "getGJLevels21.php",
                &[("str", user_id), ("type", "5"), ("secret", SECRET)],
            )
            .await?;

        if response == "-1" || response.trim().is_empty() {
            return Ok(Vec::new());
        }

        Ok(parse_created_levels_response(&response))
    }

    async fn level_comments(
        &self,
        level_id: &str,
        comment_page: u32,
    ) -> Result<Vec<LevelComment>, ApiError> {
        if level_id.trim().is_empty() {
            return Ok(Vec::new());
        }

        let page = comment_page.to_string();
        let response = self
            .post(
                "getGJComments21.php",
                &[
                    ("levelID", level_id),
                    ("page", &page),
                    ("count", "10"),
                    ("mode", "0"),
                    ("secret", SECRET),
                ],
            )
            .await?;

        if response == "-1" || response.trim().is_empty() {
            return Ok(Vec::new());
        }

        Ok(parse_comments_response(&response))
    }

    async fn post(&self, endpoint: &str, fields: &[(&str, &str)]) -> Result<String, ApiError> {
        let response = self
            .client
            .post(format!("{BASE_URL}/{endpoint}"))
            .form(fields)
            .send()
            .await
            .map_err(ApiError::Http)?;

        if !response.status().is_success() {
            return Err(ApiError::Api(format!(
                "API returned HTTP {}.",
                response.status()
            )));
        }

        response.text().await.map_err(ApiError::Http)
    }
}

#[derive(Debug)]
pub enum ApiError {
    Input(String),
    NotFound(String),
    Api(String),
    Parse(String),
    Http(reqwest::Error),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Input(message)
            | Self::NotFound(message)
            | Self::Api(message)
            | Self::Parse(message) => {
                write!(f, "{message}")
            }
            Self::Http(error) if error.is_timeout() => write!(f, "Request timed out."),
            Self::Http(error) => write!(f, "Network error: {error}"),
        }
    }
}

impl std::error::Error for ApiError {}

fn player_from_values(values: &HashMap<String, String>) -> PlayerInfo {
    PlayerInfo {
        username: value(values, "1"),
        account_id: value(values, "16"),
        user_id: value(values, "2"),
        stars: value(values, "3"),
        diamonds: value(values, "46"),
        secret_coins: value(values, "13"),
        user_coins: value(values, "17"),
        moons: value(values, "52"),
        demons: value(values, "4"),
        creator_points: value(values, "8"),
        global_rank: value_or(values, "30", "6"),
        mod_status: mod_status_name(&value(values, "49")).to_owned(),
        icon: crate::models::PlayerIcon {
            cube_id: value_or(values, "21", "9"),
            primary_color: value(values, "10"),
            secondary_color: value(values, "11"),
            glow_enabled: value(values, "28") == "1" || value(values, "15") == "2",
        },
    }
}

fn parse_level_response(response: &str) -> Option<LevelInfo> {
    let mut parts = response.split('#');
    let levels = parts.next()?;
    let creators = parts.next().unwrap_or_default();
    let songs = parts.next().unwrap_or_default();
    let level_values = parse_pairs(levels.split('|').next()?);

    let creator_id = value(&level_values, "6");
    let creator = creators
        .split('|')
        .find_map(|item| {
            let fields: Vec<&str> = item.split(':').collect();
            (fields.first()? == &creator_id)
                .then(|| fields.get(1).copied().unwrap_or_default().to_owned())
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| creator_id.clone());

    let custom_song_id = value(&level_values, "35");
    let official_song_id = value(&level_values, "12");
    let song_name = if !custom_song_id.is_empty() && custom_song_id != "0" {
        parse_song_name(songs, &custom_song_id)
            .unwrap_or_else(|| format!("Custom song {custom_song_id}"))
    } else {
        official_song_name(&official_song_id).to_owned()
    };

    Some(LevelInfo {
        name: value(&level_values, "2"),
        id: value(&level_values, "1"),
        creator,
        difficulty: difficulty_name(&level_values),
        rate_status: rate_status_name(&level_values),
        downloads: value(&level_values, "10"),
        likes: value(&level_values, "14"),
        length: length_name(&value(&level_values, "15")).to_owned(),
        song_name,
        description: decode_base64(&value(&level_values, "3")),
        comments: Vec::new(),
        comments_error: None,
    })
}

fn parse_created_levels_response(response: &str) -> Vec<CreatedLevel> {
    response
        .split('#')
        .next()
        .unwrap_or_default()
        .split('|')
        .take(10)
        .filter_map(|level| {
            if level.trim().is_empty() {
                return None;
            }

            let values = parse_pairs(level);
            Some(CreatedLevel {
                name: value(&values, "2"),
                id: value(&values, "1"),
                downloads: value(&values, "10"),
                likes: value(&values, "14"),
                difficulty: difficulty_name(&values),
            })
        })
        .collect()
}

fn parse_comments_response(response: &str) -> Vec<LevelComment> {
    response
        .split('#')
        .next()
        .unwrap_or_default()
        .split('|')
        .take(10)
        .filter_map(|item| {
            if item.trim().is_empty() {
                return None;
            }

            let (comment_part, user_part) = item.split_once(':').unwrap_or((item, ""));
            let comment_values = parse_tilde_pairs(comment_part);
            let user_values = parse_tilde_pairs(user_part);

            Some(LevelComment {
                username: value(&user_values, "1"),
                text: decode_base64(&value(&comment_values, "2")),
                likes: value(&comment_values, "4"),
                age: value(&comment_values, "9"),
                percent: value(&comment_values, "10"),
            })
        })
        .collect()
}

fn parse_pairs(input: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let mut parts = input.split(':');

    while let Some(key) = parts.next() {
        if let Some(value) = parts.next() {
            map.insert(key.to_owned(), value.to_owned());
        }
    }

    map
}

fn parse_tilde_pairs(input: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let mut parts = input.split('~');

    while let Some(key) = parts.next() {
        if let Some(value) = parts.next() {
            map.insert(key.to_owned(), value.to_owned());
        }
    }

    map
}

fn parse_song_name(songs: &str, song_id: &str) -> Option<String> {
    songs.split(":~:").find_map(|song| {
        let normalized = song.replace("~|~", ":");
        let values = parse_pairs(&normalized);
        (value(&values, "1") == song_id).then(|| value(&values, "2"))
    })
}

fn value(values: &HashMap<String, String>, key: &str) -> String {
    values.get(key).cloned().unwrap_or_default()
}

fn value_or(values: &HashMap<String, String>, first: &str, second: &str) -> String {
    let first_value = value(values, first);
    if first_value.is_empty() {
        value(values, second)
    } else {
        first_value
    }
}

fn decode_base64(input: &str) -> String {
    STANDARD
        .decode(input)
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .unwrap_or_default()
}

fn difficulty_name(values: &HashMap<String, String>) -> String {
    if value(values, "25") == "1" {
        return "Auto".to_owned();
    }

    if value(values, "17") == "1" {
        return match value(values, "43").as_str() {
            "3" => "Easy Demon",
            "4" => "Medium Demon",
            "5" => "Insane Demon",
            "6" => "Extreme Demon",
            _ => "Hard Demon",
        }
        .to_owned();
    }

    match value(values, "9").as_str() {
        "10" => "Easy",
        "20" => "Normal",
        "30" => "Hard",
        "40" => "Harder",
        "50" => "Insane",
        _ => "N/A",
    }
    .to_owned()
}

fn mod_status_name(mod_status: &str) -> &str {
    match mod_status {
        "1" => "Moderator",
        "2" => "Elder Moderator",
        "3" => "Leaderboard Moderator",
        _ => "None",
    }
}

fn rate_status_name(values: &HashMap<String, String>) -> String {
    let stars = value(values, "18");
    let feature_score = value(values, "19");
    let epic = value(values, "42");
    let star_text = if stars.trim().is_empty() || stars == "0" {
        "Unrated".to_owned()
    } else {
        format!("Rated {stars} stars")
    };

    let feature_text = if feature_score.trim().is_empty() || feature_score == "0" {
        None
    } else {
        Some("Featured".to_owned())
    };

    let epic_text = match epic.as_str() {
        "1" => Some("Epic".to_owned()),
        "2" => Some("Legendary".to_owned()),
        "3" => Some("Mythic".to_owned()),
        _ => None,
    };

    [Some(star_text), feature_text, epic_text]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(", ")
}

fn length_name(length: &str) -> &str {
    match length {
        "0" => "Tiny",
        "1" => "Short",
        "2" => "Medium",
        "3" => "Long",
        "4" => "XL",
        "5" => "Platformer",
        _ => "N/A",
    }
}

fn official_song_name(song_id: &str) -> &str {
    match song_id {
        "1" => "Stereo Madness",
        "2" => "Back On Track",
        "3" => "Polargeist",
        "4" => "Dry Out",
        "5" => "Base After Base",
        "6" => "Cant Let Go",
        "7" => "Jumper",
        "8" => "Time Machine",
        "9" => "Cycles",
        "10" => "xStep",
        "11" => "Clutterfunk",
        "12" => "Theory of Everything",
        "13" => "Electroman Adventures",
        "14" => "Clubstep",
        "15" => "Electrodynamix",
        "16" => "Hexagon Force",
        "17" => "Blast Processing",
        "18" => "Theory of Everything 2",
        "19" => "Geometrical Dominator",
        "20" => "Deadlocked",
        "21" => "Fingerdash",
        "22" => "Dash",
        _ => "N/A",
    }
}
