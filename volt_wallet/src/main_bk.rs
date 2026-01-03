#![windows_subsystem = "windows"] 

mod rpc;
use rpc::RpcClient;
use eframe::egui;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};
use serde_json::Value;
use qrcode::QrCode;

use std::collections::HashMap;

// --- Classic Professional Theme ---
// --- Premium Deep Night Theme ---
const COL_BG: egui::Color32 = egui::Color32::from_rgb(15, 23, 42);      // Slate 900
const COL_PANEL: egui::Color32 = egui::Color32::from_rgb(30, 41, 59);   // Slate 800
const COL_ACCENT: egui::Color32 = egui::Color32::from_rgb(59, 130, 246); // Blue 500
const COL_TEXT: egui::Color32 = egui::Color32::from_rgb(241, 245, 249); // Slate 100
const COL_SUBTEXT: egui::Color32 = egui::Color32::from_rgb(148, 163, 184); // Slate 400
const COL_DANGER: egui::Color32 = egui::Color32::from_rgb(239, 68, 68); // Red 500
const COL_SUCCESS: egui::Color32 = egui::Color32::from_rgb(16, 185, 129); // Emerald 500
const COL_CARD: egui::Color32 = egui::Color32::from_rgb(30, 41, 59);    // Slate 800
const COL_INPUT_BG: egui::Color32 = egui::Color32::from_rgb(15, 23, 42); // Slate 900

fn configure_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.visuals = egui::Visuals::dark();
    style.visuals.window_fill = COL_BG;
    style.visuals.panel_fill = COL_PANEL;
    
    style.visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, COL_TEXT);
    style.visuals.widgets.inactive.bg_fill = COL_PANEL; 
    
    style.visuals.widgets.active.bg_fill = COL_INPUT_BG;
    style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(50, 50, 55);
    style.visuals.selection.bg_fill = COL_ACCENT;
    
    style.visuals.widgets.noninteractive.rounding = egui::Rounding::same(8.0);
    style.visuals.widgets.inactive.rounding = egui::Rounding::same(8.0);
    style.visuals.widgets.hovered.rounding = egui::Rounding::same(8.0);
    style.visuals.widgets.active.rounding = egui::Rounding::same(8.0);
    
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.button_padding = egui::vec2(12.0, 8.0);
    style.spacing.text_edit_width = 280.0; 
    
    ctx.set_style(style);
}

// --- App ---

// Helper to load icon for Window Titlebar
// Helper to load icon for Window Titlebar
fn load_icon() -> egui::IconData {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::load_from_memory(include_bytes!("../../app_icon.png"))
            .expect("Failed to load icon")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };
    
    egui::IconData {
        rgba: icon_rgba,
        width: icon_width,
        height: icon_height,
    }
}

fn main() -> Result<(), eframe::Error> {
    env_logger::init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 650.0])
            .with_title("Volt Wallet")
            .with_min_inner_size([600.0, 450.0])
            .with_icon(load_icon()), // Set Window Icon
        ..Default::default()
    };
    
    let daemon = spawn_daemon();

    eframe::run_native("Volt Wallet", options, Box::new(|cc| {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        configure_theme(&cc.egui_ctx);
        let mut app = WalletApp::new(cc);
        app.daemon_process = daemon;
        Box::new(app)
    }))
}

// ... spawn_daemon ... 

// ... struct WalletApp fields ...

// Add logo_texture to WalletApp struct (Search and replace struct definition separate or just insert here if I had full context, but I have to be careful with replace_file_content context).
// Actually, I can't easily insert into the struct definition without seeing it all again or being very precise.
// Wait, I saw the struct definition in lines 115-155.
// I will split this into multiple replacement chunks for safety.

// Chunk 1: load_icon and main
// Chunk 2: Struct definition
// Chunk 3: WalletApp::new (initialization)
// Chunk 4: Sidebar render


fn spawn_daemon() -> Option<std::process::Child> {
    // Check if node is already running? (Simple check: try connect or just spawn and let it fail binding port)
    // Better: Spawn and detach.
    // We assume volt_core.exe is in the same directory.
    let path = "volt_core.exe";
    if std::path::Path::new(path).exists() {
        println!("Ref: Spawning volt_core daemon...");
        // Windows: CreateNoWindow flag to hide console?
        // Basic spawn for now.
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            std::process::Command::new(path)
                .args(["6000"]) // Default args (No Mining)
                .stdout(std::process::Stdio::null()) // Silence Output
                .stderr(std::process::Stdio::null()) // Silence Errors
                .creation_flags(CREATE_NO_WINDOW)
                .spawn().ok()
        }
        #[cfg(not(target_os = "windows"))]
        {
             std::process::Command::new("./volt_core")
                .args(["6000"])
                .spawn().ok()
        }
    } else {
        println!("Warning: volt_core.exe not found. Wallet running in remote mode?");
        None
    }
}

#[derive(PartialEq, Clone, Copy)]
enum Tab {
    Dashboard,
    Send,
    Receive,
    Contacts,
    Assets,
    Staking,
    Settings,
}

#[derive(PartialEq, Clone, Copy)]
enum SettingsTab {
    Connection,
    Security,
    Backup
}

struct WalletApp {
    selected_tab: Tab,
    balance: String,
    address: String,
    status: String,
    block_height: u64,
    pending_txs: u64, // Phase 12: For Fee Calc
    is_locked: bool,
    unlock_password: String,
    history: Vec<Value>,
    recent_blocks: Vec<Value>,
    send_recipient: String,
    send_amount: String,
    
    show_sync_window: bool,
    initial_sync_done: bool,
    selected_tx: Option<Value>,

