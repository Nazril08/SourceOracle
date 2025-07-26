use std::sync::{Arc, Mutex};
use eframe::egui;
use egui::{Color32, RichText, ScrollArea, Ui, Rounding, Stroke, FontId, Vec2, Frame};
use poll_promise::Promise;

use crate::models::{AppState, DownloadStatus};
use crate::downloader;

// Define UI constants based on design.json
const PRIMARY_COLOR: Color32 = Color32::from_rgb(108, 99, 255); // #6C63FF - indigo-600
const BACKGROUND_COLOR: Color32 = Color32::from_rgb(14, 14, 26); // #0e0e1a - dark blue background
const SURFACE_COLOR: Color32 = Color32::from_rgb(22, 22, 42); // #16162a - card background
const SIDEBAR_COLOR: Color32 = Color32::from_rgb(18, 18, 30); // #12121e - sidebar background
const TEXT_PRIMARY: Color32 = Color32::from_rgb(255, 255, 255); // #FFFFFF
const TEXT_SECONDARY: Color32 = Color32::from_rgb(156, 163, 175); // #9ca3af - gray-400
const INPUT_BACKGROUND: Color32 = Color32::from_rgb(18, 18, 30); // #12121e
const INPUT_BORDER: Color32 = Color32::from_rgb(30, 30, 48); // #1e1e30
const HIGHLIGHT_COLOR: Color32 = Color32::from_rgb(79, 70, 229); // #4f46e5 - indigo-700
const SIDEBAR_ACTIVE: Color32 = Color32::from_rgb(30, 30, 54); // #1e1e36

// Navigation sections
#[derive(Debug, PartialEq, Clone, Copy)]
enum NavSection {
    Game,
    Settings,
}

pub struct OracleApp {
    state: Arc<Mutex<AppState>>,
    download_promise: Option<Promise<anyhow::Result<bool>>>,
    app_id_buffer: String,
    game_name_buffer: String,
    output_dir_buffer: String,
    fetch_name_promise: Option<Promise<Result<(), reqwest::Error>>>,
    current_section: NavSection,
    search_query: String,
}

impl Default for OracleApp {
    fn default() -> Self {
        let state = AppState::default();
        
        Self {
            app_id_buffer: state.app_id.clone(),
            game_name_buffer: state.game_name.clone(),
            output_dir_buffer: state.output_dir.clone(),
            state: Arc::new(Mutex::new(state)),
            download_promise: None,
            fetch_name_promise: None,
            current_section: NavSection::Game,
            search_query: String::new(),
        }
    }
}

