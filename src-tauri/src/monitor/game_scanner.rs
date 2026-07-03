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
    {
        // Scan all drives A-Z for Steam library folders
        for drive in 'C'..='Z' {
            for pattern in [
                format!("{}:\\Program Files (x86)\\Steam\\steamapps", drive),
                format!("{}:\\Steam\\steamapps", drive),
                format!("{}:\\SteamLibrary\\steamapps", drive),
            ] {
                let common = format!("{}\\common", pattern);
                if let Ok(entries) = std::fs::read_dir(&common) {
                    for entry in entries.flatten() {
                        let n = entry.file_name().to_string_lossy().to_string();
                        if !n.starts_with('.') && !n.to_lowercase().contains("steamworks") {
                            games.push(InstalledGame { name: n, exe_path: entry.path().to_string_lossy().to_string(), source: "steam".to_string() });
                        }
                    }
                }
                // Also check libraryfolders.vdf for additional library paths
                let vdf = format!("{}\\libraryfolders.vdf", pattern);
                if let Ok(content) = std::fs::read_to_string(&vdf) {
                    for line in content.lines() {
                        if let Some(start) = line.find("\"path\"") {
                            let rest = &line[start+7..];
                            if let Some(path_start) = rest.find('"') {
                                let path = &rest[path_start+1..];
                                if let Some(path_end) = path.find('"') {
                                    let lib = format!("{}\\steamapps\\common", &path[..path_end].replace("\\\\", "\\"));
                                    if let Ok(entries) = std::fs::read_dir(&lib) {
                                        for entry in entries.flatten() {
                                            let n = entry.file_name().to_string_lossy().to_string();
                                            if !n.starts_with('.') && !n.to_lowercase().contains("steamworks") {
                                                games.push(InstalledGame { name: n, exe_path: entry.path().to_string_lossy().to_string(), source: "steam".to_string() });
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
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