    contacts: HashMap<String, String>, 
    contact_name_input: String,
    contact_addr_input: String,
    assets: HashMap<String, i64>,
    qr_texture: Option<egui::TextureHandle>,
    mnemonic_display: String,
    import_input: String,
    
    logo_texture: Option<egui::TextureHandle>, // Added Field

    // V2 Inputs
    issue_symbol: String,
    issue_supply: String,
    stake_amount: String,
    send_token: String,
    
    rx: Receiver<GuiMessage>,
    tx: Sender<BgMessage>,

    last_heartbeat: Instant,
    
    daemon_process: Option<std::process::Child>, // Process Handle

    // Settings
    peers: usize,
    settings_tab: SettingsTab,
    node_url_input: String,
}

enum GuiMessage {
    UpdateData { 
        balance: String, 
        address: String, 
        history: Vec<Value>, 
        blocks: Vec<Value>,
        status: String,
        block_height: u64,
        pending_txs: u64,
        is_locked: bool,

        assets: HashMap<String, i64>,
        peers: usize,
    },
    ShowMnemonic(String),
}

enum BgMessage {
    SendTransaction { recipient: String, amount: f64, fee: u64, token: String },
    EncryptWallet(String),
    UnlockWallet(String),
    LockWallet,
    GetMnemonic,
    ImportWallet(String),
    IssueToken { symbol: String, supply: u64 },
    Stake(u64),
    Unstake(u64),
    SetNodeUrl(String),
}

impl WalletApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let (tx_gui, rx_gui) = channel();
        let (tx_bg, rx_bg) = channel();

        let mut contacts = HashMap::new();
        if let Ok(data) = std::fs::read_to_string("contacts.json") {
             if let Ok(c) = serde_json::from_str(&data) { contacts = c; }
        }

        // Background Thread (Unchanged)
        // Background Thread
        thread::spawn(move || {
            let mut client = RpcClient::new("volt-core.zapto.org:6001");
            loop {
                // 1. Requests
                if let Ok(msg) = rx_bg.try_recv() {
                    match msg {
                        BgMessage::SendTransaction { recipient, amount, fee, token } => {
                            let params = serde_json::json!({ "recipient": recipient, "amount": amount, "fee": fee, "token": token });
                            let _ = client.call("send_transaction", Some(params)); 
                        },
                        BgMessage::EncryptWallet(pass) => {
                             let params = serde_json::json!({ "password": pass });
                             let _ = client.call("encrypt_wallet", Some(params));
                        },
                        BgMessage::UnlockWallet(pass) => {
                             let params = serde_json::json!({ "password": pass });
                             let _ = client.call("unlock_wallet", Some(params));
                        },
                        BgMessage::LockWallet => { let _ = client.call("lock_wallet", None); },
                        BgMessage::GetMnemonic => {
                             if let Ok(res) = client.call("get_mnemonic", None) {
                                  if let Some(data) = res.data {
                                       if let Some(m) = data.get("mnemonic") {
                                            let phrase = m.as_str().unwrap_or("Seed phrase not available (Legacy Wallet). \nCreate a new wallet to generate a seed.");
                                            tx_gui.send(GuiMessage::ShowMnemonic(phrase.to_string())).ok();
                                       }
                                  }
                             }
                        },

                        BgMessage::ImportWallet(phrase) => {
                             let params = serde_json::json!({ "mnemonic": phrase });
                             let _ = client.call("import_wallet", Some(params));
                        },
                        BgMessage::IssueToken { symbol, supply } => {
                             let params = serde_json::json!({ "token": symbol, "amount": supply });
                             let _ = client.call("issue_asset", Some(params));
                        },
                        BgMessage::Stake(amount) => {
                             let params = serde_json::json!({ "amount": amount });
                             let _ = client.call("stake", Some(params));
                        },
                        BgMessage::Unstake(amount) => {
                             let params = serde_json::json!({ "amount": amount });
                             let _ = client.call("unstake", Some(params));
                        },
                        BgMessage::SetNodeUrl(url) => {
                             client = RpcClient::new(&url);
                        }
                    }
                }

                // 2. Poll Data
                let mut status = "Disconnected".to_string();
                let mut address = "...".to_string();
                let mut balance = "0.00".to_string();
                let mut block_height = 0;
                let mut pending_txs = 0;

                let mut is_locked = false;
                let mut history_vec = Vec::new();
                let mut blocks_vec = Vec::new();

                let mut assets_map = HashMap::new();
                let mut peers_count = 0;

                if let Ok(res) = client.call("get_status", None) {
                     status = "Connected".to_string();
                     if let Some(data) = res.data {
                         if let Some(l) = data.get("locked") { is_locked = l.as_bool().unwrap_or(false); }
                     }

                     if !is_locked {
                        if let Ok(res) = client.call("get_address", None) {
                            if let Some(data) = res.data {
                                if let Some(a) = data.get("address") { address = a.as_str().unwrap_or("?").to_string(); }
                            }
                        }
                        let params = serde_json::json!({ "address": address });
                        if let Ok(res) = client.call("get_balance", Some(params.clone())) {
                            if let Some(data) = res.data {
                                 if let Some(b) = data.get("balance") {
                                     let val = b.as_f64().unwrap_or(0.0);
                                     balance = format!("{:.8}", val / 100_000_000.0);
                                 }
                            }
                        }
                        if let Ok(res) = client.call("get_history", Some(params.clone())) {
                            if let Some(data) = res.data {
                                if let Some(h) = data.get("history") {
                                    if let Some(arr) = h.as_array() { history_vec = arr.clone(); }
                                }
                            }
                        }

                        
                        // Get Peers
                        if let Ok(res) = client.call("get_peers", None) {
                             if let Some(data) = res.data {
                                 if let Some(c) = data.get("count") { peers_count = c.as_u64().unwrap_or(0) as usize; }
                             }
                        }

                        if let Ok(res) = client.call("get_assets", Some(params)) {
                            if let Some(data) = res.data {
                                if let Some(a) = data.get("assets") {
                                    if let Ok(map) = serde_json::from_value(a.clone()) { assets_map = map; }
                                }
                            }
                        }
                     } else {
                         address = "LOCKED".to_string();
                         balance = "---".to_string();
                     }

                     if let Ok(res) = client.call("get_blocks", Some(serde_json::json!({}))) {
                         if let Some(data) = res.data {
                             if let Some(b) = data.get("blocks") {
                                 if let Some(arr) = b.as_array() { blocks_vec = arr.clone(); }
                             }
                         }
                     }
                }
                
                if status == "Connected" {
                    if let Ok(res) = client.call("get_chain_info", None) {
                         if let Some(data) = res.data {
                             if let Some(h) = data.get("height") { block_height = h.as_u64().unwrap_or(0); }
                             if let Some(p) = data.get("pending_count") { pending_txs = p.as_u64().unwrap_or(0); }
                         }
                    }
                }

                tx_gui.send(GuiMessage::UpdateData { 
                    balance, address, history: history_vec, blocks: blocks_vec,
                    status, block_height, pending_txs, is_locked, assets: assets_map, peers: peers_count
                }).ok();
                thread::sleep(Duration::from_secs(1));
            }
        });
        
        // Load Logo Texture
        let logo_texture = {
            let image = image::load_from_memory(include_bytes!("../../app_icon.png")).unwrap().to_rgba8();
            let size = [image.width() as usize, image.height() as usize];
            let pixels = image.into_raw();
            let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
            Some(_cc.egui_ctx.load_texture("app_logo", color_image, egui::TextureOptions::LINEAR))
        };

        Self {
            logo_texture, // Init Field
            daemon_process: None,
            peers: 0,
            selected_tab: Tab::Dashboard,
            show_sync_window: true, 
            initial_sync_done: false,
            selected_tx: None,
            balance: "0.00".to_string(),
            address: "Loading...".to_string(),
            status: "Connecting...".to_string(),
            block_height: 0,
            pending_txs: 0,
            is_locked: false,
            unlock_password: String::new(),
            history: vec![],
            recent_blocks: vec![],
            send_recipient: String::new(),
            send_amount: String::new(),
            send_token: "VLT".to_string(),

            contacts,
            contact_name_input: String::new(),
            contact_addr_input: String::new(),
            assets: HashMap::new(),
            qr_texture: None,
            mnemonic_display: String::new(),
            import_input: String::new(),
            issue_symbol: String::new(),
            issue_supply: String::new(),
            stake_amount: String::new(),
            settings_tab: SettingsTab::Connection,
            node_url_input: "volt-core.zapto.org:6001".to_string(),
            rx: rx_gui,
            tx: tx_bg,
            last_heartbeat: Instant::now(),
        }
    }
    
    fn save_contacts(&self) {
        if let Ok(str) = serde_json::to_string_pretty(&self.contacts) {
            let _ = std::fs::write("contacts.json", str);
        }
    }

    fn generate_qr(&mut self, ctx: &egui::Context, content: &str) {
        if content == "Loading..." || content == "LOCKED" { return; }
        let code = QrCode::new(content).unwrap();
        let matrix = code.to_colors();
        let width = code.width();
        let height = width; 
        
        let pixels: Vec<egui::Color32> = matrix.iter().map(|c| {
            match c {
                qrcode::Color::Dark => egui::Color32::BLACK,
                qrcode::Color::Light => egui::Color32::WHITE,
            }
        }).collect();
        
        let texture = ctx.load_texture("qr_code", egui::ColorImage { size: [width, height], pixels }, egui::TextureOptions::NEAREST);
        self.qr_texture = Some(texture);
    }
}