impl eframe::App for OracleApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Apply theme to the entire UI
        let mut style = (*ctx.style()).clone();
        style.visuals.window_fill = BACKGROUND_COLOR;
        style.visuals.panel_fill = BACKGROUND_COLOR;
        style.visuals.widgets.noninteractive.bg_fill = BACKGROUND_COLOR;
        style.visuals.widgets.inactive.bg_fill = INPUT_BACKGROUND;
        style.visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
        style.visuals.widgets.active.bg_fill = PRIMARY_COLOR;
        style.visuals.widgets.active.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
        style.visuals.widgets.hovered.bg_fill = HIGHLIGHT_COLOR;
        style.visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
        
        // Update font settings
        style.text_styles = [
            (egui::TextStyle::Heading, FontId::new(24.0, egui::FontFamily::Proportional)),
            (egui::TextStyle::Body, FontId::new(14.0, egui::FontFamily::Proportional)),
            (egui::TextStyle::Monospace, FontId::new(14.0, egui::FontFamily::Monospace)),
            (egui::TextStyle::Button, FontId::new(14.0, egui::FontFamily::Proportional)),
            (egui::TextStyle::Small, FontId::new(12.0, egui::FontFamily::Proportional)),
        ].into();
        
        ctx.set_style(style);
        
        // Check if game name fetch is complete
        if let Some(promise) = &self.fetch_name_promise {
            if let Some(_result) = promise.ready() {
                // Get the latest game name after fetch is complete
                if let Ok(state) = self.state.lock() {
                    self.game_name_buffer = state.game_name.clone();
                    self.search_query = self.app_id_buffer.clone();
                }
                self.fetch_name_promise = None;
            }
        }
        
        // Check if download is complete
        if let Some(promise) = &self.download_promise {
            if let Some(result) = promise.ready() {
                let mut state = self.state.lock().unwrap();
                match result {
                    Ok(true) => {
                        state.download_status = DownloadStatus::Success;
                        state.log_messages.push("Download completed successfully!".to_string());
                    },
                    Ok(false) => {
                        state.download_status = DownloadStatus::Failed("No data found".to_string());
                        state.log_messages.push("Download process completed but no data was found.".to_string());
                    },
                    Err(e) => {
                        state.download_status = DownloadStatus::Failed(e.to_string());
                        state.log_messages.push(format!("Error during download process: {}", e));
                    },
                }
                self.download_promise = None;
            }
        }

        // Sidebar - modern and minimal
        egui::SidePanel::left("sidebar")
            .exact_width(180.0)
            .resizable(false)
            .frame(Frame::none().fill(SIDEBAR_COLOR))
            .show(ctx, |ui| {
                ui.add_space(24.0);
                ui.vertical_centered(|ui| {
                    ui.heading(RichText::new("Yeyo").size(28.0).color(PRIMARY_COLOR).strong());
                });
                ui.add_space(32.0);

                self.render_sidebar_navigation(ui);
                
                // Push the restart button to the bottom
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                    ui.add_space(16.0);
                    if ui.add(egui::Button::new(
                        RichText::new("🔄 Restart Steam").size(14.0).color(TEXT_PRIMARY))
                        .min_size(Vec2::new(150.0, 36.0))
                        .fill(PRIMARY_COLOR)
                        .rounding(Rounding::same(20.0))
                    ).clicked() {
                        // Restart Steam functionality would go here
                    }
                    ui.add_space(16.0);
                });
            });

        // Main content
        egui::CentralPanel::default()
            .frame(Frame::none().fill(BACKGROUND_COLOR))
            .show(ctx, |ui| {
                match self.current_section {
                    NavSection::Game => self.render_game_section(ui),
                    NavSection::Settings => self.render_settings_section(ui),
                }
        });

        // Request repaint if download is in progress
        if self.download_promise.is_some() || self.fetch_name_promise.is_some() {
            ctx.request_repaint();
        }
    }
}

