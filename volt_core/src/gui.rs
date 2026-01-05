use eframe::egui;
use std::sync::{Arc, Mutex};
use crate::chain::Blockchain;
use crate::node::Node;

pub struct VoltNodeApp {
    blockchain: Arc<Mutex<Blockchain>>,
    mining_enabled: Arc<Mutex<bool>>,
    logs: Arc<Mutex<Vec<String>>>,
    peers_count: Arc<Mutex<usize>>, // Shared with main to show updated peer count
    // Internal
    mined_count: u64,
}

impl VoltNodeApp {
    pub fn new(
        blockchain: Arc<Mutex<Blockchain>>,
        mining_enabled: Arc<Mutex<bool>>,
        logs: Arc<Mutex<Vec<String>>>,
        peers_count: Arc<Mutex<usize>>,
    ) -> Self {
        Self {
            blockchain,
            mining_enabled,
            logs,
            peers_count,
            mined_count: 0,
        }
    }
}

impl eframe::App for VoltNodeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Refresh Request
        ctx.request_repaint_after(std::time::Duration::from_millis(500));

        // Get Data
        let (height, difficulty, last_hash) = {
            let chain = self.blockchain.lock().unwrap();
            let last = chain.chain.last().unwrap();
            (chain.chain.len(), chain.difficulty, last.hash.clone())
        };
        
        let mut mining = self.mining_enabled.lock().unwrap();
        let peers = *self.peers_count.lock().unwrap();
        let logs = self.logs.lock().unwrap().clone(); // Clone for display safety

        egui::CentralPanel::default().show(ctx, |ui| {
            // HEADER
            ui.heading("⚡ Volt Node Control Panel");
            ui.separator();

            // DASHBOARD GRID
            ui.columns(2, |columns| {
                columns[0].vertical(|ui| {
                    ui.label(egui::RichText::new("Network Status").strong());
                    ui.label(format!("Height: {}", height));
                    ui.label(format!("Difficulty: {}", difficulty));
                    ui.label(format!("Peers: {}", peers));
                });
                columns[1].vertical(|ui| {
                    ui.label(egui::RichText::new("Mining Status").strong());
                    if *mining {
                        ui.label(egui::RichText::new("⛏️ MINING ACTIVE").color(egui::Color32::GREEN));
                    } else {
                        ui.label(egui::RichText::new("🛑 IDLE").color(egui::Color32::RED));
                    }
                    ui.label(format!("Last Block: {}...", &last_hash[0..10]));
                });
            });

            ui.add_space(20.0);
            
            // CONTROLS
            ui.horizontal(|ui| {
                if ui.button(if *mining { "Stop Mining" } else { "Start Mining" }).clicked() {
                    *mining = !*mining;
                }
            });

            ui.add_space(20.0);

            // CONSOLE LOGS
            ui.label("System Logs:");
            egui::ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
                ui.set_min_height(200.0);
                ui.set_max_width(f32::INFINITY);
                
                let text_style = egui::TextStyle::Monospace;
                
                for log in logs.iter().rev().take(50).rev() { // Show last 50
                    ui.colored_label(egui::Color32::LIGHT_GRAY, log);
                }
            });
        });
    }
}
