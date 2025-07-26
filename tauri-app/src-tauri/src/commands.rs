use crate::models::{GameInfo, DownloadResult, SteamAppDetailsResponse, RepoType, GameDatabase, SearchResults, SteamAppInfo};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::io::{Write, Read};
use std::process::Command;
use std::time::{Duration, SystemTime};
use tauri::{command, AppHandle};
use anyhow::Result;
use reqwest::Client;
use uuid::Uuid;
use walkdir::WalkDir;
use zip::ZipArchive;
use serde::{Serialize, Deserialize};
use serde_json;
use dirs_next;
use regex::Regex;

// Lazy static untuk database game
lazy_static::lazy_static! {
    static ref GAME_DATABASE: GameDatabase = GameDatabase::new();
}

// Command to initialize the game database
#[command]
pub async fn initialize_database() -> Result<bool, String> {
    if GAME_DATABASE.is_loaded() {
        return Ok(true); // Already loaded
    }
    
    match GAME_DATABASE.load_or_refresh_db().await {
        Ok(_) => Ok(true),
        Err(e) => Err(format!("Failed to load game database: {}", e)),
    }
}

// Command to search games by name or AppID
#[command]
pub async fn search_games(query: String, page: usize, per_page: usize) -> Result<SearchResults, String> {
    println!("Searching for '{}' on page {} with {} items per page", query, page, per_page);
    
    // Ensure database is loaded
    if !GAME_DATABASE.is_loaded() {
        match GAME_DATABASE.load_or_refresh_db().await {
            Ok(_) => {
                println!("Database loaded successfully");
            },
            Err(e) => {
                println!("Failed to load database: {}", e);
                return Err(format!("Failed to load game database: {}", e));
            }
        }
    }
    
    // Perform search
    let results = GAME_DATABASE.search(&query, page, per_page);
    
    Ok(results)
}

// Command to search games by name (multiple terms separated by comma)
#[command]
pub async fn search_game_by_name(query: String, page: usize, per_page: usize) -> Result<SearchResults, String> {
    // Ensure database is loaded
    if !GAME_DATABASE.is_loaded() {
        match GAME_DATABASE.load_or_refresh_db().await {
            Ok(_) => println!("Database loaded successfully"),
            Err(e) => return Err(format!("Failed to load game database: {}", e))
        }
    }
    
    // Perform search directly without logging
    Ok(GAME_DATABASE.search(&query, page, per_page))
}

// Command to get game details by AppID
#[command]
pub async fn get_game_details(app_id: String) -> Result<SteamAppInfo, String> {
    // First, try to load from cache
    if let Ok(details) = load_details_from_cache(&app_id) {
        println!("Loaded details for AppID {} from cache.", app_id);
        return Ok(details);
    }

    println!("Fetching game details for AppID: {}", app_id);
    
    // If not in cache, fetch from Steam API
    let client = Client::new();
    let url = format!("https://store.steampowered.com/api/appdetails?appids={}", app_id);
    
    match client.get(&url)
        .timeout(Duration::from_secs(10))
        .send()
        .await {
        Ok(response) => {
            if !response.status().is_success() {
                println!("Steam API returned non-success status: {}", response.status());
                return Err(format!("Steam API returned status {}", response.status()));
            }
            
            match response.json::<SteamAppDetailsResponse>().await {
                Ok(app_details) => {
                    if let Some(app_data) = app_details.apps.get(&app_id) {
                        if app_data.success {
                            if let Some(data) = &app_data.data {
                            println!("Successfully fetched details for {}: {}", app_id, data.name);
                                // Save to cache
                                if let Err(e) = save_details_to_cache(&app_id, data) {
                                    eprintln!("Failed to save details to cache for AppID {}: {}", app_id, e);
                                }
                                return Ok(data.clone());
                        }
                    }
                    }
                    let msg = format!("Steam API returned success=false or no data for AppID {}", app_id);
                    println!("{}", msg);
                    Err(msg)
                },
                Err(e) => {
                    let msg = format!("Failed to parse Steam API response: {}", e);
                    println!("{}", msg);
                    Err(msg)
                }
            }
        },
        Err(e) => {
            let msg = format!("Error fetching from Steam API: {}", e);
            println!("{}", msg);
            Err(msg)
        }
    }
}

