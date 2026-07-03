use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstalledGame {
    pub name: String, pub exe_path: String, pub source: String,
}

/// Chinese name mappings for popular Steam games (folder name -> display name)
fn chinese_name_map() -> HashMap<&'static str, &'static str> {
    HashMap::from([
        ("Counter-Strike Global Offensive", "CS:GO"),
        ("Counter-Strike 2", "CS2"),
        ("Apex Legends", "Apex 英雄"),
        ("PUBG: BATTLEGROUNDS", "绝地求生"),
        ("Dota 2", "Dota 2"),
        ("Grand Theft Auto V", "GTA5"),
        ("Rainbow Six Siege", "彩虹六号：围攻"),
        ("ELDEN RING", "艾尔登法环"),
        ("Black Myth: Wukong", "黑神话：悟空"),
        ("Cyberpunk 2077", "赛博朋克2077"),
        ("Red Dead Redemption 2", "荒野大镖客2"),
        ("The Witcher 3", "巫师3"),
        ("Monster Hunter World", "怪物猎人：世界"),
        ("Monster Hunter Rise", "怪物猎人：崛起"),
        ("War Thunder", "战争雷霆"),
        ("World of Tanks", "坦克世界"),
        ("World of Warships", "战舰世界"),
        ("Call of Duty", "使命召唤"),
        ("Battlefield 2042", "战地2042"),
        ("Destiny 2", "命运2"),
        ("Warframe", "星际战甲"),
        ("Path of Exile", "流放之路"),
        ("Diablo IV", "暗黑破坏神4"),
        ("Genshin Impact", "原神"),
        ("Honkai: Star Rail", "崩坏：星穹铁道"),
        ("Team Fortress 2", "军团要塞2"),
        ("Left 4 Dead 2", "求生之路2"),
        ("Garry's Mod", "GMod"),
        ("Rust", "Rust"),
        ("ARK: Survival Evolved", "方舟：生存进化"),
        ("Stardew Valley", "星露谷物语"),
        ("Terraria", "泰拉瑞亚"),
        ("Dead by Daylight", "黎明杀机"),
        ("Forza Horizon 5", "极限竞速：地平线5"),
        ("Sid Meier's Civilization VI", "文明6"),
        ("Fall Guys", "糖豆人"),
        ("Rocket League", "火箭联盟"),
        ("Euro Truck Simulator 2", "欧洲卡车模拟2"),
        ("Football Manager", "足球经理"),
        ("FIFA 23", "FIFA 23"),
        ("NBA 2K23", "NBA 2K23"),
        ("Overwatch 2", "守望先锋2"),
        ("Palworld", "幻兽帕鲁"),
        ("Lethal Company", "致命公司"),
        ("Baldur's Gate 3", "博德之门3"),
        ("Hogwarts Legacy", "霍格沃茨之遗"),
        ("Marvel Rivals", "漫威争锋"),
        ("Delta Force", "三角洲行动"),
    ])
}

/// Parse Steam ACF manifest files to get real game names
fn parse_acf_names(steamapps_dir: &std::path::Path) -> HashMap<String, String> {
    let mut names = HashMap::new();
    if let Ok(entries) = std::fs::read_dir(steamapps_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with("appmanifest_") || !name.ends_with(".acf") {
                continue;
            }
            if let Ok(content) = std::fs::read_to_string(entry.path()) {
                let mut app_name = String::new();
                let mut install_dir = String::new();
                for line in content.lines() {
                    let trimmed = line.trim();
                    if let Some(val) = extract_acf_value(trimmed, "name") {
                        app_name = val;
                    }
                    if let Some(val) = extract_acf_value(trimmed, "installdir") {
                        install_dir = val;
                    }
                }
                if !install_dir.is_empty() && !app_name.is_empty() {
                    names.insert(install_dir, app_name);
                }
            }
        }
    }
    names
}

fn extract_acf_value(line: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\"", key);
    if let Some(pos) = line.find(&pattern) {
        let rest = &line[pos + pattern.len()..].trim();
        if rest.starts_with('"') {
            let inner = &rest[1..];
            if let Some(end) = inner.find('"') {
                return Some(inner[..end].to_string());
            }
        }
    }
    None
}

