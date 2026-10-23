// No black console window behind the app.
#![windows_subsystem = "windows"]

//! The window only. All device code lives in the library (providers, worker).

use charge_deck::model::{Card, DeviceKind, Reading, Status};
use charge_deck::worker::{self, SharedState};
use eframe::egui::{self, Color32, RichText};
use std::sync::mpsc::Sender;

struct ChargeDeck {
    shared: SharedState,
    refresh: Sender<()>,
    always_on_top: bool,
}

impl ChargeDeck {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let shared = SharedState::default();
        // The worker calls this after every update so the UI redraws at once.
        let ctx = cc.egui_ctx.clone();
        let refresh = worker::spawn(shared.clone(), move || ctx.request_repaint());
        ChargeDeck { shared, refresh, always_on_top: false }
    }
}

impl eframe::App for ChargeDeck {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Copy what we need and release the lock before drawing,
        // so the worker thread is never blocked by the UI.
        let (cards, last_updated, scanning) = {
            let s = self.shared.lock().unwrap_or_else(|e| e.into_inner());
            let cards: Vec<Card> = s.deck.sorted().into_iter().cloned().collect();
            (cards, s.last_updated.clone(), s.scanning)
        };

        egui::Panel::bottom("footer").show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Refresh").clicked() {
                    let _ = self.refresh.send(());
                }
                // Plain window flag only: no overlay, no drawing inside games.
                if ui.checkbox(&mut self.always_on_top, "Always on top").changed() {
                    let level = if self.always_on_top {
                        egui::WindowLevel::AlwaysOnTop
                    } else {
                        egui::WindowLevel::Normal
                    };
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::WindowLevel(level));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let text = match (&last_updated, scanning) {
                        (_, true) => "Checking…".to_string(),
                        (Some(t), false) => format!("Last updated: {t}"),
                        (None, false) => String::new(),
                    };
                    ui.label(RichText::new(text).weak());
                });
            });
        });

        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                if cards.is_empty() {
                    ui.add_space(20.0);
                    ui.vertical_centered(|ui| {
                        ui.label(if scanning { "Looking for devices…" } else { "No supported devices found." });
                    });
                }
                for card in &cards {
                    draw_card(ui, card);
                }
            });
        });

        // Safety net: wake up now and then so the "Last updated" text stays fresh.
        ui.ctx().request_repaint_after(std::time::Duration::from_secs(30));
    }
}

fn icon(kind: DeviceKind) -> &'static str {
    match kind {
        DeviceKind::Keyboard => "⌨",
        DeviceKind::Mouse => "🖱",
        DeviceKind::Headset => "🎧",
        DeviceKind::Controller => "🎮",
        DeviceKind::Other => "🔋",
    }
}

/// green > 50 %, yellow 20-50 %, red < 20 %
fn bar_color(percent: u8) -> Color32 {
    match percent {
        51..=100 => Color32::from_rgb(70, 170, 90),
        20..=50 => Color32::from_rgb(220, 180, 40),
        _ => Color32::from_rgb(210, 70, 60),
    }
}

fn percent_text(r: &Reading) -> String {
    format!("{}{}%", if r.estimated { "≈" } else { "" }, r.percent)
}

fn draw_card(ui: &mut egui::Ui, card: &Card) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_width(ui.available_width());
        // Greyed out when the device disappeared.
        ui.add_enabled_ui(card.present, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(icon(card.device.kind)).size(26.0));
                ui.vertical(|ui| {
                    ui.label(RichText::new(&card.device.name).strong());
                    ui.label(RichText::new(card.device.source).weak().small());
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    match &card.device.status {
                        Status::Ok(r) => ui.label(RichText::new(percent_text(r)).size(20.0).strong()),
                        _ => ui.label(""),
                    };
                });
            });

            match &card.device.status {
                Status::Ok(r) => {
                    ui.add(
                        egui::ProgressBar::new(r.percent as f32 / 100.0)
                            .fill(bar_color(r.percent))
                            .desired_width(ui.available_width()),
                    );
                    let mut line = Vec::new();
                    if r.charging {
                        line.push("Charging".to_string());
                    }
                    if let Some(mv) = r.voltage_mv {
                        line.push(format!("{:.2} V", mv as f32 / 1000.0));
                    }
                    if !line.is_empty() {
                        ui.label(RichText::new(line.join(" · ")).small());
                    }
                }
                Status::Asleep => {
                    let last = card
                        .last_known
                        .as_ref()
                        .map(|r| format!(" · last {}", percent_text(r)))
                        .unwrap_or_default();
                    ui.label(RichText::new(format!("Asleep / off{last}")).small());
                }
                Status::Checking => {
                    ui.label(RichText::new("Checking…").small());
                }
            }
        });
        if !card.present {
            ui.label(RichText::new(format!("Last seen {}", card.last_seen)).small().weak());
        }
    });
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Charge Deck")
            .with_inner_size([380.0, 520.0])
            .with_min_inner_size([300.0, 240.0]),
        ..Default::default()
    };
    eframe::run_native("Charge Deck", options, Box::new(|cc| Ok(Box::new(ChargeDeck::new(cc)))))
}