// Command to clear the app details cache
#[command]
pub async fn clear_details_cache() -> Result<(), String> {
    match get_details_cache_dir() {
        Ok(path) => {
            if path.exists() {
                println!("Clearing details cache directory: {}", path.display());
                fs::remove_dir_all(&path).map_err(|e| format!("Failed to clear cache: {}", e))?;
            }
            Ok(())
        }
        Err(e) => Err(format!("Could not get cache directory: {}", e)),
    }
}

// Helper to get the cache path for a specific app detail
fn get_details_cache_path(app_id: &str) -> Result<PathBuf> {
    let mut path = get_details_cache_dir()?;
    fs::create_dir_all(&path)?;
    path.push(format!("{}.json", app_id));
    Ok(path)
}

// Helper to get the base directory for the details cache
fn get_details_cache_dir() -> Result<PathBuf> {
    let mut path = dirs_next::data_dir().ok_or_else(|| anyhow::anyhow!("Failed to get data directory"))?;
    path.push("Oracle/cache/details");
    Ok(path)
}

// Helper to load details from cache
fn load_details_from_cache(app_id: &str) -> Result<SteamAppInfo> {
    let path = get_details_cache_path(app_id)?;
    if !path.exists() {
        return Err(anyhow::anyhow!("Cache file not found."));
    }
    
    // Check cache age (TTL: 24 hours)
    if let Ok(metadata) = fs::metadata(&path) {
        if let Ok(modified_time) = metadata.modified() {
            if let Ok(age) = SystemTime::now().duration_since(modified_time) {
                // If cache is older than 24 hours, treat it as expired.
                if age > Duration::from_secs(24 * 60 * 60) {
                    println!("Cache for AppID {} is stale. Re-fetching.", app_id);
                    return Err(anyhow::anyhow!("Cache expired"));
                }
            }
        }
    }

    let mut file = File::open(&path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    
    let details: SteamAppInfo = serde_json::from_str(&contents)?;
    Ok(details)
}

// Helper to save details to cache
fn save_details_to_cache(app_id: &str, details: &SteamAppInfo) -> Result<()> {
    let path = get_details_cache_path(app_id)?;
    let json_details = serde_json::to_string_pretty(details)?;
    fs::write(path, json_details)?;
    Ok(())
}

// Command to fetch game name from Steam API
#[command]
pub async fn fetch_game_name(app_id: String) -> Result<GameInfo, String> {
    // Directly use get_game_details and map the result to the expected type
    match get_game_details(app_id).await {
        Ok(details) => Ok(GameInfo {
            app_id: details.steam_appid.to_string(),
            game_name: details.name,
            icon_url: Some(details.header_image),
        }),
        Err(e) => Err(e),
    }
}

// Command to download game files
#[command]
pub async fn download_game(app_id: String, game_name: String, output_dir: Option<String>) -> DownloadResult {
    // Get saved settings to use the saved directory
    let settings = load_settings_sync()?;
    
    // Use the provided output_dir if available, otherwise use the one from settings
    let actual_output_dir = match output_dir {
        Some(dir) if !dir.is_empty() => dir,
        _ => settings.download_directory
    };
    
    // Create output directory if it doesn't exist
    fs::create_dir_all(&actual_output_dir).map_err(|e| e.to_string())?;
    
    // Setup repositories to try
    let mut repos = HashMap::new();
    repos.insert("Fairyvmos/bruh-hub".to_string(), RepoType::Branch);
    repos.insert("SteamAutoCracks/ManifestHub".to_string(), RepoType::Branch);
    repos.insert("ManifestHub/ManifestHub".to_string(), RepoType::Decrypted);
    
    // Implementasi download yang sebenarnya
    let sanitized_game_name = sanitize_filename::sanitize(&game_name);
    let client = reqwest::Client::builder()
        .user_agent("oracle-downloader/1.0")
        .build()
        .map_err(|e| e.to_string())?;
    
    for (repo_full_name, repo_type) in &repos {
        println!("\n--- Trying Repository: {} (Type: {:?}) ---", repo_full_name, repo_type);
        
        if *repo_type == RepoType::Branch {
            // Try to download the entire branch as a ZIP file
            let api_url = format!("https://api.github.com/repos/{}/zipball/{}", repo_full_name, app_id);
            println!("Trying to download branch zip from: {}", api_url);
            
            match client.get(&api_url)
                .timeout(Duration::from_secs(600))
                .send()
                .await {
                    Ok(response) => {
                        if response.status().is_success() {
                            println!("Successfully downloaded zip content for branch {}", app_id);
                            let bytes = response.bytes().await.map_err(|e| e.to_string())?;
                            
                            let zip_path = Path::new(&actual_output_dir)
                                .join(format!("{} - {} (Branch).zip", sanitized_game_name, app_id));
                            
                            let mut file = File::create(&zip_path).map_err(|e| e.to_string())?;
                            file.write_all(&bytes).map_err(|e| e.to_string())?;
                            
                            println!("SUCCESS! Branch repo saved to: {}", zip_path.display());
                            
                            // Process the downloaded ZIP file
                            process_downloaded_zip(&zip_path).map_err(|e| e.to_string())?;
                            
                            return Ok(true); // Stop after successfully finding from one repo
                        } else {
                            println!("Failed to download branch zip. Status: {}", response.status());
                        }
                    },
                    Err(e) => {
                        println!("Error when downloading branch zip: {}", e);
                    }
                }
        } else {
            // Implementasi untuk non-branch repos bisa ditambahkan di sini jika diperlukan
            println!("Non-branch repo type not implemented yet");
        }
    }
    
    println!("\n[FINISHED] Failed to find data for AppID {} from all selected repositories.", app_id);
    Ok(false)
}

// Synchronous version of load_settings to use in download_game
fn load_settings_sync() -> Result<AppSettings, String> {
    let settings_dir = get_settings_dir()?;
    let settings_file = settings_dir.join("settings.json");
    
    // Check if settings file exists
    if !settings_file.exists() {
        // Return default settings
        return Ok(AppSettings {
            download_directory: "downloads".to_string(),
        });
    }
    
    // Read file content
    let mut file = File::open(&settings_file)
        .map_err(|e| format!("Failed to open settings file: {}", e))?;
    
    let mut content = String::new();
    file.read_to_string(&mut content)
        .map_err(|e| format!("Failed to read settings file: {}", e))?;
    
    // Deserialize settings
    let settings: AppSettings = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse settings file: {}", e))?;
    
    Ok(settings)
}

// Command to restart Steam
#[command]
pub async fn restart_steam() -> Result<(), String> {
    // On Windows
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        // First, terminate the Steam process
        Command::new("taskkill")
            .args(&["/F", "/IM", "steam.exe"])
            .output()
            .map_err(|e| format!("Failed to terminate Steam: {}", e))?;
        
        // Find Steam installation path from registry
        if let Ok(steam_path) = find_steam_executable_path() {
            // Relaunch Steam
            Command::new(steam_path)
                .spawn()
                .map_err(|e| format!("Failed to restart Steam: {}", e))?;
        }
    }
    
    // On macOS / Linux (basic restart command)
    #[cfg(not(target_os = "windows"))]
    {
        Command::new("steam")
            .arg("--restart")
            .spawn()
            .map_err(|e| format!("Failed to restart Steam: {}", e))?;
    }
    
    println!("Steam restarted successfully.");
    Ok(())
}

