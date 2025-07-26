use serde::{Deserialize, Serialize, Deserializer};
use std::collections::HashMap;
use std::sync::RwLock;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use anyhow::Result;
use dirs_next::data_dir;
use serde_json::Value;

// Repository type enum
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RepoType {
    Branch,
    Encrypted,
    Decrypted,
}

// Types for GitHub API responses
#[derive(Debug, Deserialize)]
pub struct BranchResponse {
    pub commit: CommitObject,
}

#[derive(Debug, Deserialize)]
pub struct CommitObject {
    pub sha: String,
}

#[derive(Debug, Deserialize)]
pub struct TreeResponse {
    pub tree: Vec<TreeItem>,
}

#[derive(Debug, Deserialize)]
pub struct TreeItem {
    pub path: String,
    #[serde(rename = "type")]
    pub item_type: String,
}

// Steam API response structures
#[derive(Debug, Deserialize)]
pub struct SteamAppDetailsResponse {
    #[serde(flatten)]
    pub apps: HashMap<String, SteamAppData>,
}

#[derive(Debug, Deserialize)]
pub struct SteamAppData {
    pub success: bool,
    pub data: Option<SteamAppInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReleaseDateInfo {
    pub coming_soon: bool,
    pub date: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SteamAppInfo {
    pub name: String,
    pub steam_appid: u64,
    pub header_image: String,
    #[serde(default)]
    pub publishers: Vec<String>,
    #[serde(default)]
    pub developers: Vec<String>,
    pub release_date: ReleaseDateInfo,
    pub short_description: String,
    #[serde(default)]
    pub drm_notice: Option<String>,
    #[serde(default, deserialize_with = "deserialize_dlc_robustly")]
    pub dlc: Vec<u64>,
}

fn deserialize_dlc_robustly<'de, D>(deserializer: D) -> Result<Vec<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    // Deserialize into an optional, generic JSON Value to handle `null` or missing fields gracefully.
    let json_value: Option<Value> = Deserialize::deserialize(deserializer)?;

    let json_value = match json_value {
        Some(v) => v,
        None => return Ok(Vec::new()),
    };
    
    // Helper closure to convert a JSON Value to u64, trying from number or string.
    let to_u64 = |v: &Value| {
        v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse::<u64>().ok()))
    };

    match json_value {
        // Case 1: It's a JSON Array, e.g., [123, 456]
        Value::Array(arr) => {
            Ok(arr.iter().filter_map(to_u64).collect())
        }
        // Case 2: It's a JSON Object, e.g., {"0": 123, "1": 456}
        Value::Object(obj) => {
            Ok(obj.values().filter_map(to_u64).collect())
        }
        // Any other case, return an empty Vec.
        _ => Ok(Vec::new()),
    }
}

// Game data structure for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameInfo {
    pub app_id: String,
    pub game_name: String,
    pub icon_url: Option<String>,
}

// Search results structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResults {
    pub games: Vec<GameInfo>,
    pub total: usize,
    pub page: usize,
    pub total_pages: usize,
    pub query: String,
}

// Types for download functionality
pub type DownloadResult = Result<bool, String>;

#[derive(Debug, Serialize, Clone, Copy, PartialEq)]
pub enum DownloadStatus {
    Pending,
    Downloading,
    Completed,
    Failed,
}

// Steam API response structure for GetAppList
#[derive(Debug, Deserialize)]
pub struct SteamAppListResponse {
    pub applist: SteamAppList,
}

#[derive(Debug, Deserialize)]
pub struct SteamAppList {
    pub apps: Vec<SteamAppListEntry>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SteamAppListEntry {
    pub appid: u64,
    pub name: String,
}

// Struct to hold the cached app list with a timestamp
#[derive(Debug, Serialize, Deserialize)]
struct CachedAppList {
    timestamp: u64,
    apps: Vec<SteamAppListEntry>,
}

// Game database with efficient search capabilities
pub struct GameDatabase {
    apps: RwLock<Vec<SteamAppListEntry>>,
    is_loaded: RwLock<bool>,
    cache_path: PathBuf,
}

// Steam app cache for quick AppID to name lookups
pub struct SteamAppCache {
    apps: RwLock<HashMap<String, String>>,
    is_loaded: RwLock<bool>,
}

impl SteamAppCache {
    pub fn new() -> Self {
        Self {
            apps: RwLock::new(HashMap::new()),
            is_loaded: RwLock::new(false),
        }
    }

    pub fn is_loaded(&self) -> bool {
        *self.is_loaded.read().unwrap()
    }

