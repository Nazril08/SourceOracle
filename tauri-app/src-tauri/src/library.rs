use std::fs;
use std::path::Path;
use serde::{Serialize, Deserialize};
use tauri::command;
use crate::APP_CACHE;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DirectoryStatus {
    lua: bool,
    manifest: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameInfo {
    pub app_id: String,
    pub name: String,
    pub lua_file: bool,
    pub manifest_file: bool,
}

/// Initializes the Steam app cache
#[command]
pub async fn initialize_app_cache() -> Result<bool, String> {
    if APP_CACHE.is_loaded() {
        println!("App cache already loaded");
        return Ok(true);
    }
    
    println!("Initializing Steam app cache...");
    match APP_CACHE.load_from_steam_api().await {
        Ok(_) => {
            println!("Successfully loaded Steam app cache");
            Ok(true)
        },
        Err(e) => {
            println!("Failed to load Steam app cache: {}", e);
            Err(e)
        }
    }
}

/// Gets a game name by its AppID
#[command]
pub async fn get_game_name_by_appid(app_id: String) -> Result<String, String> {
    if !APP_CACHE.is_loaded() {
        let _ = initialize_app_cache().await;
    }
    
    Ok(APP_CACHE.get_game_name(&app_id).unwrap_or_else(|| format!("AppID: {}", app_id)))
}

/// Checks if the required Steam directories exist
#[command]
pub async fn check_steam_directories(lua_path: String, manifest_path: String) -> Result<DirectoryStatus, String> {
    let lua_exists = Path::new(&lua_path).exists();
    let manifest_exists = Path::new(&manifest_path).exists();
    
    Ok(DirectoryStatus {
        lua: lua_exists,
        manifest: manifest_exists,
    })
}

/// Gets all games in the library by reading LUA and manifest files
#[command]
pub async fn get_library_games(lua_dir: String, manifest_dir: String) -> Result<Vec<GameInfo>, String> {
    // Check if directories exist
    let lua_path = Path::new(&lua_dir);
    let manifest_path = Path::new(&manifest_dir);
    
    if !lua_path.exists() {
        return Err(format!("Steam directory not found: {}", lua_dir));
    }
    
    if !manifest_path.exists() {
        return Err(format!("Steam directory not found: {}", manifest_dir));
    }
    
    // Make sure app cache is initialized
    if !APP_CACHE.is_loaded() {
        let _ = initialize_app_cache().await;
    }
    
    let mut games: Vec<GameInfo> = Vec::new();
    
    // Read LUA directory to find games
    match fs::read_dir(lua_path) {
        Ok(entries) => {
            for entry in entries {
                if let Ok(entry) = entry {
                    let file_name = entry.file_name();
                    let file_name_str = file_name.to_string_lossy();
                    
                    // Check if it's a LUA file
                    if file_name_str.ends_with(".lua") {
                        // Extract app_id from filename
                        let app_id = file_name_str
                            .trim_end_matches(".lua")
                            .to_string();
                        
                        // Check if manifest file exists
                        let manifest_file = manifest_path.join(format!("{}.manifest", app_id));
                        let manifest_exists = manifest_file.exists();
                        
                        // Look up the game name from the app cache
                        let name = APP_CACHE.get_game_name(&app_id).unwrap_or_else(|| format!("AppID: {}", app_id));
                        
                        games.push(GameInfo {
                            app_id,
                            name,
                            lua_file: true,
                            manifest_file: manifest_exists,
                        });
                    }
                }
            }
        },
        Err(e) => {
            return Err(format!("Failed to read directory: {}", e));
        }
    }
    
    // Sort games by name
    games.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    
    Ok(games)
}

/// Updates game files in the library (simplified version)
#[command]
pub async fn update_game(app_id: String) -> Result<(), String> {
    // Get Steam directories
    let (lua_dir, manifest_dir, bin_dir) = get_steam_directories();
    
    // In a real implementation, this would download and extract files
    // For now, just return a success message
    println!("Would update game with AppID: {} to directories:", app_id);
    println!("LUA dir: {}", lua_dir);
    println!("Manifest dir: {}", manifest_dir);
    println!("BIN dir: {}", bin_dir);
    
    Ok(())
}

/// Removes game files from the library
#[command]
pub async fn remove_game(app_id: String) -> Result<(), String> {
    // Get Steam directories
    let (lua_dir, manifest_dir, bin_dir) = get_steam_directories();
    
    // Delete LUA file
    let lua_file = Path::new(&lua_dir).join(format!("{}.lua", app_id));
    if lua_file.exists() {
        if let Err(e) = fs::remove_file(&lua_file) {
            return Err(format!("Failed to delete LUA file: {}", e));
        }
    }
    
    // Delete manifest file
    let manifest_file = Path::new(&manifest_dir).join(format!("{}.manifest", app_id));
    if manifest_file.exists() {
        if let Err(e) = fs::remove_file(&manifest_file) {
            return Err(format!("Failed to delete manifest file: {}", e));
        }
    }
    
    // Delete BIN file
    let bin_file = Path::new(&bin_dir).join(format!("{}.bin", app_id));
    if bin_file.exists() {
        if let Err(e) = fs::remove_file(&bin_file) {
            return Err(format!("Failed to delete BIN file: {}", e));
        }
    }
    
    Ok(())
}

// Helper function to get Steam directories (in a real app, you'd get this from a config)
pub fn get_steam_directories() -> (String, String, String) {
    (
        "C:\\Program Files (x86)\\Steam\\config\\stplug-in".to_string(),
        "C:\\Program Files (x86)\\Steam\\config\\depotcache".to_string(),
        "C:\\Program Files (x86)\\Steam\\config\\StatsExport".to_string(),
    )
} 