#[tauri::command]
pub async fn install_steam_tools(app_handle: AppHandle) -> Result<(), String> {
    let resource_path = app_handle.path_resolver()
        .resolve_resource("../../st-setup-1.8.16.exe")
        .ok_or_else(|| "Failed to resolve resource path.".to_string())?;

    if !resource_path.exists() {
        return Err("Setup file not found in app resources.".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::ffi::OsStrExt;
        use std::iter::once;
        use winapi::um::shellapi::ShellExecuteW;
        use winapi::um::winuser::SW_SHOWNORMAL;
        use std::ffi::OsStr;

        let path_ws: Vec<u16> = resource_path.as_os_str().encode_wide().chain(once(0)).collect();
        let operation_ws: Vec<u16> = OsStr::new("runas").encode_wide().chain(once(0)).collect();
        
        let result = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                operation_ws.as_ptr(),
                path_ws.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                SW_SHOWNORMAL,
            )
        };

        if (result as isize) > 32 {
            Ok(())
        } else {
            Err(format!("Failed to start setup. The requested operation requires elevation. (os error {:?})", result))
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        // For non-Windows OS, attempt to run normally. Might fail if it needs root.
        Command::new(resource_path)
            .spawn()
            .map_err(|e| format!("Failed to start setup: {}", e))?;
        Ok(())
    }
}


