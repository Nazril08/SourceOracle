# Library Commands

## Required Rust Commands

### `check_steam_directories`
Checks if the required Steam directories exist.

```rust
#[tauri::command]
async fn check_steam_directories(lua_path: String, manifest_path: String) -> Result<DirectoryStatus, String> {
    // Return type
    struct DirectoryStatus {
        lua: bool,
        manifest: bool,
    }
    
    // Check if directories exist
    Ok(DirectoryStatus {
        lua: std::path::Path::new(&lua_path).exists(),
        manifest: std::path::Path::new(&manifest_path).exists(),
    })
}
```

### `get_library_games`
Gets all games in the library by reading LUA and manifest files.

```rust
#[tauri::command]
async fn get_library_games(lua_dir: String, manifest_dir: String) -> Result<Vec<GameInfo>, String> {
    // Return type
    struct GameInfo {
        app_id: u32,
        name: String,
        lua_file: bool,
        manifest_file: bool,
    }
    
    // Implementation should:
    // 1. Read both directories
    // 2. Match files with game info
    // 3. Return combined information
}
```

### `update_game`
Updates game files in the library.

```rust
#[tauri::command]
async fn update_game(app_id: u32) -> Result<(), String> {
    // Implementation should:
    // 1. Download new files if available
    // 2. Update LUA and manifest files
    // 3. Return success or error
}
```

### `remove_game`
Removes game files from the library.

```rust
#[tauri::command]
async fn remove_game(app_id: u32) -> Result<(), String> {
    // Implementation should:
    // 1. Remove LUA file
    // 2. Remove manifest file
    // 3. Return success or error
}
```

## Directory Structure

```
C:\Program Files (x86)\Steam\config\
├── stplug-in\      # LUA files
│   ├── game1.lua
│   └── game2.lua
└── depotcache\     # Manifest files
    ├── game1.manifest
    └── game2.manifest
```

## Error Handling

The frontend expects errors in these formats:
1. Directory not found: "Steam directory not found: {path}"
2. File operation failed: "Failed to {operation} file: {error}"
3. Invalid file format: "Invalid {file_type} file format: {file}"

## File Naming Convention

- LUA files: `{app_id}.lua`
- Manifest files: `{app_id}.manifest` 