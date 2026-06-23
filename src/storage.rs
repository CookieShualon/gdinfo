use std::{env, fs, path::PathBuf};

use crate::models::SearchEntry;

const MAX_HISTORY: usize = 10;

pub fn load_history() -> Vec<SearchEntry> {
    let Some(path) = history_path() else {
        return Vec::new();
    };

    let Ok(contents) = fs::read_to_string(path) else {
        return Vec::new();
    };

    serde_json::from_str(&contents).unwrap_or_default()
}

pub fn remember_search(history: &mut Vec<SearchEntry>, entry: SearchEntry) {
    history.retain(|item| item.query != entry.query || item.search_type != entry.search_type);
    history.insert(0, entry);
    history.truncate(MAX_HISTORY);
    save_history(history);
}

fn save_history(history: &[SearchEntry]) {
    let Some(path) = history_path() else {
        return;
    };

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    if let Ok(contents) = serde_json::to_string_pretty(history) {
        let _ = fs::write(path, contents);
    }
}

fn history_path() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join("gd-info").join("history.json"))
}

fn config_dir() -> Option<PathBuf> {
    if cfg!(target_os = "windows") {
        env::var_os("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        env::var_os("HOME")
            .map(PathBuf::from)
            .map(|home| home.join("Library").join("Application Support"))
    } else if let Some(dir) = env::var_os("XDG_CONFIG_HOME") {
        Some(PathBuf::from(dir))
    } else {
        env::var_os("HOME")
            .map(PathBuf::from)
            .map(|home| home.join(".config"))
    }
}