// Command to get local IP address (dummy implementation)
#[command]
pub fn get_local_ip_address() -> Result<String, String> {
    // Return a dummy IP to fix the type error.
    // A real implementation would query the system's network interfaces.
    Ok("127.0.0.1".to_string())
}

// Helper function to process downloaded ZIP files
fn process_downloaded_zip(zip_path: &Path) -> Result<(), anyhow::Error> {
    println!("Processing downloaded ZIP file: {}", zip_path.display());
    
    // Create temporary directory for extraction
    let temp_dir = std::env::temp_dir().join(format!("oracle_extract_{}", Uuid::new_v4()));
    fs::create_dir_all(&temp_dir)?;
    println!("Created temporary directory: {}", temp_dir.display());
    
    // Open and extract the ZIP file
    let zip_file = File::open(zip_path)?;
    let mut archive = ZipArchive::new(zip_file)?;
    
    // Extract all files to temporary directory
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = temp_dir.join(file.name());
        
        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p)?;
                }
            }
            let mut outfile = File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }
    }
    
    println!("Extracted {} files to temporary directory", archive.len());
    
    // Define target directories
    let steam_config_base = Path::new("C:\\Program Files (x86)\\Steam\\config");
    let stplugin_dir = steam_config_base.join("stplug-in");
    let depotcache_dir = steam_config_base.join("depotcache");
    let statsexport_dir = steam_config_base.join("StatsExport");
    
    // Create target directories if they don't exist
    fs::create_dir_all(&stplugin_dir)?;
    fs::create_dir_all(&depotcache_dir)?;
    fs::create_dir_all(&statsexport_dir)?;
    
    // Count moved files
    let mut lua_count = 0;
    let mut manifest_count = 0;
    let mut bin_count = 0;
    
    // Walk through all files recursively
    let walker = WalkDir::new(&temp_dir).into_iter();
    for entry in walker.filter_map(Result::ok) {
        if entry.file_type().is_file() {
            let path = entry.path();
            let file_name = path.file_name().unwrap_or_default().to_string_lossy();
            
            // Process based on file extension/name
            if let Some(ext) = path.extension() {
                if ext == "lua" {
                    let target = stplugin_dir.join(path.file_name().unwrap_or_default());
                    fs::copy(path, &target)?;
                    lua_count += 1;
                    println!("Moved LUA file to stplug-in: {}", file_name);
                } else if ext == "bin" {
                    let target = statsexport_dir.join(path.file_name().unwrap_or_default());
                    fs::copy(path, &target)?;
                    bin_count += 1;
                    println!("Moved BIN file to StatsExport: {}", file_name);
                }
            }
            
            // Check for manifest files
            if file_name.to_lowercase().contains("manifest") {
                let target = depotcache_dir.join(path.file_name().unwrap_or_default());
                fs::copy(path, &target)?;
                manifest_count += 1;
                println!("Moved manifest file to depotcache: {}", file_name);
            }
        }
    }
    
    // Summary
    println!("File processing complete:");
    println!("- {} LUA files moved to stplug-in", lua_count);
    println!("- {} manifest files moved to depotcache", manifest_count);
    println!("- {} BIN files moved to StatsExport", bin_count);
    
    // Clean up temporary directory
    fs::remove_dir_all(&temp_dir)?;
    println!("Temporary directory cleaned up");
    
    Ok(())
}

