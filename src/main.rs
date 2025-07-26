mod models;
mod downloader;
mod gui;

use std::collections::HashMap;
use std::path::Path;
use std::fs::{self, File};
use std::io::Write;
use std::time::Duration;
use std::process::Command;
use std::sync::Mutex;

use anyhow::Result;
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use indicatif::{ProgressBar, ProgressStyle};
use clap::Parser;
use tauri::{Manager, State};

// Command-line arguments
#[derive(Parser, Debug)]
#[clap(author, version, about = "Download game data from GitHub repositories")]
struct Args {
    /// Run in CLI mode instead of GUI
    #[clap(short, long)]
    cli: bool,

    /// Steam AppID of the game (CLI mode only)
    #[clap(short, long, default_value = "381210")]
    app_id: String,

    /// Name of the game (CLI mode only)
    #[clap(short, long, default_value = "Dead by Daylight")]
    game_name: String,

    /// Output directory for downloaded files (CLI mode only)
    #[clap(short, long, default_value = "downloads")]
    output_dir: String,
}

// Types for GitHub API responses
#[derive(Debug, Deserialize)]
struct BranchResponse {
    commit: CommitObject,
}

#[derive(Debug, Deserialize)]
struct CommitObject {
    sha: String,
}

#[derive(Debug, Deserialize)]
struct TreeResponse {
    tree: Vec<TreeItem>,
}

#[derive(Debug, Deserialize)]
struct TreeItem {
    path: String,
    #[serde(rename = "type")]
    item_type: String,
}