impl Drop for WalletApp {
    fn drop(&mut self) {
        if let Some(mut child) = self.daemon_process.take() {
            println!("Stopping Volt Daemon...");
            let _ = child.kill();
        }
    }
}

impl eframe::App for WalletApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(msg) = self.rx.try_recv() {
             match msg {
                GuiMessage::UpdateData { balance, address, history, blocks, status, block_height, pending_txs, is_locked, assets, peers } => {
                    if self.address != address { self.qr_texture = None; }
                    self.balance = balance;
                    self.address = address;
                    self.history = history;
                    self.recent_blocks = blocks;
                    self.status = status;
                    self.block_height = block_height;
                    self.pending_txs = pending_txs;
                    self.is_locked = is_locked;
                    self.assets = assets;
                    self.peers = peers;
                    self.last_heartbeat = Instant::now();

                    // Auto-close sync window on first connection
                    if self.status == "Connected" && !self.initial_sync_done {
                        self.initial_sync_done = true;
                        self.show_sync_window = false;
                    }
                },
                GuiMessage::ShowMnemonic(m) => { self.mnemonic_display = m; }
             }
        }
        
        if self.last_heartbeat.elapsed() > Duration::from_secs(4) { self.status = "Connection Lost".to_string(); }
        ctx.request_repaint_after(Duration::from_secs(1));

        // --- BOTTOM STATUS BAR ---
        // --- TOP NAV BAR ---
        // --- LEFT SIDEBAR NAV ---
        egui::SidePanel::left("sidebar_panel")
            .resizable(false)
            .default_width(200.0)
            .frame(egui::Frame::none().fill(COL_PANEL).inner_margin(15.0).stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(51, 65, 85))))
            .show(ctx, |ui| {
                ui.add_space(10.0);
                ui.add_space(10.0);
                ui.vertical_centered(|ui| {
                     // Logo Image
                     if let Some(texture) = &self.logo_texture {
                         ui.image((texture.id(), egui::vec2(80.0, 80.0))); // Size 80x80
                     }
                     ui.add_space(10.0);
                     ui.heading(egui::RichText::new("VOLT").size(24.0).strong().color(COL_ACCENT));
                     ui.label(egui::RichText::new("The Future of Mining").size(10.0).color(COL_SUBTEXT)); // Updated subtitle
                });
                ui.add_space(30.0);

                let tabs = [
                    ("🏠 Dashboard", Tab::Dashboard),
                    ("💸 Send", Tab::Send),
                    ("📥 Receive", Tab::Receive),
                    ("👥 Contacts", Tab::Contacts),
                    ("💎 Assets", Tab::Assets),
                    ("🔒 Staking", Tab::Staking),
                ];
                
                for (label, tab) in tabs {
                    let selected = self.selected_tab == tab;
                    let text_color = if selected { egui::Color32::WHITE } else { COL_SUBTEXT };
                    let bg_color = if selected { COL_ACCENT } else { egui::Color32::TRANSPARENT };
                    
                    if ui.add(egui::Button::new(egui::RichText::new(label).color(text_color).size(16.0))
                           .fill(bg_color).min_size(egui::vec2(ui.available_width(), 40.0)).rounding(8.0)).clicked() {
                        self.selected_tab = tab;
                    }
                    ui.add_space(5.0);
                }
                
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                     ui.add_space(20.0);
                     let s_color = if self.status == "Connected" { COL_SUCCESS } else { COL_DANGER };
                     ui.label(egui::RichText::new(format!("● {}", self.status)).color(s_color).size(12.0));
                     ui.label(egui::RichText::new(format!("Peers: {}", self.peers)).color(COL_SUBTEXT).size(11.0));
                });
            });

        // Remove old top panel logic - replaced by Sidebar
        // (Top panel removed from render)

        // --- BOTTOM STATUS BAR ---
        egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(false)
            .min_height(35.0)
            .frame(egui::Frame::none().fill(egui::Color32::from_rgb(35, 35, 40)).inner_margin(8.0).stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(50, 50, 50))))
            .show(ctx, |ui| {
                 ui.horizontal(|ui| {
                     ui.add_space(10.0);
                     let s_color = if self.status == "Connected" { COL_SUCCESS } else { COL_DANGER };
                     ui.label(egui::RichText::new(format!("● Status: {}", self.status)).color(s_color).strong());
                     ui.separator();
                     ui.label(egui::RichText::new(format!("Height: #{}", self.block_height)).color(COL_TEXT));
                     
                     ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                         ui.add_space(10.0);
                         if !self.show_sync_window {
                             if ui.button(egui::RichText::new("Show Sync Info").size(12.0)).clicked() {
                                 self.show_sync_window = true;
                             }
                         }
                     });
                 });
        });

        // --- SYNC WINDOW POPUP ---
        if self.show_sync_window {
            egui::Window::new("Network Synchronization")
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .fixed_size([400.0, 250.0])
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(20.0);
                        if self.status == "Connected" {
                            ui.spinner();
                            ui.add_space(20.0);
                            ui.heading(egui::RichText::new("Synchronizing Blockchain...").color(COL_ACCENT));
                            ui.label("Downloading latest blocks from peers.");
                            ui.add_space(10.0);
                            ui.label(egui::RichText::new(format!("Current Height: {}", self.block_height)).size(18.0).strong());
                            
                            ui.add_space(30.0);
                            ui.label(egui::RichText::new("You can minimize this window and continue.").size(12.0).color(COL_SUBTEXT));
                        } else {
                            ui.heading(egui::RichText::new("Connecting to Network...").color(COL_DANGER));
                            ui.spinner();
                        }
                        
                        ui.add_space(30.0);
                        if ui.button(egui::RichText::new("Minimize & Continue").size(16.0)).clicked() {
                            self.show_sync_window = false;
                        }
                    });
                });
        }

        // --- TX DETAIL POPUP ---
        if let Some(tx) = self.selected_tx.clone() {
             let mut open = true;
             egui::Window::new("Transaction Details")
                 .open(&mut open)
                 .default_pos([400.0, 300.0])
                 .show(ctx, |ui| {
                      egui::Grid::new("tx_grid").striped(true).spacing([20.0, 10.0]).show(ui, |ui| {
                          ui.label("Amount:");
                          let amt_val = tx["amount"].as_f64().unwrap_or(0.0);
                          ui.label(egui::RichText::new(format!("{:.8} VLT", amt_val / 100_000_000.0)).strong().color(COL_ACCENT));
                          ui.end_row();

                          ui.label("Sender:");
                          ui.monospace(format!("{}", tx["sender"]).replace("\"", ""));
                          ui.end_row();

                          ui.label("Receiver:");
                          ui.monospace(format!("{}", tx["receiver"]).replace("\"", ""));
                          ui.end_row();
                          
                          ui.label("Block Height:");
                          ui.monospace(format!("{}", tx["block_index"]));
                          ui.end_row();

                          ui.label("Timestamp:");
                          let ts = tx["timestamp"].as_u64().unwrap_or(0);
                          // formatting timestamp requires chrono generally, keeping raw for now or simple
                          ui.monospace(format!("{}", ts)); 
                          ui.end_row();
                          
                          ui.label("Signature:");
                          // Truncate signature for display
                          let sig = format!("{}", tx["signature"]).replace("\"", "");
                          let short_sig = if sig.len() > 30 { format!("{}...", &sig[..30]) } else { sig };
                          ui.monospace(short_sig);
                          ui.end_row();
                      });
                      ui.add_space(20.0);
                      if ui.button("Close").clicked() {
                          self.selected_tx = None;
                      }
                 });
             if !open { self.selected_tx = None; }
        }

        // --- CENTRAL PANEL ---

        egui::CentralPanel::default().show(ctx, |ui| {
            // Header with Settings Icon
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                     if ui.add(egui::Button::new(egui::RichText::new("⚙").size(32.0)).frame(false)).clicked() {
                         self.selected_tab = Tab::Settings;
                     }
                });
            });

            if self.is_locked && self.selected_tab != Tab::Settings {
                 ui.centered_and_justified(|ui| {
                      ui.vertical_centered(|ui| {
                          ui.label(egui::RichText::new("🔒").size(60.0).color(COL_SUBTEXT));
                          ui.add_space(10.0);
                          ui.heading(egui::RichText::new("Wallet Locked").size(24.0).color(COL_TEXT));
                          ui.add_space(20.0);
                          if ui.add(egui::Button::new("Unlock Wallet").min_size(egui::vec2(200.0, 45.0)).fill(COL_ACCENT)).clicked() {
                              self.selected_tab = Tab::Settings;
                          }
                      });
                 });
                 return;
            }

            ui.add_space(20.0);
            
            match self.selected_tab {
                Tab::Dashboard => {
                     // Balance Card
                     egui::Frame::none().fill(COL_CARD).rounding(10.0).inner_margin(30.0).show(ui, |ui| {
                         ui.set_width(ui.available_width());
                         ui.vertical_centered(|ui| {
                             ui.label(egui::RichText::new("Total Balance").color(COL_SUBTEXT).size(16.0));
                             ui.add_space(5.0);
                             ui.label(egui::RichText::new(format!("{} VLT", self.balance)).size(48.0).strong().color(COL_TEXT));
                         });
                     });
                     
                     // Assets & Status
                     ui.add_space(15.0);
                     // Assets Only
                     ui.add_space(15.0);
                     ui.group(|ui| {
                          ui.vertical_centered(|ui| {
                             ui.label(egui::RichText::new("Other Assets").color(COL_SUBTEXT));
                             ui.add_space(5.0);
                             if self.assets.is_empty() {
                                 ui.label("None");
                             } else {
                                 for (t, b) in &self.assets {
                                     if t != "VLT" { ui.label(format!("{} {}", b, t)); }
                                 }
                             }
                          });
                     });

                     ui.add_space(25.0);
                     ui.heading("Recent Activity");
                     egui::Frame::none().fill(COL_CARD).rounding(8.0).inner_margin(10.0).show(ui, |ui| {
                         ui.set_width(ui.available_width());
                         if self.history.is_empty() {
                             ui.vertical_centered(|ui| { ui.label("No transactions yet."); });
                         } else {
                             for (i, tx) in self.history.iter().take(5).enumerate() {
                                 let receiver = tx["receiver"].as_str().unwrap_or("?");
                                 let amt = tx["amount"].as_f64().unwrap_or(0.0);
                                 let is_in = receiver == self.address;
                                 
                                 let resp = ui.push_id(i, |ui| {
                                     egui::Frame::none().fill(COL_CARD).rounding(5.0).inner_margin(10.0).show(ui, |ui| {
                                         ui.set_width(ui.available_width());
                                         ui.horizontal(|ui| {
                                             ui.label(egui::RichText::new(if is_in { "⬇" } else { "⬆" }).color(if is_in {COL_SUCCESS} else {COL_DANGER}));
                                             ui.label(egui::RichText::new(if is_in { "Received" } else { "Sent" }).strong());
                                             ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                  ui.label(egui::RichText::new(format!("{:.8} VLT", amt / 100_000_000.0)).strong().color(if is_in {COL_SUCCESS} else {COL_DANGER}));
                                             });
                                         });
                                     }).response
                                 }).inner;
                                 
                                 if resp.interact(egui::Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                     self.selected_tx = Some(tx.clone());
                                 }
                                 
                                 ui.add_space(5.0);
                             }
                         }
                     });
                },
                Tab::Send => {
                      ui.heading("Send Transaction");
                      ui.add_space(10.0);
                      egui::Frame::none().fill(COL_CARD).rounding(10.0).inner_margin(30.0).show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            // Removed Grid to allow full expansion
                                ui.label(egui::RichText::new("Recipient:").size(16.0));
                                ui.add(egui::TextEdit::singleline(&mut self.send_recipient)
                                    .hint_text("Paste address (e.g. Vlt...)")
                                    .desired_width(f32::INFINITY)); // Stretches to container

                                ui.add_space(15.0);

                                ui.label(egui::RichText::new("Asset:").size(16.0));
                                egui::ComboBox::from_id_source("token_selector")
                                    .selected_text(&self.send_token)
                                    .width(200.0) // Reasonable width
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut self.send_token, "VLT".to_string(), "VLT (Volt Coin)");
                                        for k in self.assets.keys() {
                                            if k != "VLT" {
                                                ui.selectable_value(&mut self.send_token, k.clone(), k);
                                            }
                                        }
                                    });

                                ui.add_space(15.0);
                                
                                ui.label(egui::RichText::new("Amount:").size(16.0));
                                ui.add(egui::TextEdit::singleline(&mut self.send_amount)
                                   .hint_text("0.00")
                                   .desired_width(f32::INFINITY));

                                ui.add_space(15.0);

                                // Phase 12: Dynamic Fee Calc
                                let amt = self.send_amount.parse::<f64>().unwrap_or(0.0);
                                let atomic_amount = (amt * 100_000_000.0) as u64;
                                
                                let base_fee = atomic_amount / 1000; // 0.1%
                                let congestion_surcharge = self.pending_txs * 100_000_000; // 1 VLT per tx
                                let calc_fee = base_fee + congestion_surcharge;
                                
                                let base_min = 100_000; // 0.001 VLT
                                let fee = if calc_fee < base_min { base_min } else { calc_fee };
                                
                                ui.horizontal(|ui| {
                                     ui.label("Est. Fee:");
                                     ui.label(egui::RichText::new(format!("{:.8} VLT", fee as f64 / 100_000_000.0)).color(COL_SUBTEXT));
                                });
                            
                            ui.add_space(30.0);
                            ui.horizontal(|ui| {
                                ui.add_space(95.0); // Offset to align with inputs
                                // Recalculate fee for button action scope or move logic out
                                let amt = self.send_amount.parse::<f64>().unwrap_or(0.0);
                                let atomic_amount = (amt * 100_000_000.0) as u64; 
                                let base_fee = atomic_amount / 1000;
                                let congestion_surcharge = self.pending_txs * 100_000_000;
                                let calc = base_fee + congestion_surcharge;
                                let base_min = 100_000;
                                let final_fee = if calc < base_min { base_min } else { calc };

                                if ui.add(egui::Button::new(egui::RichText::new("Confirm & Send").size(16.0)).fill(COL_ACCENT).min_size(egui::vec2(150.0, 45.0))).clicked() {
                                     if let Ok(a) = self.send_amount.parse::<f64>() {
                                          if a > 0.0 {
                                             // Send atomic amount
                                             self.tx.send(BgMessage::SendTransaction{ recipient: self.send_recipient.clone(), amount: atomic_amount as f64, fee: final_fee, token: self.send_token.clone() }).unwrap();
                                             
                                             // Optimistic UI Update: Deduct instantly
                                             if self.send_token == "VLT" {
                                                  if let Ok(current_bal) = self.balance.parse::<f64>() {
                                                      let total_deduct = a + (final_fee as f64 / 100_000_000.0);
                                                      let new_bal = current_bal - total_deduct;
                                                      self.balance = format!("{:.8}", if new_bal < 0.0 { 0.0 } else { new_bal });
                                                  }
                                             }

                                             self.send_amount.clear();
                                             self.send_recipient.clear();
                                             self.selected_tab = Tab::Dashboard;
                                          }
                                     }
                                }
                            });
                      });
                      
                      // Quick Contacts
                      ui.add_space(20.0);
                      if !self.contacts.is_empty() {
                          ui.label("Quick Send (Contacts):");
                          ui.horizontal_wrapped(|ui| {
                              for (name, addr) in &self.contacts {
                                  if ui.button(name).clicked() { self.send_recipient = addr.clone(); }
                              }
                          });
                      }
                },
                Tab::Receive => {
                     ui.heading("Receive VLT");
                     ui.add_space(20.0);
                     ui.vertical_centered(|ui| {
                         egui::Frame::none().fill(egui::Color32::WHITE).rounding(10.0).inner_margin(10.0).show(ui, |ui| {
                             if self.qr_texture.is_none() && self.address.len() > 10 {
                                 self.generate_qr(ctx, &self.address.clone());
                             }
                             if let Some(texture) = &self.qr_texture {
                                 ui.image((texture.id(), egui::vec2(220.0, 220.0)));
                             } else {
                                 ui.spinner();
                             }
                         });
                         ui.add_space(20.0);
                         ui.label("Your Address:");
                         ui.add(egui::TextEdit::singleline(&mut self.address.clone()).font(egui::TextStyle::Monospace).desired_width(500.0));
                         ui.label(egui::RichText::new("Share this address to receive funds.").color(COL_SUBTEXT));
                     });
                },
                Tab::Contacts => {
                     ui.heading("Contacts");
                     
                     // Add New Form
                     egui::Frame::none().fill(COL_CARD).rounding(8.0).inner_margin(20.0).show(ui, |ui| {
                         ui.label(egui::RichText::new("Add New Contact").strong());
                         ui.add_space(10.0);
                         ui.horizontal(|ui| {
                             ui.add(egui::TextEdit::singleline(&mut self.contact_name_input).hint_text("Name").desired_width(150.0));
                             ui.add(egui::TextEdit::singleline(&mut self.contact_addr_input).hint_text("Wallet Address").desired_width(350.0));
                             if ui.button("Save").clicked() {
                                 if !self.contact_name_input.is_empty() && !self.contact_addr_input.is_empty() {
                                     self.contacts.insert(self.contact_name_input.clone(), self.contact_addr_input.clone());
                                     self.contact_name_input.clear();
                                     self.contact_addr_input.clear();
                                     self.save_contacts();
                                 }
                             }
                         });
                     });
                     
                     ui.add_space(20.0);
                     ui.label("Saved:");
                     egui::ScrollArea::vertical().show(ui, |ui| {
                         // FIX: Clone to allow mutation
                         let items: Vec<(String, String)> = self.contacts.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
                         for (name, addr) in items {
                             ui.group(|ui| {
                                 ui.horizontal(|ui| {
                                     ui.label(egui::RichText::new(&name).size(16.0).strong());
                                     ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                          if ui.button("🗑").clicked() {
                                              self.contacts.remove(&name);
                                              self.save_contacts();
                                          }
                                          ui.monospace(&addr);
                                     });
                                 });
                             });
                         }
                     });
                },
                Tab::Assets => {
                     ui.heading("My Assets");
                     ui.add_space(10.0);
                     
                     // Issue Form
                     egui::Frame::none().fill(COL_CARD).rounding(10.0).inner_margin(20.0).show(ui, |ui| {
                         ui.label(egui::RichText::new("Issue New Token").size(18.0).strong().color(COL_ACCENT));
                         ui.add_space(15.0);
                         
                         egui::Grid::new("issue_grid").spacing([20.0, 15.0]).show(ui, |ui| {
                             ui.label("Token Symbol:");
                             ui.add(egui::TextEdit::singleline(&mut self.issue_symbol).hint_text("e.g. GOLD").desired_width(120.0));
                             ui.end_row();
                             
                             ui.label("Initial Supply:");
                             ui.add(egui::TextEdit::singleline(&mut self.issue_supply).hint_text("e.g. 1000000").desired_width(200.0));
                             ui.end_row();
                         });
                         
                         ui.add_space(20.0);
                         if ui.add(egui::Button::new("Issue Token").min_size(egui::vec2(120.0, 35.0)).fill(COL_ACCENT)).clicked() {
                             let sym = self.issue_symbol.trim().to_uppercase();
                             if let Ok(sup) = self.issue_supply.parse::<u64>() {
                                 if sym.len() >= 3 && sym.len() <= 8 && sup > 0 {
                                     self.tx.send(BgMessage::IssueToken { symbol: sym, supply: sup }).unwrap();
                                     self.issue_symbol.clear();
                                     self.issue_supply.clear();
                                     // Optional: Show success toast/modal?
                                 }
                             }
                         }
                         ui.label(egui::RichText::new("Fee: 0 VLT").size(10.0).color(COL_SUBTEXT));
                     });
                     
                     ui.add_space(20.0);
                     ui.label("Your Tokens:");
                     // Reuse dashboard assets logic or similar
                     if self.assets.is_empty() {
                         ui.label("No assets found.");
                     } else {
                         for (sym, bal) in &self.assets {
                             if sym != "VLT" {
                                  ui.group(|ui| {
                                      ui.horizontal(|ui| {
                                          ui.label(egui::RichText::new(sym).strong().size(16.0));
                                           ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                               // Assuming other assets are also atomic? If not, this might be formatted wrong.
                                               // For safety, let's treat custom tokens as raw units for now or consistent?
                                               // User likely wants consistency. Let's assume all tokens are 8-decimals.
                                               let val = *bal as f64;
                                               ui.label(egui::RichText::new(format!("{:.8}", val / 100_000_000.0)).strong());
                                           });
                                      });
                                  });
                             }
                         }
                     }
                },
                Tab::Staking => {
                     ui.heading("Staking Station");
                     ui.add_space(10.0);
                     
                     ui.label("Lock your VLT coins to help secure the network and earn rewards.");
                     
                     egui::Frame::none().fill(COL_CARD).rounding(10.0).inner_margin(20.0).show(ui, |ui| {
                          ui.horizontal(|ui| {
                               ui.label("Available to Stake:");
                               ui.label(egui::RichText::new(format!("{} VLT", self.balance)).strong().color(COL_SUCCESS));
                          });
                          ui.add_space(15.0);
                          
                          ui.horizontal(|ui| {
                              ui.label("Amount:");
                              ui.add(egui::TextEdit::singleline(&mut self.stake_amount).desired_width(150.0));
                              if ui.button("Max").clicked() {
                                  self.stake_amount = self.balance.clone();
                              }
                          });
                          
                          ui.add_space(20.0);
                          ui.columns(2, |columns| {
                              columns[0].vertical_centered(|ui| {
                                  ui.label("Action: Stake to Network");
                                  ui.add_space(5.0);
                                  if ui.add(egui::Button::new("🔒 STAKE").min_size(egui::vec2(120.0, 40.0)).fill(COL_ACCENT)).clicked() {
                                       if let Ok(amt) = self.stake_amount.parse::<f64>() {
                                            if amt > 0.0 {
                                                let atomic = (amt * 100_000_000.0) as u64;
                                                self.tx.send(BgMessage::Stake(atomic)).unwrap();
                                                self.stake_amount.clear();
                                            }
                                       }
                                  }
                              });
                              columns[1].vertical_centered(|ui| {
                                  ui.label("Action: Withdraw Funds");
                                  ui.add_space(5.0);
                                  if ui.add(egui::Button::new("🔓 UNSTAKE").min_size(egui::vec2(120.0, 40.0)).fill(COL_DANGER)).clicked() {
                                       if let Ok(amt) = self.stake_amount.parse::<f64>() {
                                            if amt > 0.0 {
                                                let atomic = (amt * 100_000_000.0) as u64;
                                                self.tx.send(BgMessage::Unstake(atomic)).unwrap();
                                                self.stake_amount.clear();
                                            }
                                       }
                                  }
                              });
                          });
                     });
                     
                     ui.add_space(20.0);
                     ui.label(egui::RichText::new("Note: Unstaking returns funds after verification.").color(COL_SUBTEXT).size(12.0));
                },

                Tab::Settings => {
                     ui.heading("Settings");
                     ui.separator();
                     ui.add_space(20.0);
                     
                     ui.columns(2, |cols| {
                         // Left Column: Navigation
                         cols[0].vertical(|ui| {
                             ui.set_max_width(150.0);
                             let btn_size = egui::vec2(ui.available_width(), 40.0);
                             
                             if ui.add_sized(btn_size, egui::Button::new("📡 Connection").fill(if self.settings_tab == SettingsTab::Connection { COL_ACCENT } else { egui::Color32::from_rgb(30,30,30) })).clicked() { self.settings_tab = SettingsTab::Connection; }
                             ui.add_space(5.0);
                             if ui.add_sized(btn_size, egui::Button::new("🔒 Security").fill(if self.settings_tab == SettingsTab::Security { COL_ACCENT } else { egui::Color32::from_rgb(30,30,30) })).clicked() { self.settings_tab = SettingsTab::Security; }
                             ui.add_space(5.0);
                             if ui.add_sized(btn_size, egui::Button::new("💾 Backup").fill(if self.settings_tab == SettingsTab::Backup { COL_ACCENT } else { egui::Color32::from_rgb(30,30,30) })).clicked() { self.settings_tab = SettingsTab::Backup; }
                         });

                         // Right Column: Content
                         cols[1].vertical(|ui| {
                             egui::Frame::none().fill(COL_PANEL).rounding(5.0).inner_margin(20.0).show(ui, |ui| {
                                 ui.set_width(ui.available_width());
                                 match self.settings_tab {
                                     SettingsTab::Connection => {
                                         ui.heading("Connection Settings");
                                         ui.add_space(10.0);
                                         ui.label(egui::RichText::new("Light Wallet Configuration").strong());
                                         ui.label("Connect to a remote or local Volt Node.");
                                         ui.add_space(15.0);
                                         
                                         ui.label("Node URL:");
                                         ui.horizontal(|ui| {
                                             ui.add(egui::TextEdit::singleline(&mut self.node_url_input).hint_text("127.0.0.1:6001").desired_width(250.0));
                                             if ui.button("Connect").clicked() {
                                                  let url = self.node_url_input.trim().to_string();
                                                  if !url.is_empty() {
                                                      self.tx.send(BgMessage::SetNodeUrl(url)).unwrap();
                                                      self.status = "Connecting...".to_string();
                                                  }
                                             }
                                         });
                                         
                                         ui.add_space(15.0);
                                         ui.group(|ui| {
                                             ui.label(egui::RichText::new("Common Nodes:").strong());
                                             ui.label("- Localhost: 127.0.0.1:6001");
                                             ui.label("- Public Node 1: 85.x.x.x:6001");
                                         });
                                     },
                                     SettingsTab::Security => {
                                         ui.heading("Security Settings");
                                         ui.add_space(10.0);
                                         
                                         if self.is_locked {
                                              ui.label("Wallet is currently LOCKED.");
                                              ui.add_space(10.0);
                                              ui.label("Enter Password to Unlock:");
                                              ui.add(egui::TextEdit::singleline(&mut self.unlock_password).password(true).desired_width(300.0));
                                              if ui.button("Unlock").clicked() {
                                                  self.tx.send(BgMessage::UnlockWallet(self.unlock_password.clone())).unwrap();
                                                  self.unlock_password.clear();
                                              }
                                         } else {
                                              ui.label("Wallet is UNLOCKED.");
                                              ui.add_space(10.0);
                                              if ui.button("Lock Wallet Now").clicked() { self.tx.send(BgMessage::LockWallet).unwrap(); }
                                              
                                              ui.separator();
                                              ui.label("Change Password / Encrypt:");
                                              ui.horizontal(|ui| {
                                                  ui.add(egui::TextEdit::singleline(&mut self.unlock_password).password(true).hint_text("New Password"));
                                                  if ui.button("Encrypt").clicked() {
                                                      self.tx.send(BgMessage::EncryptWallet(self.unlock_password.clone())).unwrap();
                                                      self.unlock_password.clear();
                                                  }
                                              });
                                         }
                                     },
                                     SettingsTab::Backup => {
                                         ui.heading("Backup & Recovery");
                                         ui.add_space(10.0);
                                         
                                         if self.is_locked {
                                             ui.label(egui::RichText::new("⚠️ Authenticate (Unlock) to access backup.").color(COL_DANGER));
                                         } else {
                                             ui.label("Your Seed Phrase is the only way to recover funds.");
                                             ui.add_space(10.0);
                                             
                                             ui.horizontal(|ui| {
                                                 if ui.button("Show Seed Phrase").clicked() { self.tx.send(BgMessage::GetMnemonic).unwrap(); }
                                                 if !self.mnemonic_display.is_empty() {
                                                     if ui.button("Hide").clicked() { self.mnemonic_display.clear(); }
                                                 }
                                             });

                                             if !self.mnemonic_display.is_empty() {
                                                 ui.add_space(10.0);
                                                 egui::Frame::none().fill(egui::Color32::from_rgb(20, 20, 25)).rounding(5.0).inner_margin(10.0).show(ui, |ui| {
                                                     ui.add(egui::TextEdit::multiline(&mut self.mnemonic_display)
                                                         .font(egui::TextStyle::Monospace)
                                                         .desired_rows(3)
                                                         .desired_width(f32::INFINITY));
                                                         
                                                     ui.add_space(5.0);
                                                     if ui.button("📋 Copy to Clipboard").clicked() {
                                                         ui.output_mut(|o| o.copied_text = self.mnemonic_display.clone());
                                                     }
                                                 });
                                             }
                                             
                                             ui.separator();
                                             ui.label("Import Wallet:");
                                             ui.horizontal(|ui| {
                                                 ui.add(egui::TextEdit::singleline(&mut self.import_input).hint_text("12 words...").desired_width(250.0));
                                                 if ui.button("Restore").clicked() {
                                                     self.tx.send(BgMessage::ImportWallet(self.import_input.clone())).unwrap();
                                                 }
                                             });
                                         }
                                     }
                                 }
                             });
                         });
                     });
                }
            }
        });
    }
}