#[command]
pub async fn list_downloaded_files(directory: String) -> Result<Vec<FileInfo>, String> {
    let path = Path::new(&directory);
    if !path.exists() {
        return Err(format!("Directory does not exist: {}", directory));
    }
    
    if !path.is_dir() {
        return Err(format!("Path is not a directory: {}", directory));
    }
    
    let mut files = Vec::new();
    
    for entry in WalkDir::new(path).max_depth(2).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        
        // Skip directories, only list files
        if path.is_dir() {
            continue;
        }
        
        // Only include zip files and manifests
        let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");
        if extension == "zip" || extension == "manifest" || extension == "lua" {
            if let Some(file_name) = path.file_name().and_then(|name| name.to_str()) {
                let file_size = fs::metadata(path).map(|meta| meta.len()).unwrap_or(0);
                let _relative_path = path.strip_prefix(directory.clone())
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| file_name.to_string());
                
                files.push(FileInfo {
                    name: file_name.to_string(),
                    path: path.to_string_lossy().to_string(),
                    size: file_size,
                    file_type: extension.to_string(),
                });
            }
        }
    }
    
    // Sort files by name
    files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    
    Ok(files)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub file_type: String,
}

#[command]
pub async fn open_file_or_folder(path: String) -> Result<(), String> {
    let path = Path::new(&path);
    
    if !path.exists() {
        return Err(format!("Path does not exist: {}", path.display()));
    }
    
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .args([path.to_string_lossy().to_string()])
            .spawn()
            .map_err(|e| format!("Failed to open file: {}", e))?;
    }
    
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .args([path.to_string_lossy().to_string()])
            .spawn()
            .map_err(|e| format!("Failed to open file: {}", e))?;
    }
    
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .args([path.to_string_lossy().to_string()])
            .spawn()
            .map_err(|e| format!("Failed to open file: {}", e))?;
    }
    
    Ok(())
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppSettings {
    pub download_directory: String,
}

#[command]
pub async fn save_settings(settings: AppSettings) -> Result<(), String> {
    let settings_dir = get_settings_dir()?;
    let settings_file = settings_dir.join("settings.json");
    
    // Create settings directory if it doesn't exist
    if !settings_dir.exists() {
        fs::create_dir_all(&settings_dir)
            .map_err(|e| format!("Failed to create settings directory: {}", e))?;
    }
    
    // Serialize settings to JSON
    let json = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;
    
    // Write to file
    fs::write(&settings_file, json)
        .map_err(|e| format!("Failed to write settings file: {}", e))?;
    
    Ok(())
}

#[command]
pub async fn load_settings() -> Result<AppSettings, String> {
    let settings_dir = get_settings_dir()?;
    let settings_file = settings_dir.join("settings.json");
    
    // Check if settings file exists
    if !settings_file.exists() {
        // Return default settings
        return Ok(AppSettings {
            download_directory: "downloads".to_string(),
        });
    }
    
    // Read file content
    let mut file = File::open(&settings_file)
        .map_err(|e| format!("Failed to open settings file: {}", e))?;
    
    let mut content = String::new();
    file.read_to_string(&mut content)
        .map_err(|e| format!("Failed to read settings file: {}", e))?;
    
    // Deserialize settings
    let settings: AppSettings = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse settings file: {}", e))?;
    
    Ok(settings)
}

// Helper function to get settings directory
fn get_settings_dir() -> Result<PathBuf, String> {
    let mut settings_dir = dirs_next::config_dir()
        .ok_or_else(|| "Could not find config directory".to_string())?;
    
    settings_dir.push("oracle-app");
    Ok(settings_dir)
}

