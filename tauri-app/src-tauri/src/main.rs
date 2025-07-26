// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod models;
mod library;

use commands::{search_games, get_game_details, fetch_game_name, download_game, restart_steam, initialize_database, search_game_by_name, list_downloaded_files, open_file_or_folder, save_settings, load_settings};
use library::{check_steam_directories, get_library_games, update_game, remove_game, initialize_app_cache, get_game_name_by_appid};
use models::{SteamAppCache, Account, Note};
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;
use tauri::{Manager, State, AppHandle};
use std::fs;
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::path::PathBuf;
use uuid::Uuid;

// Global instance of the Steam App Cache
pub static APP_CACHE: Lazy<Arc<SteamAppCache>> = Lazy::new(|| {
    Arc::new(SteamAppCache::new())
});

// Tauri state to hold the list of accounts
struct AccountState(Mutex<Vec<Account>>);

// Tauri state to hold the list of notes
struct NoteState(Mutex<Vec<Note>>);

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

    // Relaunch Steam with the new account credentials, without the -silent flag
    Command::new(steam_path)
        .args(&["-login", &username, &password])
        .spawn() // Use spawn to not block the Tauri app
        .map_err(|e| e.to_string())?;

    println!("Attempting to launch Steam with user: {}", username);
    Ok(())
}

// Helper function to get the path to accounts.json in the app's data directory
fn get_accounts_path(app_handle: &AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app_handle.path_resolver().app_data_dir()
        .ok_or_else(|| "Failed to get app data directory.".to_string())?;
    
    // Ensure the directory exists
    fs::create_dir_all(&app_data_dir)
        .map_err(|e| format!("Failed to create app data directory: {}", e))?;
        
    Ok(app_data_dir.join("accounts.json"))
}

// Helper function to get the path to notes.json
fn get_notes_path(app_handle: &AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app_handle.path_resolver().app_data_dir()
        .ok_or_else(|| "Failed to get app data directory.".to_string())?;
    
    // Ensure the directory exists
    fs::create_dir_all(&app_data_dir)
        .map_err(|e| format!("Failed to create app data directory: {}", e))?;
        
    Ok(app_data_dir.join("notes.json"))
}

// Helper function to save accounts to the JSON file
fn save_accounts_to_disk(app_handle: &AppHandle, accounts: &Vec<Account>) -> Result<(), String> {
    let account_file_path = get_accounts_path(app_handle)?;
    let json_data = serde_json::to_string_pretty(accounts)
        .map_err(|e| format!("Failed to serialize accounts: {}", e))?;
    fs::write(account_file_path, json_data)
        .map_err(|e| format!("Failed to write to accounts.json: {}", e))?;
    Ok(())
}

// Helper function to save notes to the JSON file
fn save_notes_to_disk(app_handle: &AppHandle, notes: &Vec<Note>) -> Result<(), String> {
    let notes_file_path = get_notes_path(app_handle)?;
    let json_data = serde_json::to_string_pretty(notes)
        .map_err(|e| format!("Failed to serialize notes: {}", e))?;
    fs::write(notes_file_path, json_data)
        .map_err(|e| format!("Failed to write to notes.json: {}", e))?;
    Ok(())
}

// Tauri command to add a new account
#[tauri::command]
fn add_account(app_handle: AppHandle, account: Account, state: State<AccountState>) -> Result<Vec<Account>, String> {
    let mut accounts = state.0.lock().unwrap();
    accounts.push(account);
    save_accounts_to_disk(&app_handle, &accounts)?;
    Ok(accounts.clone())
}

// Tauri command to update an existing account
#[tauri::command]
fn update_account(app_handle: AppHandle, index: usize, account: Account, state: State<AccountState>) -> Result<Vec<Account>, String> {
    let mut accounts = state.0.lock().unwrap();
    if index < accounts.len() {
        accounts[index] = account;
        save_accounts_to_disk(&app_handle, &accounts)?;
        Ok(accounts.clone())
    } else {
        Err("Account index out of bounds".to_string())
    }
}

// Tauri command to delete an account
#[tauri::command]
fn delete_account(app_handle: AppHandle, index: usize, state: State<AccountState>) -> Result<Vec<Account>, String> {
    let mut accounts = state.0.lock().unwrap();
    if index < accounts.len() {
        accounts.remove(index);
        save_accounts_to_disk(&app_handle, &accounts)?;
        Ok(accounts.clone())
    } else {
        Err("Account index out of bounds".to_string())
    }
}

