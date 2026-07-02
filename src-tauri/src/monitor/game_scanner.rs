use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstalledGame {
    pub name: String, pub exe_path: String, pub source: String,
}

/// Scan Steam library only
pub fn scan_installed_games() -> Vec<InstalledGame> {
    let mut games = Vec::new();

    #[cfg(target_os = "macos")]
    if let Ok(home) = std::env::var("HOME") {
        let steam = home + "/Library/Application Support/Steam/steamapps/common";
        if let Ok(entries) = std::fs::read_dir(&steam) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !name.starts_with('.') && !name.to_lowercase().contains("steam") {
                    games.push(InstalledGame {
                        name, exe_path: entry.path().to_string_lossy().to_string(), source: "steam".to_string(),
                    });
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    for base in ["C:\\Program Files (x86)\\Steam\\steamapps\\common", "D:\\Steam\\steamapps\\common"] {
        if let Ok(entries) = std::fs::read_dir(base) {
            for entry in entries.flatten() {
                let n = entry.file_name().to_string_lossy().to_string();
                if !n.starts_with('.') { games.push(InstalledGame { name: n, exe_path: entry.path().to_string_lossy().to_string(), source: "steam".to_string() }); }
            }
        }
    }

    // Always add Steam as a built-in entry
    games.insert(0, InstalledGame {
        name: "Steam".to_string(),
        exe_path: String::new(),
        source: "builtin".to_string(),
    });

    games.sort_by(|a, b| a.name.cmp(&b.name));
    games
}
