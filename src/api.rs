use std::{collections::HashMap, time::Duration};

use base64::{
    Engine as _,
    engine::general_purpose::{STANDARD, URL_SAFE, URL_SAFE_NO_PAD},
};
use reqwest::Client;

use crate::models::{
    CreatedLevel, LevelComment, LevelInfo, PlayerComment, PlayerInfo, PlayerProfile,
};

const BASE_URL: &str = "https://www.boomlings.com/database";
const SECRET: &str = "Wmfd2893gb7";

#[derive(Clone)]
pub struct BoomlingsApi {
    client: Client,
}

impl BoomlingsApi {
    pub fn with_timeout_secs(timeout_secs: u64) -> Result<Self, ApiError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs.max(1)))
            .user_agent("")
            .build()
            .map_err(ApiError::Http)?;

        Ok(Self { client })
    }

    pub async fn search_player(
        &self,
        username: &str,
        created_levels_page: u32,
    ) -> Result<PlayerProfile, ApiError> {
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
            let created_levels = self
                .created_levels(&player.user_id, created_levels_page)
                .await?;
            return Ok(PlayerProfile {
                player,
                created_levels,
                comment_history: Vec::new(),
                comment_history_error: None,
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
            let created_levels = self
                .created_levels(&player.user_id, created_levels_page)
                .await?;
            return Ok(PlayerProfile {
                player,
                created_levels,
                comment_history: Vec::new(),
                comment_history_error: None,
            });
        }

        let player = player_from_values(&parse_pairs(&details));
        let created_levels = self
            .created_levels(&player.user_id, created_levels_page)
            .await?;

        Ok(PlayerProfile {
            player,
            created_levels,
            comment_history: Vec::new(),
            comment_history_error: None,
        })
    }

    pub async fn account_comments(
        &self,
        account_id: &str,
        comment_page: u32,
    ) -> Result<Vec<PlayerComment>, ApiError> {
        if account_id.trim().is_empty() {
            return Ok(Vec::new());
        }

        let page = comment_page.to_string();
        let response = self
            .post(
                "getGJAccountComments20.php",
                &[
                    ("accountID", account_id),
                    ("page", &page),
                    ("total", "0"),
                    ("secret", SECRET),
                ],
            )
            .await?;

        if response == "-1" || response.trim().is_empty() {
            return Ok(Vec::new());
        }

        Ok(parse_account_comments_response(&response))
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

        if let Ok(Some(copy_info)) = self.level_copy_info(&level.id).await {
            level.password = copy_info.state;
            level.copy_password = copy_info.password;
        }

        match self.level_comments(&level.id, comment_page).await {
            Ok(comments) => level.comments = comments,
            Err(error) => level.comments_error = Some(error.to_string()),
        }

        Ok(level)
    }

    async fn created_levels(
        &self,
        user_id: &str,
        created_levels_page: u32,
    ) -> Result<Vec<CreatedLevel>, ApiError> {
        if user_id.trim().is_empty() {
            return Ok(Vec::new());
        }

        let page = created_levels_page.to_string();
        let response = self
            .post(
                "getGJLevels21.php",
                &[
                    ("str", user_id),
                    ("type", "5"),
                    ("page", &page),
                    ("count", "10"),
                    ("secret", SECRET),
                ],
            )
            .await?;

        if response == "-1" || response.trim().is_empty() {
            return Ok(Vec::new());
        }

        Ok(parse_created_levels_response(&response))
    }

    pub async fn level_comments(
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

    async fn level_copy_info(&self, level_id: &str) -> Result<Option<CopyInfo>, ApiError> {
        if level_id.trim().is_empty() {
            return Ok(None);
        }

        let response = self
            .post(
                "downloadGJLevel22.php",
                &[("levelID", level_id), ("secret", SECRET)],
            )
            .await?;

        if response == "-1" || response.trim().is_empty() {
            return Ok(None);
        }

        Ok(parse_level_copy_info(&response))
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
        cube_icon: value(values, "21"),
        ship_icon: value(values, "22"),
        ball_icon: value(values, "23"),
        ufo_icon: value(values, "24"),
        wave_icon: value(values, "25"),
        robot_icon: value(values, "26"),
        spider_icon: value(values, "43"),
        swing_icon: value(values, "53"),
        primary_color: value(values, "10"),
        secondary_color: value(values, "11"),
        glow: enabled_name(&value(values, "28")).to_owned(),
        message_privacy: message_privacy_name(&value(values, "18")).to_owned(),
        friend_privacy: friend_privacy_name(&value(values, "19")).to_owned(),
        comment_history_privacy: comment_history_privacy_name(&value(values, "50")).to_owned(),
        youtube: value(values, "20"),
        twitter: value(values, "44"),
        twitch: value(values, "45"),
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
    let song = if !custom_song_id.is_empty() && custom_song_id != "0" {
        parse_song(songs, &custom_song_id).unwrap_or_else(|| SongInfo {
            id: custom_song_id.clone(),
            name: format!("Custom song {custom_song_id}"),
            artist: String::new(),
            size: String::new(),
        })
    } else {
        SongInfo {
            id: official_song_id.clone(),
            name: official_song_name(&official_song_id).to_owned(),
            artist: "RobTop".to_owned(),
            size: String::new(),
        }
    };

    Some(LevelInfo {
        name: value(&level_values, "2"),
        id: value(&level_values, "1"),
        creator,
        creator_id,
        difficulty: difficulty_name(&level_values),
        rate_status: rate_status_name(&level_values),
        downloads: value(&level_values, "10"),
        likes: value(&level_values, "14"),
        length: length_name(&value(&level_values, "15")).to_owned(),
        stars: value(&level_values, "18"),
        coins: value(&level_values, "37"),
        verified_coins: enabled_name(&value(&level_values, "38")).to_owned(),
        object_count: value(&level_values, "45"),
        version: value(&level_values, "5"),
        game_version: game_version_name(&value(&level_values, "13")).to_owned(),
        password: copy_info(&value(&level_values, "27")).state,
        copy_password: copy_info(&value(&level_values, "27")).password,
        original_id: value(&level_values, "30"),
        two_player: enabled_name(&value(&level_values, "31")).to_owned(),
        song_id: song.id,
        song_name: song.name,
        song_artist: song.artist,
        song_size: song.size,
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
                stars: value(&values, "18"),
                length: length_name(&value(&values, "15")).to_owned(),
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

fn parse_account_comments_response(response: &str) -> Vec<PlayerComment> {
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

            let values = parse_tilde_pairs(item);
            Some(PlayerComment {
                text: decode_base64(&value(&values, "2")),
                likes: value(&values, "4"),
                age: value(&values, "9"),
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

#[derive(Clone, Debug)]
struct SongInfo {
    id: String,
    name: String,
    artist: String,
    size: String,
}

#[derive(Clone, Debug)]
struct CopyInfo {
    state: String,
    password: String,
}

fn parse_song(songs: &str, song_id: &str) -> Option<SongInfo> {
    songs.split("~:~").find_map(|song| {
        let normalized = song.replace("~|~", ":");
        let values = parse_pairs(&normalized);
        (value(&values, "1") == song_id).then(|| SongInfo {
            id: value(&values, "1"),
            name: value(&values, "2"),
            artist: value(&values, "4"),
            size: song_size_name(&value(&values, "5")),
        })
    })
}

fn parse_level_copy_info(response: &str) -> Option<CopyInfo> {
    let values = parse_pairs(response.split('#').next()?);
    Some(copy_info(&value(&values, "27")))
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

fn enabled_name(value: &str) -> &'static str {
    match value {
        "1" => "Yes",
        "0" => "No",
        _ => "N/A",
    }
}

fn message_privacy_name(value: &str) -> &'static str {
    match value {
        "0" => "Open",
        "1" => "Friends only",
        "2" => "Closed",
        _ => "N/A",
    }
}

fn friend_privacy_name(value: &str) -> &'static str {
    match value {
        "0" => "Open",
        "1" => "Closed",
        _ => "N/A",
    }
}

fn comment_history_privacy_name(value: &str) -> &'static str {
    match value {
        "0" => "Visible",
        "1" => "Friends only",
        "2" => "Hidden",
        _ => "N/A",
    }
}

fn game_version_name(value: &str) -> String {
    if value.trim().is_empty() {
        return String::new();
    }

    if value.len() >= 2 {
        let (major, minor) = value.split_at(value.len() - 1);
        format!("{major}.{minor}")
    } else {
        value.to_owned()
    }
}

fn copy_info(value: &str) -> CopyInfo {
    match value.trim() {
        "" | "0" => CopyInfo {
            state: "Not copyable".to_owned(),
            password: String::new(),
        },
        "1" => CopyInfo {
            state: "Free copy".to_owned(),
            password: String::new(),
        },
        value => match decode_level_password(value).as_deref() {
            Some("0") => CopyInfo {
                state: "Not copyable".to_owned(),
                password: String::new(),
            },
            Some("1") => CopyInfo {
                state: "Free copy".to_owned(),
                password: String::new(),
            },
            Some(decoded) => CopyInfo {
                state: "Password protected".to_owned(),
                password: decoded.strip_prefix('1').unwrap_or(decoded).to_owned(),
            },
            None => CopyInfo {
                state: "Password protected".to_owned(),
                password: String::new(),
            },
        },
    }
}

fn decode_level_password(value: &str) -> Option<String> {
    let bytes = URL_SAFE
        .decode(value)
        .or_else(|_| URL_SAFE_NO_PAD.decode(value))
        .or_else(|_| STANDARD.decode(value))
        .ok()?;
    let key = b"26364";
    let decoded = bytes
        .iter()
        .enumerate()
        .map(|(index, byte)| byte ^ key[index % key.len()])
        .collect::<Vec<_>>();
    let decoded = String::from_utf8(decoded).ok()?;

    decoded
        .chars()
        .all(|ch| ch.is_ascii_digit())
        .then_some(decoded)
}

fn song_size_name(value: &str) -> String {
    if value.trim().is_empty() {
        String::new()
    } else if value.ends_with("MB") {
        value.to_owned()
    } else {
        format!("{value} MB")
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
        "0" => "Stereo Madness",
        "1" => "Back On Track",
        "2" => "Polargeist",
        "3" => "Dry Out",
        "4" => "Base After Base",
        "5" => "Cant Let Go",
        "6" => "Jumper",
        "7" => "Time Machine",
        "8" => "Cycles",
        "9" => "xStep",
        "10" => "Clutterfunk",
        "11" => "Theory of Everything",
        "12" => "Electroman Adventures",
        "13" => "Clubstep",
        "14" => "Electrodynamix",
        "15" => "Hexagon Force",
        "16" => "Blast Processing",
        "17" => "Theory of Everything 2",
        "18" => "Geometrical Dominator",
        "19" => "Deadlocked",
        "20" => "Fingerdash",
        "21" => "Dash",
        _ => "N/A",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rich_level_fields() {
        let response = "1:123:2:Test Level:3:SGVsbG8=:5:7:6:42:9:50:10:1000:12:1:13:22:14:250:15:3:18:10:19:1:27:0:30:99:31:1:35:55:37:3:38:1:42:2:45:12345#42:Creator:9#1~|~55~|~2~|~Song Name~|~4~|~Artist~|~5~|~1.23MB";
        let level = parse_level_response(response).expect("level parses");

        assert_eq!(level.id, "123");
        assert_eq!(level.creator, "Creator");
        assert_eq!(level.difficulty, "Insane");
        assert_eq!(level.rate_status, "Rated 10 stars, Featured, Legendary");
        assert_eq!(level.length, "Long");
        assert_eq!(level.description, "Hello");
        assert_eq!(level.coins, "3");
        assert_eq!(level.verified_coins, "Yes");
        assert_eq!(level.object_count, "12345");
        assert_eq!(level.original_id, "99");
        assert_eq!(level.two_player, "Yes");
        assert_eq!(level.song_name, "Song Name");
        assert_eq!(level.song_artist, "Artist");
        assert_eq!(level.song_size, "1.23MB");
        assert_eq!(level.password, "Not copyable");
        assert_eq!(level.copy_password, "");
    }

    #[test]
    fn maps_official_song_ids_from_zero() {
        let response = "1:1:2:Official Song Level:6:42:12:0:15:0#42:Creator:9#";
        let level = parse_level_response(response).expect("level parses");

        assert_eq!(level.song_id, "0");
        assert_eq!(level.song_name, "Stereo Madness");
    }

    #[test]
    fn parses_copy_state_from_download_metadata() {
        let response = "1:1:2:Copyable Level:27:Aw==#hash#hash";
        let copy_info = parse_level_copy_info(response).expect("copy info parses");
        assert_eq!(copy_info.state, "Free copy");
        assert_eq!(copy_info.password, "");

        let response = "1:1:2:Protected Level:27:AwYDAgwKAQ==#hash#hash";
        let copy_info = parse_level_copy_info(response).expect("copy info parses");
        assert_eq!(copy_info.state, "Password protected");
        assert_eq!(copy_info.password, "004887");
    }

    #[test]
    fn parses_account_comment_history() {
        let response = "2~aSBiZWF0IHNvbmljIHdhdmU=~4~562182~9~5 years#1:0:10";
        let comments = parse_account_comments_response(response);

        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].text, "i beat sonic wave");
        assert_eq!(comments[0].likes, "562182");
        assert_eq!(comments[0].age, "5 years");
    }

    #[test]
    fn parses_player_icons_and_privacy() {
        let values = parse_pairs(
            "1:User:2:88:16:99:21:1:22:2:23:3:24:4:25:5:26:6:43:7:53:8:10:12:11:13:28:1:18:1:19:0:50:2:44:xuser:45:tuser",
        );
        let player = player_from_values(&values);

        assert_eq!(player.username, "User");
        assert_eq!(player.cube_icon, "1");
        assert_eq!(player.swing_icon, "8");
        assert_eq!(player.primary_color, "12");
        assert_eq!(player.glow, "Yes");
        assert_eq!(player.message_privacy, "Friends only");
        assert_eq!(player.friend_privacy, "Open");
        assert_eq!(player.comment_history_privacy, "Hidden");
    }
}