/// Get display name: try Chinese map, then Steam ACF name, then cleaned folder name
fn display_name(folder_name: &str, acf_names: &HashMap<String, String>) -> String {
    // Check Chinese name map
    if let Some(cn) = chinese_name_map().get(folder_name) {
        return cn.to_string();
    }
    // Check ACF real name
    if let Some(real) = acf_names.get(folder_name) {
        // Also check Chinese map for the real name
        if let Some(cn) = chinese_name_map().get(real.as_str()) {
            return cn.to_string();
        }
        return real.clone();
    }
    // Clean up folder name
    let cleaned = folder_name
        .replace("TM", "")
        .replace("â„¢", "")
        .replace("®", "")
        .replace("™", "")
        .trim()
        .to_string();
    // Check Chinese map again after cleaning
    if let Some(cn) = chinese_name_map().get(cleaned.as_str()) {
        return cn.to_string();
    }
    cleaned
}

/// Scan Steam library with proper game names
pub fn scan_installed_games() -> Vec<InstalledGame> {
    let mut games = Vec::new();

    #[cfg(target_os = "macos")]
    if let Ok(home) = std::env::var("HOME") {
        let steamapps = std::path::PathBuf::from(&home)
            .join("Library/Application Support/Steam/steamapps");
        let acf_names = parse_acf_names(&steamapps);
        let common = steamapps.join("common");
        if let Ok(entries) = std::fs::read_dir(&common) {
            for entry in entries.flatten() {
                let folder = entry.file_name().to_string_lossy().to_string();
                if folder.starts_with('.') || folder.to_lowercase().contains("steamworks") {
                    continue;
                }
                games.push(InstalledGame {
                    name: display_name(&folder, &acf_names),
                    exe_path: entry.path().to_string_lossy().to_string(),
                    source: "steam".to_string(),
                });
            }
        }
    }

    #[cfg(target_os = "windows")]
    for drive in 'C'..='Z' {
        for pattern in [
            format!("{}:\\Program Files (x86)\\Steam\\steamapps", drive),
            format!("{}:\\Steam\\steamapps", drive),
            format!("{}:\\SteamLibrary\\steamapps", drive),
        ] {
            let steamapps = std::path::PathBuf::from(&pattern);
            if !steamapps.exists() { continue; }
            let acf_names = parse_acf_names(&steamapps);
            let common = steamapps.join("common");
            if let Ok(entries) = std::fs::read_dir(&common) {
                for entry in entries.flatten() {
                    let folder = entry.file_name().to_string_lossy().to_string();
                    if folder.starts_with('.') || folder.to_lowercase().contains("steamworks") {
                        continue;
                    }
                    games.push(InstalledGame {
                        name: display_name(&folder, &acf_names),
                        exe_path: entry.path().to_string_lossy().to_string(),
                        source: "steam".to_string(),
                    });
                }
            }
            // Parse libraryfolders.vdf for additional libraries
            let vdf = steamapps.join("libraryfolders.vdf");
            if let Ok(content) = std::fs::read_to_string(&vdf) {
                for line in content.lines() {
                    if let Some(start) = line.find("\"path\"") {
                        let rest = &line[start + 7..];
                        if let Some(p1) = rest.find('"') {
                            let path = &rest[p1 + 1..];
                            if let Some(p2) = path.find('"') {
                                let lib = format!("{}\\steamapps", &path[..p2].replace("\\\\", "\\"));
                                let lib_path = std::path::PathBuf::from(&lib);
                                let lib_acf = parse_acf_names(&lib_path);
                                let lib_common = lib_path.join("common");
                                if let Ok(entries) = std::fs::read_dir(&lib_common) {
                                    for entry in entries.flatten() {
                                        let folder = entry.file_name().to_string_lossy().to_string();
                                        if folder.starts_with('.') { continue; }
                                        games.push(InstalledGame {
                                            name: display_name(&folder, &lib_acf),
                                            exe_path: entry.path().to_string_lossy().to_string(),
                                            source: "steam".to_string(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Built-in entries
    games.insert(0, InstalledGame {
        name: "Steam 商店".to_string(),
        exe_path: String::new(),
        source: "builtin".to_string(),
    });

    games.sort_by(|a, b| a.name.cmp(&b.name));
    games.dedup_by(|a, b| a.name == b.name);
    games
}
