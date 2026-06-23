use std::{env, fs, path::PathBuf};

use crate::models::{AppData, SearchEntry};

pub fn load_data() -> AppData {
    let Some(path) = data_path() else {
        return AppData::default();
    };

    let Ok(contents) = fs::read_to_string(path) else {
        return AppData::default();
    };

    serde_json::from_str(&contents).unwrap_or_default()
}

pub fn save_data(data: &AppData) {
    let Some(path) = data_path() else {
        return;
    };

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    if let Ok(contents) = serde_json::to_string_pretty(data) {
        let _ = fs::write(path, contents);
    }
}

pub fn remember_search(history: &mut Vec<SearchEntry>, entry: SearchEntry, limit: usize) {
    history.retain(|item| item.query != entry.query || item.search_type != entry.search_type);
    history.insert(0, entry);
    history.truncate(limit.max(1));
}

pub fn toggle_favorite(favorites: &mut Vec<SearchEntry>, entry: SearchEntry) {
    if let Some(index) = favorites
        .iter()
        .position(|item| item.query == entry.query && item.search_type == entry.search_type)
    {
        favorites.remove(index);
    } else {
        favorites.insert(0, entry);
    }
}

fn data_path() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join("gd-info").join("app-data.json"))
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