impl OracleApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }

    fn render_sidebar_navigation(&mut self, ui: &mut Ui) {
        let button_size = Vec2::new(160.0, 40.0);
        
        // Game button
        let game_button = ui.add(
            egui::Button::new(
                RichText::new("🎮 Game")
                    .size(16.0)
                    .color(if self.current_section == NavSection::Game { TEXT_PRIMARY } else { TEXT_SECONDARY })
            )
            .min_size(button_size)
            .fill(if self.current_section == NavSection::Game { SIDEBAR_ACTIVE } else { SIDEBAR_COLOR })
            .rounding(Rounding::same(4.0))
        );
        
        if game_button.clicked() {
            self.current_section = NavSection::Game;
        }
        
        ui.add_space(4.0);
        
        // Settings button
        let settings_button = ui.add(
            egui::Button::new(
                RichText::new("⚙️ Settings")
                    .size(16.0)
                    .color(if self.current_section == NavSection::Settings { TEXT_PRIMARY } else { TEXT_SECONDARY })
            )
            .min_size(button_size)
            .fill(if self.current_section == NavSection::Settings { SIDEBAR_ACTIVE } else { SIDEBAR_COLOR })
            .rounding(Rounding::same(4.0))
        );
        
        if settings_button.clicked() {
            self.current_section = NavSection::Settings;
        }
    }
    
    fn render_game_section(&mut self, ui: &mut Ui) {
        ui.add_space(24.0);
        ui.horizontal(|ui| {
            ui.heading(RichText::new("Game List").size(28.0).color(TEXT_PRIMARY).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Theme toggle button
                if ui.add(egui::Button::new("🌙").min_size(Vec2::new(32.0, 32.0))
                    .fill(SIDEBAR_COLOR).rounding(Rounding::same(16.0))).clicked() {
                    // Theme toggle would go here
                }
            });
        });
        
        ui.add_space(24.0);
        
        // Search bar - modern design
        ui.horizontal(|ui| {
            ui.label(RichText::new("🔍").size(16.0).color(TEXT_SECONDARY));
            let search_response = ui.add(
                egui::TextEdit::singleline(&mut self.search_query)
                    .hint_text("Cari game berdasarkan nama atau AppID")
                    .desired_width(ui.available_width() - 100.0)
                    .text_color(TEXT_PRIMARY)
                    .margin(Vec2::new(8.0, 8.0))
                    .frame(true)
            );
            
            if ui.add(egui::Button::new(
                RichText::new("Cari").size(14.0).color(TEXT_PRIMARY))
                .min_size(Vec2::new(80.0, 32.0))
                .fill(PRIMARY_COLOR)
                .rounding(Rounding::same(4.0))
            ).clicked() || search_response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                if !self.search_query.is_empty() {
                    self.app_id_buffer = self.search_query.clone();
                    if let Ok(mut state) = self.state.lock() {
                        state.app_id = self.app_id_buffer.clone();
                    }
                    self.fetch_game_name();
                }
            }
        });
        
        ui.add_space(8.0);
        
        // Search tip
        ui.label(RichText::new("Tip: Anda dapat mencari berdasarkan nama game atau AppID (pisahkan dengan koma untuk mencari beberapa sekaligus)")
            .size(12.0).color(TEXT_SECONDARY).italics());
            
        ui.add_space(12.0);
        
        // Search results
        ui.label(RichText::new(format!("Hasil Pencarian: {} game ditemukan", 
            if self.game_name_buffer == "Dead by Daylight" || self.game_name_buffer.is_empty() { "0" } else { "1" }))
            .size(16.0).color(TEXT_PRIMARY));
            
        ui.add_space(16.0);
        
        // Game details card - glassmorphism style
        if self.game_name_buffer != "Dead by Daylight" && !self.game_name_buffer.is_empty() {
            // Create a card with glassmorphism effect
            Frame::none()
                .fill(SURFACE_COLOR)
                .rounding(Rounding::same(8.0))
                .stroke(Stroke::new(1.0, INPUT_BORDER))
                .inner_margin(16.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // Game icon placeholder
                        Frame::none()
                            .fill(SIDEBAR_COLOR)
                            .rounding(Rounding::same(8.0))
                            .show(ui, |ui| {
                                ui.add_sized([48.0, 48.0], 
                                    egui::Label::new(RichText::new("🎮").size(24.0).color(PRIMARY_COLOR)));
                            });
                            
                        ui.add_space(16.0);
                        
                        ui.vertical(|ui| {
                            ui.heading(RichText::new(&self.game_name_buffer).size(20.0).color(TEXT_PRIMARY));
                            ui.label(RichText::new(format!("AppID: {}", self.app_id_buffer)).color(TEXT_SECONDARY));
                            
                            ui.add_space(12.0);
                            
                            if ui.add(egui::Button::new(
                                RichText::new("Download").size(14.0).color(TEXT_PRIMARY))
                                .min_size(Vec2::new(100.0, 32.0))
                                .fill(PRIMARY_COLOR)
                                .rounding(Rounding::same(4.0))
                            ).clicked() {
                                self.start_download();
                            }
                        });
                    });
                });
        }
        
        // Pagination
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
            ui.add_space(20.0);
            ui.horizontal(|ui| {
                if ui.add_enabled(false, egui::Button::new(
                    RichText::new("« Previous").size(14.0).color(TEXT_SECONDARY))
                    .min_size(Vec2::new(100.0, 32.0))
                    .fill(SURFACE_COLOR)
                    .rounding(Rounding::same(4.0))
                ).clicked() {
                    // Previous page functionality would go here
                }
                
                ui.add_space(20.0);
                
                if ui.add_enabled(false, egui::Button::new(
                    RichText::new("Next »").size(14.0).color(TEXT_SECONDARY))
                    .min_size(Vec2::new(100.0, 32.0))
                    .fill(SURFACE_COLOR)
                    .rounding(Rounding::same(4.0))
                ).clicked() {
                    // Next page functionality would go here
                }
            });
        });
    }
    
    fn render_settings_section(&mut self, ui: &mut Ui) {
        ui.add_space(24.0);
        ui.heading(RichText::new("Settings").size(28.0).color(TEXT_PRIMARY).strong());
        ui.add_space(24.0);
        
        // Settings content
        Frame::none()
            .fill(SURFACE_COLOR)
            .rounding(Rounding::same(8.0))
            .stroke(Stroke::new(1.0, INPUT_BORDER))
            .inner_margin(24.0)
            .show(ui, |ui| {
                ui.label(RichText::new("Download Settings").size(18.0).color(TEXT_PRIMARY).strong());
                ui.add_space(16.0);
                
                // App ID
                ui.label(RichText::new("App ID:").color(TEXT_PRIMARY));
                let app_id_response = ui.add(
                    egui::TextEdit::singleline(&mut self.app_id_buffer)
                        .desired_width(300.0)
                        .text_color(TEXT_PRIMARY)
                        .margin(Vec2::new(8.0, 8.0))
                        .frame(true));
                        
                if app_id_response.changed() {
                    if let Ok(mut state) = self.state.lock() {
                        state.app_id = self.app_id_buffer.clone();
                    }
                    
                    // Fetch game name when App ID changes
                    self.fetch_game_name();
                }
                ui.add_space(12.0);
                
                // Game Name
                ui.label(RichText::new("Game Name:").color(TEXT_PRIMARY));
                ui.add(
                    egui::TextEdit::singleline(&mut self.game_name_buffer)
                        .desired_width(300.0)
                        .text_color(TEXT_SECONDARY)
                        .margin(Vec2::new(8.0, 8.0))
                        .frame(true)
                        .interactive(false)
                );
                ui.add_space(12.0);
                
                // Output Directory
                ui.label(RichText::new("Output Directory:").color(TEXT_PRIMARY));
                let output_dir_response = ui.add(
                    egui::TextEdit::singleline(&mut self.output_dir_buffer)
                        .desired_width(300.0)
                        .text_color(TEXT_PRIMARY)
                        .margin(Vec2::new(8.0, 8.0))
                        .frame(true));
                        
                if output_dir_response.changed() {
                    if let Ok(mut state) = self.state.lock() {
                        state.output_dir = self.output_dir_buffer.clone();
                    }
                }
                
                ui.add_space(24.0);
                
                // Download button
                ui.vertical_centered(|ui| {
                    let is_downloading = {
            let state = self.state.lock().unwrap();
                        state.download_status == DownloadStatus::Downloading
                    };
        
                    let button_text = match {
                        let state = self.state.lock().unwrap();
                        state.download_status.clone()
                    } {
            DownloadStatus::Idle => "Start Download",
            DownloadStatus::Downloading => "Downloading...",
            DownloadStatus::Success => "Download Again",
            DownloadStatus::Failed(_) => "Try Again",
        };
        
                    let button = ui.add_enabled(
                        !is_downloading,
                        egui::Button::new(
                            RichText::new(button_text)
                                .size(16.0)
                                .color(TEXT_PRIMARY)
                        )
                        .min_size(Vec2::new(180.0, 40.0))
                        .fill(PRIMARY_COLOR)
                        .rounding(Rounding::same(20.0))
                    );
                    
                    if button.clicked() {
            self.start_download();
        }
                });
            });
            
        // Log area
        ui.add_space(24.0);
        ui.label(RichText::new("Log Messages").size(16.0).color(TEXT_PRIMARY).strong());
        ui.add_space(8.0);
        
        // Create a card-like frame for log messages
        Frame::none()
            .fill(SURFACE_COLOR)
            .rounding(Rounding::same(8.0))
            .stroke(Stroke::new(1.0, INPUT_BORDER))
            .show(ui, |ui| {
            let state = self.state.lock().unwrap();
                let log_height = 180.0;
            
            ScrollArea::vertical()
                .auto_shrink([false, false])
                .stick_to_bottom(true)
                .max_height(log_height)
                .show(ui, |ui| {
                        ui.add_space(8.0);
                    for message in &state.log_messages {
                        let color = if message.contains("[ERROR]") || message.contains("FAIL") {
                            Color32::RED
                        } else if message.contains("SUCCESS") {
                                Color32::from_rgb(0, 180, 0)
                        } else if message.contains("[OK]") {
                            Color32::from_rgb(0, 180, 0)
                        } else {
                                TEXT_SECONDARY
                        };
                        
                            ui.horizontal(|ui| {
                                ui.add_space(12.0);
                        ui.label(RichText::new(message).color(color));
                            });
                    }
                        ui.add_space(8.0);
                });
        });
    }

    fn fetch_game_name(&mut self) {
        let state_clone = Arc::clone(&self.state);
        
        // Create promise for async fetch
        self.fetch_name_promise = Some(Promise::spawn_thread(
            "fetch_name_thread".to_string(),
            move || {
                // Create tokio runtime
                let rt = tokio::runtime::Runtime::new().unwrap();
                
                rt.block_on(async {
                    let mut state = state_clone.lock().unwrap();
                    state.fetch_game_name().await
                })
            }
        ));
    }

    fn start_download(&mut self) {
        let state_clone = Arc::clone(&self.state);
        
        // Ambil data yang diperlukan sebelum memperbarui status
        let app_id;
        let game_name;
        let output_dir;
        let repos;
        
        {
            let state = self.state.lock().unwrap();
            app_id = state.app_id.clone();
            game_name = state.game_name.clone();
            output_dir = state.output_dir.clone();
            repos = state.repos.clone();
        }
        
        // Update state to downloading
        {
            let mut state = self.state.lock().unwrap();
            state.download_status = DownloadStatus::Downloading;
            state.log_messages.clear();
            state.log_messages.push(format!("Starting download for {} (AppID: {})", game_name, app_id));
        }
        
        // Create promise for async download
        self.download_promise = Some(Promise::spawn_thread(
            "download_thread".to_string(),
            move || {
                // Menggunakan tokio runtime untuk menjalankan async code dalam thread
                let rt = tokio::runtime::Runtime::new().unwrap();
                
                rt.block_on(async {
                    // Start download
                    let result = downloader::download_from_repo(
                        &app_id,
                        &game_name,
                        &repos,
                        &output_dir,
                        &mut *state_clone.lock().unwrap()
                    ).await;
                    
                    // If download was successful, process the ZIP file
                    if let Ok(true) = &result {
                        let mut state = state_clone.lock().unwrap();
                        let zip_path = std::path::Path::new(&output_dir)
                            .join(format!("{} - {} (Branch).zip", 
                                sanitize_filename::sanitize(&game_name), 
                                app_id));
                        
                        if zip_path.exists() {
                            state.log_messages.push("Processing downloaded ZIP file...".to_string());
                            if let Err(e) = state.process_downloaded_zip(&zip_path) {
                                state.log_messages.push(format!("Error processing ZIP file: {}", e));
                            }
                        }
                    }
                    
                    result
                })
            }
        ));
    }
}

// Function to run the GUI
pub fn run_app() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1000.0, 700.0)),
        min_window_size: Some(egui::vec2(800.0, 600.0)),
        decorated: true,
        transparent: false,
        default_theme: eframe::Theme::Dark,
        ..Default::default()
    };
    
    eframe::run_native(
        "Yeyo - Oracle Downloader",
        options,
        Box::new(|cc| Box::new(OracleApp::new(cc)))
    )
} 