    pub async fn load_from_steam_api(&self) -> Result<(), String> {
        let client = reqwest::Client::new();
        let url = "https://api.steampowered.com/ISteamApps/GetAppList/v2/";
        
        match client.get(url).send().await {
            Ok(response) => {
                if !response.status().is_success() {
                    return Err(format!("Steam API returned error: {}", response.status()));
                }
                
                match response.json::<SteamAppListResponse>().await {
                    Ok(app_list) => {
                        let mut apps = self.apps.write().unwrap();
                        *apps = app_list.applist.apps.iter().map(|app| {
                            (app.appid.to_string(), app.name.clone())
                        }).collect();
                        
                        let mut is_loaded = self.is_loaded.write().unwrap();
                        *is_loaded = true;
                        
                        println!("Loaded {} games from Steam API for name cache", apps.len());
                        Ok(())
                    },
                    Err(e) => Err(format!("Failed to parse Steam API response: {}", e)),
                }
            },
            Err(e) => Err(format!("Failed to connect to Steam API: {}", e)),
        }
    }

    pub fn get_game_name(&self, app_id: &str) -> Option<String> {
        let apps = self.apps.read().unwrap();
        apps.get(app_id).cloned()
    }
}

impl GameDatabase {
    pub fn new() -> Self {
        let cache_path = Self::get_cache_path().expect("Failed to determine cache directory");
        // Ensure cache directory exists
        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent).expect("Failed to create cache directory");
        }