// Repository type enum
#[derive(Debug, Clone, PartialEq)]
enum RepoType {
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

// Helper function to format errors
fn stack_error(e: &anyhow::Error) -> String {
    format!("{:?}", e)
}

// Function to download content from a CDN
async fn get_from_cdn(client: &Client, url: &str) -> Result<Option<bytes::Bytes>> {
    let domain = url.split('/').nth(2).unwrap_or("unknown");
    
    println!("Trying to download from {}", domain);
    
    match client.get(url)
        .timeout(Duration::from_secs(20))
        .send()
        .await {
            Ok(response) => {
                if response.status() == StatusCode::OK {
                    println!("[OK] Success from {}", domain);
                    let bytes = response.bytes().await?;
                    Ok(Some(bytes))
                } else {
                    println!("[FAIL] Status {} from {}", response.status(), domain);
                    Ok(None)
                }
            },
            Err(e) => {
                println!("[ERROR] Failed to contact {}: {}", domain, e);
                Ok(None)
            }
        }
}

// Function to download a single file from a repository
async fn download_file_content(client: &Client, repo_full_name: &str, sha: &str, path: &str) -> Result<Option<bytes::Bytes>> {
    println!("Trying to download: {} from repo {}", path, repo_full_name);
    
    let urls = vec![
        format!("https://gcore.jsdelivr.net/gh/{}@{}/{}", repo_full_name, sha, path),
        format!("https://fastly.jsdelivr.net/gh/{}@{}/{}", repo_full_name, sha, path),
        format!("https://cdn.jsdelivr.net/gh/{}@{}/{}", repo_full_name, sha, path),
        format!("https://raw.githubusercontent.com/{}/{}/{}", repo_full_name, sha, path),
    ];
    
    for url in urls {
        match get_from_cdn(client, &url).await? {
            Some(content) => return Ok(Some(content)),
            None => continue,
        }
    }
    
    println!("[TOTAL FAILURE] Could not download file: {}", path);
    Ok(None)
}

// Function to download an entire branch as a ZIP file
async fn download_branch_zip(client: &Client, repo_full_name: &str, branch_name: &str) -> Result<Option<bytes::Bytes>> {
    let api_url = format!("https://api.github.com/repos/{}/zipball/{}", repo_full_name, branch_name);
    println!("Trying to download branch zip from: {}", api_url);
    
    match client.get(&api_url)
        .timeout(Duration::from_secs(600))
        .send()
        .await {
            Ok(response) => {
                if response.status() == StatusCode::OK {
                    println!("Successfully downloaded zip content for branch {}", branch_name);
                    let bytes = response.bytes().await?;
                    Ok(Some(bytes))
                } else {
                    println!("Failed to download branch zip. Status: {}", response.status());
                    Ok(None)
                }
            },
            Err(e) => {
                println!("Error when downloading branch zip: {}", stack_error(&e.into()));
                Ok(None)
            }
        }
}

// Main function to download from a repository
async fn download_from_repo(
    app_id: &str, 
    game_name: &str, 
    repo_info: &HashMap<String, RepoType>, 
    output_dir: &str
) -> Result<bool> {
    fs::create_dir_all(output_dir)?;
    
    let sanitized_game_name = sanitize_filename::sanitize(game_name);
    let client = Client::builder()
        .user_agent("oracle-downloader/1.0")
        .build()?;
    
    for (repo_full_name, repo_type) in repo_info {
        println!("\n--- Trying Repository: {} (Type: {:?}) ---", repo_full_name, repo_type);
        
        if *repo_type == RepoType::Branch {
            // Try to download the entire branch as a ZIP file
            match download_branch_zip(&client, repo_full_name, app_id).await? {
                Some(zip_content) => {
                    let zip_path = Path::new(output_dir)
                        .join(format!("{} - {} (Branch).zip", sanitized_game_name, app_id));
                    
                    let mut file = File::create(&zip_path)?;
                    file.write_all(&zip_content)?;
                    
                    println!("SUCCESS! Branch repo saved to: {}", zip_path.display());
                    return Ok(true); // Stop after successfully finding from one repo
                },
                None => continue,
            }
        } else {
            // Logic for non-branch repos (more complex)
            // 1. Get the latest commit SHA from the AppID branch
            let branch_api_url = format!("https://api.github.com/repos/{}/branches/{}", repo_full_name, app_id);
            
            let branch_response = match client.get(&branch_api_url).send().await {
                Ok(response) => {
                    if response.status() != StatusCode::OK {
                        println!("Failed to find branch {} in repo {}", app_id, repo_full_name);
                        continue;
                    }
                    response.json::<BranchResponse>().await?
                },
                Err(e) => {
                    println!("Error fetching branch info: {}", e);
                    continue;
                }
            };
            
            let sha = &branch_response.commit.sha;
            
            // 2. Get the list of files in that branch
            let tree_url = format!("https://api.github.com/repos/{}/git/trees/{}?recursive=1", repo_full_name, sha);
            
            let tree_response = match client.get(&tree_url).send().await {
                Ok(response) => {
                    if response.status() != StatusCode::OK {
                        println!("Failed to get file list for branch {}", app_id);
                        continue;
                    }
                    response.json::<TreeResponse>().await?
                },
                Err(e) => {
                    println!("Error fetching tree info: {}", e);
                    continue;
                }
            };
            
            let files_to_download: Vec<String> = tree_response.tree
                .iter()
                .filter(|item| item.item_type == "blob")
                .map(|item| item.path.clone())
                .collect();
            
            // 3. Download all files
            let temp_download_dir = Path::new(output_dir)
                .join(format!("_{}_{}_{}_temp", sanitized_game_name, app_id, repo_type == &RepoType::Encrypted));
            
            fs::create_dir_all(&temp_download_dir)?;
            
            let mut files_written = 0;
            
            // Create a progress bar
            let pb = ProgressBar::new(files_to_download.len() as u64);
            pb.set_style(ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} files ({eta})")
                .unwrap()
                .progress_chars("#>-"));
            
            for path in &files_to_download {
                match download_file_content(&client, repo_full_name, sha, path).await? {
                    Some(content) => {
                        let file_path = temp_download_dir.join(path);
                        
                        // Create parent directories if they don't exist
                        if let Some(parent) = file_path.parent() {
                            fs::create_dir_all(parent)?;
                        }
                        
                        let mut file = File::create(&file_path)?;
                        file.write_all(&content)?;
                        
                        files_written += 1;
                    },
                    None => {},
                }
                
                pb.inc(1);
            }
            
            pb.finish_with_message("Download complete");
            
            if files_written > 0 {
                println!("SUCCESS! {} files from non-branch repo saved in temp folder: {}", 
                    files_written, temp_download_dir.display());
                return Ok(true);
            }
        }
    }
    
    println!("\n[FINISHED] Failed to find data for AppID {} from all selected repositories.", app_id);
    Ok(false)
}

// Struct for the new Account Sharing feature
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
struct Account {
    #[serde(rename = "gameName")]
    game_name: String,
    #[serde(rename = "displayName")]
    display_name: String,
    #[serde(rename = "steamUsername")]
    steam_username: String,
    #[serde(rename = "steamPassword")]
    steam_password: String,
    #[serde(rename = "imageUrl")]
    image_url: String,
}

// Tauri state to hold the list of accounts
struct AccountState(Mutex<Vec<Account>>);

// Tauri command to get the list of accounts for the frontend
#[tauri::command]
fn get_accounts(state: State<AccountState>) -> Result<Vec<Account>, String> {
    let accounts = state.0.lock().unwrap();
    Ok(accounts.clone())
}

// Tauri command to switch the Steam account
#[tauri::command]
fn switch_steam_account(username: String, password: String) -> Result<(), String> {
    // This path might need to be configurable in the future
    let steam_path = "C:\\Program Files (x86)\\Steam\\Steam.exe";

    // Forcefully close any running Steam process to allow a new login
    let kill_status = Command::new("taskkill")
        .args(&["/F", "/IM", "steam.exe"])
        .status()
        .map_err(|e| e.to_string())?;

    if kill_status.success() {
        println!("Successfully terminated running Steam process.");
    } else {
        // This is not a fatal error; Steam might not have been running.
        println!("Could not terminate Steam process (it might not have been running).");
    }

    // A short delay to ensure the process has fully terminated
    std::thread::sleep(Duration::from_secs(3));

    // Relaunch Steam with the new account credentials
    Command::new(steam_path)
        .args(&["-login", &username, &password])
        .spawn() // Use spawn to not block the Tauri app
        .map_err(|e| e.to_string())?;

    println!("Attempting to launch Steam with user: {}", username);
    Ok(())
}

