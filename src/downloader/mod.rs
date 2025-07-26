use std::collections::HashMap;
use std::path::Path;
use std::fs::{self, File};
use std::io::Write;
use std::time::Duration;

use anyhow::Result;
use bytes::Bytes;
use reqwest::{Client, StatusCode};
use indicatif::{ProgressBar, ProgressStyle};

use crate::models::{RepoType, BranchResponse, TreeResponse, Logger, DownloadResult};

// Helper function to format errors
fn stack_error(e: &anyhow::Error) -> String {
    format!("{:?}", e)
}

// Function to download content from a CDN
pub async fn get_from_cdn<L: Logger>(client: &Client, url: &str, logger: &mut L) -> Result<Option<Bytes>> {
    let domain = url.split('/').nth(2).unwrap_or("unknown");
    
    logger.log(&format!("Trying to download from {}", domain));
    
    match client.get(url)
        .timeout(Duration::from_secs(20))
        .send()
        .await {
            Ok(response) => {
                if response.status() == StatusCode::OK {
                    logger.log(&format!("[OK] Success from {}", domain));
                    let bytes = response.bytes().await?;
                    Ok(Some(bytes))
                } else {
                    logger.log(&format!("[FAIL] Status {} from {}", response.status(), domain));
                    Ok(None)
                }
            },
            Err(e) => {
                logger.log(&format!("[ERROR] Failed to contact {}: {}", domain, e));
                Ok(None)
            }
        }
}

// Function to download a single file from a repository
pub async fn download_file_content<L: Logger>(
    client: &Client, 
    repo_full_name: &str, 
    sha: &str, 
    path: &str,
    logger: &mut L
) -> Result<Option<Bytes>> {
    logger.log(&format!("Trying to download: {} from repo {}", path, repo_full_name));
    
    let urls = vec![
        format!("https://gcore.jsdelivr.net/gh/{}@{}/{}", repo_full_name, sha, path),
        format!("https://fastly.jsdelivr.net/gh/{}@{}/{}", repo_full_name, sha, path),
        format!("https://cdn.jsdelivr.net/gh/{}@{}/{}", repo_full_name, sha, path),
        format!("https://raw.githubusercontent.com/{}/{}/{}", repo_full_name, sha, path),
    ];
    
    for url in urls {
        match get_from_cdn(client, &url, logger).await? {
            Some(content) => return Ok(Some(content)),
            None => continue,
        }
    }
    
    logger.log(&format!("[TOTAL FAILURE] Could not download file: {}", path));
    Ok(None)
}

// Function to download an entire branch as a ZIP file
pub async fn download_branch_zip<L: Logger>(
    client: &Client, 
    repo_full_name: &str, 
    branch_name: &str,
    logger: &mut L
) -> Result<Option<Bytes>> {
    let api_url = format!("https://api.github.com/repos/{}/zipball/{}", repo_full_name, branch_name);
    logger.log(&format!("Trying to download branch zip from: {}", api_url));
    
    match client.get(&api_url)
        .timeout(Duration::from_secs(600))
        .send()
        .await {
            Ok(response) => {
                if response.status() == StatusCode::OK {
                    logger.log(&format!("Successfully downloaded zip content for branch {}", branch_name));
                    let bytes = response.bytes().await?;
                    Ok(Some(bytes))
                } else {
                    logger.log(&format!("Failed to download branch zip. Status: {}", response.status()));
                    Ok(None)
                }
            },
            Err(e) => {
                logger.log(&format!("Error when downloading branch zip: {}", stack_error(&e.into())));
                Ok(None)
            }
        }
}

// Main function to download from a repository
pub async fn download_from_repo<L: Logger>(
    app_id: &str, 
    game_name: &str, 
    repo_info: &HashMap<String, RepoType>, 
    output_dir: &str,
    logger: &mut L
) -> DownloadResult {
    fs::create_dir_all(output_dir)?;
    
    let sanitized_game_name = sanitize_filename::sanitize(game_name);
    let client = Client::builder()
        .user_agent("oracle-downloader/1.0")
        .build()?;
    
    for (repo_full_name, repo_type) in repo_info {
        logger.log(&format!("\n--- Trying Repository: {} (Type: {:?}) ---", repo_full_name, repo_type));
        
        if *repo_type == RepoType::Branch {
            // Try to download the entire branch as a ZIP file
            match download_branch_zip(&client, repo_full_name, app_id, logger).await? {
                Some(zip_content) => {
                    let zip_path = Path::new(output_dir)
                        .join(format!("{} - {} (Branch).zip", sanitized_game_name, app_id));
                    
                    let mut file = File::create(&zip_path)?;
                    file.write_all(&zip_content)?;
                    
                    logger.log(&format!("SUCCESS! Branch repo saved to: {}", zip_path.display()));
                    
                    // Process the downloaded ZIP file
                    let mut app_state = crate::models::AppState::default();
                    app_state.app_id = app_id.to_string();
                    app_state.game_name = game_name.to_string();
                    app_state.output_dir = output_dir.to_string();
                    
                    // Log messages from AppState will be forwarded to our logger
                    match app_state.process_downloaded_zip(&zip_path) {
                        Ok(_) => {
                            logger.log("Successfully processed ZIP file contents");
                            for message in &app_state.log_messages {
                                logger.log(message);
                            }
                        },
                        Err(e) => {
                            logger.log(&format!("Error processing ZIP file: {}", e));
                        }
                    }
                    
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
                        logger.log(&format!("Failed to find branch {} in repo {}", app_id, repo_full_name));
                        continue;
                    }
                    response.json::<BranchResponse>().await?
                },
                Err(e) => {
                    logger.log(&format!("Error fetching branch info: {}", e));
                    continue;
                }
            };
            
            let sha = &branch_response.commit.sha;
            
            // 2. Get the list of files in that branch
            let tree_url = format!("https://api.github.com/repos/{}/git/trees/{}?recursive=1", repo_full_name, sha);
            
            let tree_response = match client.get(&tree_url).send().await {
                Ok(response) => {
                    if response.status() != StatusCode::OK {
                        logger.log(&format!("Failed to get file list for branch {}", app_id));
                        continue;
                    }
                    response.json::<TreeResponse>().await?
                },
                Err(e) => {
                    logger.log(&format!("Error fetching tree info: {}", e));
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
            let total_files = files_to_download.len();
            
            // Create a progress bar
            let pb = ProgressBar::new(total_files as u64);
            pb.set_style(ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} files ({eta})")
                .unwrap()
                .progress_chars("#>-"));
            
            logger.log(&format!("Starting download of {} files", total_files));
            
            for (i, path) in files_to_download.iter().enumerate() {
                if i % 10 == 0 {
                    logger.log(&format!("Progress: {}/{} files", i, total_files));
                }
                
                match download_file_content(&client, repo_full_name, sha, path, logger).await? {
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
                logger.log(&format!("SUCCESS! {} files from non-branch repo saved in temp folder: {}", 
                    files_written, temp_download_dir.display()));
                return Ok(true);
            }
        }
    }
    
    logger.log(&format!("\n[FINISHED] Failed to find data for AppID {} from all selected repositories.", app_id));
    Ok(false)
} 