// Tauri command to import accounts, overwriting existing ones
#[tauri::command]
fn import_accounts(app_handle: AppHandle, accounts: Vec<Account>, state: State<AccountState>) -> Result<Vec<Account>, String> {
    let mut state_accounts = state.0.lock().unwrap();
    *state_accounts = accounts.clone();
    save_accounts_to_disk(&app_handle, &state_accounts)?;
    Ok(accounts)
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

// --- Commands for Notes ---

#[tauri::command]
fn get_notes(state: State<NoteState>) -> Result<Vec<Note>, String> {
    let notes = state.0.lock().unwrap();
    Ok(notes.clone())
}

#[tauri::command]
fn add_note(app_handle: AppHandle, title: String, content: String, state: State<NoteState>) -> Result<Vec<Note>, String> {
    let mut notes = state.0.lock().unwrap();
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    
    let new_note = Note {
        id: Uuid::new_v4().to_string(),
        title,
        content,
        created_at: now,
        updated_at: now,
    };

    notes.push(new_note);
    save_notes_to_disk(&app_handle, &notes)?;
    Ok(notes.clone())
}

#[tauri::command]
fn update_note(app_handle: AppHandle, note: Note, state: State<NoteState>) -> Result<Vec<Note>, String> {
    let mut notes = state.0.lock().unwrap();
    if let Some(index) = notes.iter().position(|n| n.id == note.id) {
        notes[index] = note;
        save_notes_to_disk(&app_handle, &notes)?;
        Ok(notes.clone())
    } else {
        Err("Note not found".to_string())
    }
}

#[tauri::command]
fn delete_note(app_handle: AppHandle, id: String, state: State<NoteState>) -> Result<Vec<Note>, String> {
    let mut notes = state.0.lock().unwrap();
    if let Some(index) = notes.iter().position(|n| n.id == id) {
        notes.remove(index);
        save_notes_to_disk(&app_handle, &notes)?;
        Ok(notes.clone())
    } else {
        Err("Note not found".to_string())
    }
}

fn main() {
    tauri::Builder::default()
        // Manage an empty state initially. It will be populated by the setup task.
        .manage(AccountState(Mutex::new(Vec::new())))
        .manage(NoteState(Mutex::new(Vec::new())))
        .setup(|app| {
            let app_handle = app.handle();
            
            // Spawn the initialization task to run in the background.
            tauri::async_runtime::spawn(async move {
                // Run heavy async tasks in the background
                let _ = initialize_database().await;
                let _ = initialize_app_cache().await;

                // Load account data from JSON file
                let account_file_path = get_accounts_path(&app_handle)
                    .expect("Could not resolve accounts.json path at startup");

                let accounts: Vec<Account> = match fs::read_to_string(&account_file_path) {
                    Ok(data) => serde_json::from_str(&data).unwrap_or_else(|_| Vec::new()),
                    Err(_) => {
                        fs::write(&account_file_path, "[]").expect("Failed to create empty accounts.json");
                        Vec::new()
                    }
                };
                
                // Get the state and update it with the loaded accounts.
                let account_state = app_handle.state::<AccountState>();
                let mut state_accounts = account_state.0.lock().unwrap();
                *state_accounts = accounts;
                
                // Load notes data from JSON file
                let notes_file_path = get_notes_path(&app_handle)
                    .expect("Could not resolve notes.json path at startup");

                let notes: Vec<Note> = match fs::read_to_string(&notes_file_path) {
                    Ok(data) => serde_json::from_str(&data).unwrap_or_else(|_| Vec::new()),
                    Err(_) => {
                        fs::write(&notes_file_path, "[]").expect("Failed to create empty notes.json");
                        Vec::new()
                    }
                };

                let note_state = app_handle.state::<NoteState>();
                let mut state_notes = note_state.0.lock().unwrap();
                *state_notes = notes;
                
                // Signal to the frontend that all background initialization is complete.
                app_handle.emit_all("initialization_complete", ()).unwrap();
            });
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            search_games,
            search_game_by_name,
            get_game_details,
            fetch_game_name,
            download_game,
            restart_steam,
            initialize_database,
            check_steam_directories,
            get_library_games,
            update_game,
            remove_game,
            get_game_name_by_appid,
            list_downloaded_files,
            open_file_or_folder,
            save_settings,
            load_settings,
            commands::update_game_files,
            commands::get_dlcs_in_lua,
            commands::restart_steam,
            commands::install_steam_tools,
            commands::get_local_ip_address,
            commands::list_downloaded_files,
            commands::open_file_or_folder,
            commands::get_batch_game_details,
            commands::clear_details_cache,
            commands::sync_dlcs_in_lua,
            // Add the new commands here
            get_accounts,
            switch_steam_account,
            add_account,
            update_account,
            delete_account,
            import_accounts,
            // Notes Commands
            get_notes,
            add_note,
            update_note,
            delete_note
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
} 