// Main entry point of the application
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if args.cli {
        // If --cli flag is used, run the command-line version
        println!("Running in CLI mode...");
        run_cli_mode(&args).await?;
    } else {
        // Otherwise, run the new Tauri-based GUI
        println!("Starting GUI mode...");
        
        // Load account data from JSON file at startup
        let account_file_path = "tauri-app/src-tauri/accounts.json";
        let accounts_data = fs::read_to_string(account_file_path)
            .expect("Fatal: Could not read accounts.json file.");

        let accounts: Vec<Account> = serde_json::from_str(&accounts_data)
            .expect("Fatal: Could not parse accounts.json file.");

        // Build and run the Tauri application
        tauri::Builder::default()
            .manage(AccountState(Mutex::new(accounts)))
            .setup(|app| {
                let app_handle = app.handle();
                // We need to figure out the correct way to call the initialize function
                // from the other crate. This will likely cause a compile error, which
                // will guide the next step.
                tauri::async_runtime::spawn(async move {
                    // This path is incorrect and needs to be fixed.
                    // let _ = tauri_app::src_tauri::library::initialize_app_cache().await;
                    
                    app_handle.emit_all("initialization_complete", ()).unwrap();
                });
                Ok(())
            })
            .invoke_handler(tauri::generate_handler![
                get_accounts,
                switch_steam_account
                // Re-add library commands once the module path is resolved.
            ])
            .run(tauri::generate_context!("./tauri-app/src-tauri/tauri.conf.json"))
            .expect("Fatal: Error while running Tauri application.");
    }

    Ok(())
}

// The existing CLI mode logic. This remains unchanged.
// It uses the downloader and models modules.
async fn run_cli_mode(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    use std::collections::HashMap;
    use models::{RepoType, Logger, AppState};

    // Simple logger for CLI mode
    struct CliLogger;
    impl Logger for CliLogger {
        fn log(&mut self, message: &str) {
            println!("{}", message);
        }
    }

    let mut logger = CliLogger;
    
    // List of repositories to try (name: type)
    let mut repos_to_try: HashMap<String, RepoType> = HashMap::new();
    repos_to_try.insert("Fairyvmos/bruh-hub".to_string(), RepoType::Branch);
    repos_to_try.insert("SteamAutoCracks/ManifestHub".to_string(), RepoType::Branch);
    repos_to_try.insert("ManifestHub/ManifestHub".to_string(), RepoType::Decrypted);
    
    // If app_id is provided but game_name is still the default, try to fetch the game name
    let mut game_name = args.game_name.clone();
    
    if args.app_id != "381210" && args.game_name == "Dead by Daylight" {
        println!("Trying to fetch game name for AppID: {}", args.app_id);
        let mut app_state = AppState {
            app_id: args.app_id.clone(),
            game_name: "Unknown".to_string(),
            output_dir: args.output_dir.clone(),
            repos: repos_to_try.clone(),
            download_status: models::DownloadStatus::Idle,
            log_messages: Vec::new(),
        };
        
        if let Ok(_) = app_state.fetch_game_name().await {
            game_name = app_state.game_name;
            println!("Found game name: {}", game_name);
        } else {
            println!("Failed to fetch game name, using default");
        }
    }
    
    println!("Starting download for {} (AppID: {})", game_name, args.app_id);
    
    let download_result = downloader::download_from_repo(&args.app_id, &game_name, &repos_to_try, &args.output_dir, &mut logger).await;
    
    match download_result {
        Ok(true) => {
            println!("Download completed successfully!");
            
            // Process the downloaded ZIP file
            let zip_path = std::path::Path::new(&args.output_dir)
                .join(format!("{} - {} (Branch).zip", 
                    sanitize_filename::sanitize(&game_name), 
                    args.app_id));
            
            if zip_path.exists() {
                println!("Processing downloaded ZIP file...");
                let mut app_state = AppState {
                    app_id: args.app_id.clone(),
                    game_name: game_name.clone(),
                    output_dir: args.output_dir.clone(),
                    repos: repos_to_try,
                    download_status: models::DownloadStatus::Idle,
                    log_messages: Vec::new(),
                };
                
                match app_state.process_downloaded_zip(&zip_path) {
                    Ok(_) => println!("ZIP file processed successfully!"),
                    Err(e) => println!("Error processing ZIP file: {}", e),
                }
            }
        },
        Ok(false) => println!("Download process completed but no data was found."),
        Err(e) => println!("Error during download process: {}", e),
    }
    
    Ok(())
}