        Self {
            apps: RwLock::new(Vec::new()),
            is_loaded: RwLock::new(false),
            cache_path,
        }
    }

    fn get_cache_path() -> Result<PathBuf> {
        let mut path = data_dir().ok_or_else(|| anyhow::anyhow!("Failed to get data directory"))?;
        path.push("Oracle/cache/applist.json");
        Ok(path)
    }

    pub fn is_loaded(&self) -> bool {
        *self.is_loaded.read().unwrap()
    }

    // Tries to load apps from the cache file
    fn load_from_cache(&self) -> Result<Vec<SteamAppListEntry>> {
        let mut file = File::open(&self.cache_path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        
        let cached_data: CachedAppList = serde_json::from_str(&contents)?;

        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let cache_age = Duration::from_secs(now - cached_data.timestamp);

        // Cache is valid for 3 days
        if cache_age < Duration::from_secs(3 * 24 * 60 * 60) {
            println!("Loaded {} games from cache.", cached_data.apps.len());
            Ok(cached_data.apps)
        } else {
            println!("Cache is outdated. Fetching new list from Steam API.");
            Err(anyhow::anyhow!("Cache expired"))
        }
    }

    // Saves the app list to the cache file
    fn save_to_cache(&self, apps: &[SteamAppListEntry]) -> Result<()> {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let cached_data = CachedAppList {
            timestamp,
            apps: apps.to_vec(),
        };
        let contents = serde_json::to_string(&cached_data)?;
        let mut file = File::create(&self.cache_path)?;
        file.write_all(contents.as_bytes())?;
        Ok(())
    }
    
    // Combined function to load from cache or fetch from API
    pub async fn load_or_refresh_db(&self) -> Result<(), String> {
        if self.is_loaded() {
            return Ok(());
        }

        // Try to load from cache first
        if let Ok(apps_from_cache) = self.load_from_cache() {
            let mut apps = self.apps.write().unwrap();
            *apps = apps_from_cache;
            let mut is_loaded = self.is_loaded.write().unwrap();
            *is_loaded = true;
            return Ok(());
        }

        // If cache fails or is expired, fetch from API
        println!("Fetching app list from Steam API...");
        let client = reqwest::Client::new();
        let url = "https://api.steampowered.com/ISteamApps/GetAppList/v2/";
        
        match client.get(url).send().await {
            Ok(response) => {
                if !response.status().is_success() {
                    return Err(format!("Steam API returned error: {}", response.status()));
                }
                
                match response.json::<SteamAppListResponse>().await {
                    Ok(app_list) => {
                        let mut apps = self.apps.write().unwrap();
                        *apps = app_list.applist.apps;
                        
                        if let Err(e) = self.save_to_cache(&apps) {
                            eprintln!("Failed to save game list to cache: {}", e);
                        }
                        
                        let mut is_loaded = self.is_loaded.write().unwrap();
                        *is_loaded = true;
                        
                        println!("Loaded {} games from Steam API", apps.len());
                        Ok(())
                    },
                    Err(e) => Err(format!("Failed to parse Steam API response: {}", e)),
                }
            },
            Err(e) => Err(format!("Failed to connect to Steam API: {}", e)),
        }
    }

    pub fn search(&self, query: &str, page: usize, per_page: usize) -> SearchResults {
        let apps = self.apps.read().unwrap();
        
        if query.trim().is_empty() {
            return SearchResults {
                games: Vec::new(),
                total: 0,
                page: 1,
                total_pages: 1,
                query: String::new(),
            };
        }
        
        // Split query by comma for multiple search terms
        let search_terms: Vec<String> = query.split(',')
            .map(|term| term.trim().to_lowercase())
            .filter(|term| !term.is_empty())
            .collect();
        
        // Only log search terms on first page or when terms change
        if page == 1 {
            println!("Search terms: {:?}", search_terms);
        }

        // Check if the user is explicitly searching for a non-game type
        let searching_for_non_game = search_terms.iter().any(|term| {
            ["dlc", "soundtrack", "demo", "pack", "artbook", "trailer", "movie", "beta", "pass"].contains(&term.as_str())
        });
        
        let matching_apps: Vec<_> = apps.iter().filter(|app| {
            let app_name_lower = app.name.to_lowercase();
            
            // The item must match one of the search terms to even be considered.
            let matches_query = search_terms.iter().any(|term| app_name_lower.contains(term));
            if !matches_query {
                return false;
            }

            // If the user is specifically looking for DLC, packs, etc., don't filter them.
            if searching_for_non_game {
                return true;
            }

            // Otherwise, filter out items containing common non-game keywords.
            let is_non_game = [
                "dlc", "soundtrack", "demo", "pack", "sdk", "artbook", "trailer", 
                "movie", "beta", "ost", "original sound", "wallpaper", "art book", 
                "season pass", "bonus content", "uncut", "spin-off", "spinoff", "costume", 
                "hd", "technique", "sneakers", "pre-purchase", "pre-order", "pre-orders",
                "expansion", "upgrade", "additional", "perks", "gesture","guide", "manual",
                "jingle", "ce", "playtest", "special weapon", "danbo head", "making weapon",
                "outfit", "dress", "bonus stamp", "add-on", "debundle", "the great ninja war",
                "training set", "cd key", "key", "code", "gift", "gift code", "gift card",
                "mac", "activation", "uplay activation", "ubisoft activation", "deluxe", "(SP)", "Fields of Elysium"
            ].iter().any(|keyword| app_name_lower.contains(keyword));
            
            !is_non_game
        }).cloned().collect();
        
        let total = matching_apps.len();
        let total_pages = (total as f64 / per_page as f64).ceil() as usize;
        let current_page = page.max(1).min(total_pages);
        
        // Get the slice for current page
        let start = (current_page - 1) * per_page;
        let end = (start + per_page).min(total);
        
        let page_items: Vec<GameInfo> = if start <= end {
            matching_apps[start..end]
                .iter()
                .map(|app| GameInfo {
                    app_id: app.appid.to_string(),
                    game_name: app.name.clone(),
                    icon_url: Some(format!("https://cdn.akamai.steamstatic.com/steam/apps/{}/header.jpg", app.appid)),
                })
                .collect()
        } else {
            Vec::new()
        };
        
        // Only log pagination info on debug mode
        if cfg!(debug_assertions) {
            println!("Found {} total results, returning page {} of {} with {} items",
                total, current_page, total_pages, page_items.len());
        }
        
        SearchResults {
            games: page_items,
            total,
            page: current_page,
            total_pages,
            query: query.to_string(),
        }
    }
    
    pub fn get_by_app_id(&self, app_id: &str) -> Option<GameInfo> {
        if let Ok(app_id_num) = app_id.parse::<u64>() {
            let apps = self.apps.read().unwrap();
            
            apps.iter()
                .find(|app| app.appid == app_id_num)
                .map(|app| GameInfo {
                    app_id: app.appid.to_string(),
                    game_name: app.name.clone(),
                    icon_url: Some(format!("https://steamcdn-a.akamaihd.net/steam/apps/{}/header.jpg", app.appid)),
                })
        } else {
            None
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Account {
    #[serde(rename = "gameName")]
    pub game_name: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "steamUsername")]
    pub steam_username: String,
    #[serde(rename = "steamPassword")]
    pub steam_password: String,
    #[serde(rename = "imageUrl")]
    pub image_url: String,
    #[serde(default)]
    pub drm: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Note {
    pub id: String,
    pub title: String,
    pub content: String,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SteamApp {
    // ... existing code ...
} 