#[command]
pub async fn update_game_files(app_id: String, game_name: String) -> Result<String, String> {
    println!("Starting update for AppID: {} ({})", app_id, game_name);

    let steam_config_path = find_steam_config_path().map_err(|e| e.to_string())?;
    let lua_file_path = find_lua_file_for_appid(&steam_config_path, &app_id)
        .map_err(|e| e.to_string())?;

    // --- 1. Download Branch Zip ---
    let client = reqwest::Client::builder()
        .user_agent("oracle-updater/1.0")
        .build().map_err(|e| e.to_string())?;
    
    // Define repositories to try
    let mut repos = HashMap::new();
    repos.insert("Fairyvmos/bruh-hub".to_string(), RepoType::Branch);
    repos.insert("SteamAutoCracks/ManifestHub".to_string(), RepoType::Branch);
    repos.insert("ManifestHub/ManifestHub".to_string(), RepoType::Decrypted);

    let mut zip_content: Option<bytes::Bytes> = None;

    for (repo_full_name, _) in &repos {
        let api_url = format!("https://api.github.com/repos/{}/zipball/{}", repo_full_name, app_id);
        println!("Trying to download from: {}", api_url);
        
        match client.get(&api_url).timeout(Duration::from_secs(600)).send().await {
            Ok(response) if response.status().is_success() => {
                zip_content = Some(response.bytes().await.map_err(|e| e.to_string())?);
                println!("Successfully downloaded zip from {}", repo_full_name);
                break;
            }
            Ok(response) => {
                 println!("Failed to download from {}. Status: {}", repo_full_name, response.status());
                continue;
            }
            Err(e) => {
                println!("Error downloading from {}: {}", repo_full_name, e);
                continue;
            }
        }
    }

    let Some(zip_bytes) = zip_content else {
        return Err("Failed to download game data from all repositories.".to_string());
    };

    // --- 2. Extract Manifests ---
    let temp_dir = std::env::temp_dir().join(format!("oracle_update_{}", Uuid::new_v4()));
    fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;

    let mut manifest_map: HashMap<String, String> = HashMap::new();
    let mut archive = ZipArchive::new(std::io::Cursor::new(zip_bytes)).map_err(|e| e.to_string())?;

    for i in 0..archive.len() {
        let file = archive.by_index(i).map_err(|e| e.to_string())?;
        let file_path = file.enclosed_name().ok_or("Invalid file path in zip".to_string())?;

        if let Some(ext) = file_path.extension() {
            if ext == "manifest" {
                if let Some(file_name_os) = file_path.file_name() {
                     if let Some(file_name) = file_name_os.to_str() {
                        // Filename format is DepotID_ManifestID.manifest
                        let re = Regex::new(r"(\d+)_(\d+)\.manifest").unwrap();
                        if let Some(caps) = re.captures(file_name) {
                            let depot_id = caps.get(1).unwrap().as_str().to_string();
                            let manifest_id = caps.get(2).unwrap().as_str().to_string();
                            manifest_map.insert(depot_id, manifest_id);
                        }
                    }
                }
            }
        }
    }
    
    if manifest_map.is_empty() {
        fs::remove_dir_all(&temp_dir).ok();
        return Err("No manifest files found in the downloaded archive.".to_string());
    }
    println!("Found {} new manifest IDs.", manifest_map.len());

    // --- 3. Update Lua File ---
    let original_lua_content = fs::read_to_string(&lua_file_path).map_err(|e| e.to_string())?;
    
    let mut updated_count = 0;
    let mut appended_count = 0;

    // Regex to find setManifestid(depot_id, "manifest_id", 0)
    let re_replace = Regex::new(r#"setManifestid\s*\(\s*(\d+)\s*,\s*"(\d+)"\s*,\s*0\s*\)"#).unwrap();
    let mut processed_depots: HashMap<String, bool> = HashMap::new();

    let mut updated_lua_content = re_replace.replace_all(&original_lua_content, |caps: &regex::Captures| {
        let depot_id = caps.get(1).unwrap().as_str();
        let old_manifest_id = caps.get(2).unwrap().as_str();
        processed_depots.insert(depot_id.to_string(), true);

        if let Some(new_manifest_id) = manifest_map.get(depot_id) {
            if new_manifest_id != old_manifest_id {
                updated_count += 1;
                format!(r#"setManifestid({}, "{}", 0)"#, depot_id, new_manifest_id)
            } else {
                caps.get(0).unwrap().as_str().to_string() // No change
            }
        } else {
            caps.get(0).unwrap().as_str().to_string() // No new manifest for this depot
        }
    }).to_string();

    // Append new manifest IDs
    let mut lines_to_append = Vec::new();
    for (depot_id, manifest_id) in &manifest_map {
        if !processed_depots.contains_key(depot_id) {
            lines_to_append.push(format!(r#"setManifestid({}, "{}", 0)"#, depot_id, manifest_id));
            appended_count += 1;
        }
    }
    
    if !lines_to_append.is_empty() {
        updated_lua_content.push_str("\n-- Appended by Yeyo Updater --\n");
        updated_lua_content.push_str(&lines_to_append.join("\n"));
        updated_lua_content.push('\n');
    }

    // --- 4. Save and Cleanup ---
    if updated_count > 0 || appended_count > 0 {
        fs::write(&lua_file_path, updated_lua_content).map_err(|e| e.to_string())?;
    }
    fs::remove_dir_all(&temp_dir).ok();

    let result_message = format!(
        "Update for {} complete. Updated: {}, Appended: {}.",
        game_name, updated_count, appended_count
    );
    println!("{}", result_message);
    Ok(result_message)
}

// Helper function to find Steam executable path from registry
#[cfg(target_os = "windows")]
fn find_steam_executable_path() -> Result<PathBuf, anyhow::Error> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(steam) = hkcu.open_subkey("Software\\Valve\\Steam") {
        if let Ok(steam_path_str) = steam.get_value::<String, _>("SteamPath") {
            let steam_exe_path = PathBuf::from(steam_path_str).join("Steam.exe");
            if steam_exe_path.exists() {
                return Ok(steam_exe_path);
            }
        }
    }
    
    // Fallback paths if registry fails
    let common_paths = [
        "C:\\Program Files (x86)\\Steam\\Steam.exe",
        "C:\\Program Files\\Steam\\Steam.exe",
    ];
    for path in common_paths.iter() {
        let p = PathBuf::from(path);
        if p.exists() {
            return Ok(p);
        }
    }

    Err(anyhow::anyhow!("Steam executable not found."))
}


fn find_steam_config_path() -> Result<PathBuf, anyhow::Error> {
    // For Windows
    #[cfg(target_os = "windows")]
    {
        use winreg::enums::*;
        use winreg::RegKey;

        // Check common paths first
        let common_paths = [
            "C:\\Program Files (x86)\\Steam\\config",
            "C:\\Program Files\\Steam\\config",
        ];
        for path in common_paths.iter() {
            let p = PathBuf::from(path);
            if p.exists() {
                return Ok(p);
            }
        }
        
        // Fallback to registry
        if let Ok(hkcu) = RegKey::predef(HKEY_CURRENT_USER).open_subkey("Software\\Valve\\Steam") {
            if let Ok(steam_path_str) = hkcu.get_value::<String, _>("SteamPath") {
                 let config_path = PathBuf::from(steam_path_str).join("config");
                 if config_path.exists() { return Ok(config_path); }
            }
        }
    }

    // For macOS and Linux (add paths as needed)
    #[cfg(not(target_os = "windows"))]
    {
        if let Some(home_dir) = dirs_next::home_dir() {
            let linux_paths = [
                ".steam/steam/config",
                ".local/share/Steam/config"
            ];
            let macos_path = "Library/Application Support/Steam/config";

            if cfg!(target_os = "linux") {
                for path in linux_paths.iter() {
                    let p = home_dir.join(path);
                    if p.exists() { return Ok(p); }
                }
            } else if cfg!(target_os = "macos") {
                let p = home_dir.join(macos_path);
                if p.exists() { return Ok(p); }
            }
        }
    }
    
    Err(anyhow::anyhow!("Steam config directory not found. Please set it manually in the settings."))
}

fn find_lua_file_for_appid(steam_config_path: &Path, app_id_to_find: &str) -> Result<PathBuf, anyhow::Error> {
    let stplugin_dir = steam_config_path.join("stplug-in");
    if !stplugin_dir.exists() {
        return Err(anyhow::anyhow!("'stplug-in' directory not found in Steam config."));
    }

    for entry in WalkDir::new(&stplugin_dir).max_depth(1).into_iter().filter_map(Result::ok) {
        if entry.file_type().is_file() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == "lua" {
                    // Check 1: Filename matches AppID (e.g., 12345.lua)
                    if let Some(stem) = path.file_stem() {
                        if stem.to_string_lossy() == app_id_to_find {
                            return Ok(path.to_path_buf());
                        }
                    }

                    // Check 2: File content contains addappid(AppID)
                    if let Ok(content) = fs::read_to_string(path) {
                        let re = Regex::new(&format!(r"addappid\s*\(\s*({})\s*\)", app_id_to_find)).unwrap();
                        if re.is_match(&content) {
                            return Ok(path.to_path_buf());
                        }
                    }
                }
            }
        }
    }

    Err(anyhow::anyhow!(format!("Could not find a .lua file for AppID: {}", app_id_to_find)))
}

#[command]
pub async fn get_batch_game_details(app_ids: Vec<String>) -> Result<Vec<SteamAppInfo>, String> {
    let mut details_list = Vec::new();
    for app_id in app_ids {
        match get_game_details(app_id.clone()).await {
            Ok(details) => details_list.push(details),
            Err(e) => eprintln!("Could not fetch details for AppID {}: {}", app_id, e), // Log error but continue
        }
    }
    Ok(details_list)
}

#[command]
pub async fn sync_dlcs_in_lua(main_app_id: String, dlc_ids_to_set: Vec<String>) -> Result<String, String> {
    // 1. Find the LUA file
    let steam_config_path = find_steam_config_path().map_err(|e| e.to_string())?;
    let lua_file_path = find_lua_file_for_appid(&steam_config_path, &main_app_id)
        .map_err(|e| e.to_string())?;

    // 2. Read the file content
    let original_content = fs::read_to_string(&lua_file_path).map_err(|e| e.to_string())?;

    // 3. Filter the content, keeping only non-DLC lines
    let addappid_re = Regex::new(r"addappid\s*\(\s*(\d+)\s*\)").unwrap();

    let filtered_lines: Vec<&str> = original_content
        .lines()
        .filter(|line| {
            if let Some(caps) = addappid_re.captures(line) {
                // This line contains an `addappid` call.
                // We check if the ID matches the main game ID.
                if let Some(id_str) = caps.get(1) {
                    // If it's the main game, we keep it. Otherwise, it's a DLC and we filter it out.
                    return id_str.as_str() == main_app_id;
                }
            }
            // Not an `addappid` line, so we keep it.
            true
        })
        .collect();

    let mut new_content = filtered_lines.join("\n");
    
    // 4. Append the new set of DLCs
    if !dlc_ids_to_set.is_empty() {
        if !new_content.is_empty() && !new_content.ends_with('\n') {
            new_content.push('\n');
        }
        new_content.push_str("\n-- DLCs Synced by Oracle --\n");
        for dlc_id in &dlc_ids_to_set {
            new_content.push_str(&format!("addappid({})\n", dlc_id));
        }
    }

    // 5. Write the new content back to the file
    fs::write(&lua_file_path, new_content).map_err(|e| e.to_string())?;

    Ok(format!("Successfully synced {} DLC(s).", dlc_ids_to_set.len()))
}

#[command]
pub async fn get_dlcs_in_lua(app_id: String) -> Result<Vec<String>, String> {
    let steam_config_path = find_steam_config_path().map_err(|e| e.to_string())?;
    let lua_file_path = find_lua_file_for_appid(&steam_config_path, &app_id)
        .map_err(|e| e.to_string())?;
    
    let content = fs::read_to_string(&lua_file_path).map_err(|e| e.to_string())?;
    
    let re = Regex::new(r"addappid\s*\(\s*(\d+)\s*\)").unwrap();
    let installed_dlcs = re.captures_iter(&content)
        .map(|cap| cap[1].to_string())
        .filter(|id| *id != app_id) // Exclude the main game's ID from the result
        .collect();
        
    Ok(installed_dlcs)
}