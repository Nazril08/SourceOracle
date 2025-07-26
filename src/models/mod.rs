use serde::Deserialize;
use std::collections::HashMap;

// Repository type enum
#[derive(Debug, Clone, PartialEq)]
pub enum RepoType {
    Branch,
    Encrypted,
    Decrypted,
}

impl From<&str> for RepoType {
    fn from(s: &str) -> Self {
        match s {
            "Branch" => RepoType::Branch,
            "Encrypted" => RepoType::Encrypted,
            "Decrypted" => RepoType::Decrypted,
            _ => RepoType::Branch, // Default to Branch for unknown types
        }
    }
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
    pub apps: HashMap<String, SteamAppDetails>,
}

#[derive(Debug, Deserialize)]
pub struct SteamAppDetails {
    pub success: bool,
    pub data: Option<SteamAppData>,
}

#[derive(Debug, Deserialize)]
pub struct SteamAppData {
    pub name: String,
    #[serde(rename = "type")]
    pub app_type: String,
}

// App state for GUI
#[derive(Debug, Clone)]
pub struct AppState {
    pub app_id: String,
    pub game_name: String,
    pub output_dir: String,
    pub repos: HashMap<String, RepoType>,
    pub download_status: DownloadStatus,
    pub log_messages: Vec<String>,
}

impl Default for AppState {
    fn default() -> Self {
        let mut repos = HashMap::new();
        repos.insert("Fairyvmos/bruh-hub".to_string(), RepoType::Branch);
        repos.insert("SteamAutoCracks/ManifestHub".to_string(), RepoType::Branch);
        repos.insert("ManifestHub/ManifestHub".to_string(), RepoType::Decrypted);

        Self {
            app_id: "381210".to_string(),
            game_name: "Dead by Daylight".to_string(),
            output_dir: "downloads".to_string(),
            repos,
            download_status: DownloadStatus::Idle,
            log_messages: Vec::new(),
        }
    }
}

impl AppState {
    pub async fn fetch_game_name(&mut self) -> Result<(), reqwest::Error> {
        let client = reqwest::Client::new();
        let url = format!("https://store.steampowered.com/api/appdetails?appids={}", self.app_id);
        
        self.log(&format!("Fetching game name for AppID: {}", self.app_id));
        
        match client.get(&url).send().await {
            Ok(response) => {
                if let Ok(app_details) = response.json::<SteamAppDetailsResponse>().await {
                    if let Some(app_data) = app_details.apps.get(&self.app_id) {
                        if app_data.success {
                            if let Some(data) = &app_data.data {
                                self.game_name = data.name.clone();
                                self.log(&format!("Found game name: {}", self.game_name));
                                return Ok(());
                            }
                        }
                    }
                    self.log(&format!("Failed to get game name from API response for AppID: {}", self.app_id));
                } else {
                    self.log("Failed to parse API response");
                }
            }
            Err(e) => {
                self.log(&format!("Error fetching game name: {}", e));
                return Err(e);
            }
        }
        
        Ok(())
    }
}

// Download status for the app
#[derive(Debug, Clone, PartialEq)]
pub enum DownloadStatus {
    Idle,
    Downloading,
    Success,
    Failed(String),
}

// Download result
pub type DownloadResult = Result<bool, anyhow::Error>;

// Logger trait for GUI integration
pub trait Logger {
    fn log(&mut self, message: &str);
}

impl Logger for AppState {
    fn log(&mut self, message: &str) {
        self.log_messages.push(message.to_string());
    }
} 