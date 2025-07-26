// This is a placeholder library file to satisfy Rust's requirement for a target.
// The actual functionality is in the Tauri app's src-tauri directory.

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
} 