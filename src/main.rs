// No black console window behind the app.
#![windows_subsystem = "windows"]

//! The window only. All device code lives in the library (providers, worker).
//! Look: dark "gaming" style with neon accents and glowing segmented bars.

use charge_deck::model::{Card, DeviceKind, Reading, Status};
use charge_deck::worker::{self, SharedState};
use eframe::egui::{self, Color32, FontId, RichText, Stroke, StrokeKind, Vec2};
use std::sync::mpsc::Sender;

// ---- Palette (all colours in one place, easy to tweak) ----
const BG: Color32 = Color32::from_rgb(10, 12, 18);
const CARD: Color32 = Color32::from_rgb(18, 22, 31);
const CARD_BORDER: Color32 = Color32::from_rgb(35, 42, 58);
const SEGMENT_OFF: Color32 = Color32::from_rgb(28, 34, 48);
const ACCENT: Color32 = Color32::from_rgb(0, 229, 255); // neon cyan
const TEXT: Color32 = Color32::from_rgb(230, 234, 242);
const DIM: Color32 = Color32::from_rgb(122, 132, 153);
const GREEN: Color32 = Color32::from_rgb(57, 255, 136);
const YELLOW: Color32 = Color32::from_rgb(255, 196, 0);
const RED: Color32 = Color32::from_rgb(255, 59, 92);

const SEGMENTS: usize = 25;

struct ChargeDeck {
    shared: SharedState,
    refresh: Sender<()>,
    always_on_top: bool,
}

impl ChargeDeck {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        apply_style(&cc.egui_ctx);
        let shared = SharedState::default();
        // The worker calls this after every update so the UI redraws at once.
        let ctx = cc.egui_ctx.clone();
        let refresh = worker::spawn(shared.clone(), move || ctx.request_repaint());
        ChargeDeck { shared, refresh, always_on_top: false }
    }
}

/// Dark theme with neon accents for buttons, checkboxes and hover states.
fn apply_style(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.panel_fill = BG;
    v.window_fill = BG;
    v.extreme_bg_color = Color32::from_rgb(6, 8, 12);
    v.override_text_color = Some(TEXT);
    v.selection.bg_fill = ACCENT.gamma_multiply(0.35);
    v.selection.stroke = Stroke::new(1.0, ACCENT);

    let radius = egui::CornerRadius::same(6);
    for w in [&mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active] {
        w.corner_radius = radius;
    }
    v.widgets.inactive.bg_fill = CARD;
    v.widgets.inactive.weak_bg_fill = CARD;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, CARD_BORDER);
    v.widgets.hovered.bg_fill = Color32::from_rgb(24, 32, 46);
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(24, 32, 46);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT);
    v.widgets.hovered.fg_stroke = Stroke::new(1.5, ACCENT);
    v.widgets.active.bg_fill = ACCENT.gamma_multiply(0.25);
    v.widgets.active.weak_bg_fill = ACCENT.gamma_multiply(0.25);
    v.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
    ctx.set_visuals(v);
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

        egui::Panel::top("header").frame(egui::Frame::new().fill(BG).inner_margin(egui::Margin::symmetric(14, 10))).show(
            ui,
            |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("CHARGE").font(FontId::monospace(22.0)).strong().color(TEXT));
                    ui.label(RichText::new("DECK").font(FontId::monospace(22.0)).strong().color(ACCENT));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let online = cards.iter().filter(|c| c.present).count();
                        ui.label(RichText::new(format!("{online} DEVICES")).font(FontId::monospace(12.0)).color(DIM));
                    });
                });
                // Neon underline.
                let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 2.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 1.0, ACCENT);
                ui.painter().rect_filled(rect.expand2(Vec2::new(0.0, 3.0)), 3.0, ACCENT.gamma_multiply(0.12));
            },
        );

        egui::Panel::bottom("footer").frame(egui::Frame::new().fill(BG).inner_margin(egui::Margin::symmetric(14, 8))).show(
            ui,
            |ui| {
                ui.horizontal(|ui| {
                    if ui.button(RichText::new("REFRESH").font(FontId::monospace(12.0)).strong().color(ACCENT)).clicked() {
                        let _ = self.refresh.send(());
                    }
                    // Plain window flag only: no overlay, no drawing inside games.
                    let label = RichText::new("ALWAYS ON TOP").font(FontId::monospace(12.0)).color(DIM);
                    if ui.checkbox(&mut self.always_on_top, label).changed() {
                        let level = if self.always_on_top {
                            egui::WindowLevel::AlwaysOnTop
                        } else {
                            egui::WindowLevel::Normal
                        };
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::WindowLevel(level));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let (text, color) = match (&last_updated, scanning) {
                            (_, true) => ("CHECKING…".to_string(), ACCENT),
                            (Some(t), false) => (format!("LAST UPDATED {t}"), DIM),
                            (None, false) => (String::new(), DIM),
                        };
                        ui.label(RichText::new(text).font(FontId::monospace(11.0)).color(color));
                    });
                });
            },
        );

        egui::CentralPanel::default().frame(egui::Frame::new().fill(BG).inner_margin(egui::Margin::symmetric(14, 10))).show(
            ui,
            |ui| {
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    if cards.is_empty() {
                        ui.add_space(30.0);
                        ui.vertical_centered(|ui| {
                            let msg = if scanning { "SCANNING FOR DEVICES…" } else { "NO SUPPORTED DEVICES FOUND" };
                            ui.label(RichText::new(msg).font(FontId::monospace(13.0)).color(DIM));
                        });
                    }
                    for card in &cards {
                        draw_card(ui, card);
                        ui.add_space(8.0);
                    }
                });
            },
        );

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
fn level_color(percent: u8) -> Color32 {
    match percent {
        51..=100 => GREEN,
        20..=50 => YELLOW,
        _ => RED,
    }
}

fn percent_text(r: &Reading) -> String {
    format!("{}{}%", if r.estimated { "≈" } else { "" }, r.percent)
}

/// Small rounded "chip" label such as CHARGING or 3.99 V.
fn chip(ui: &mut egui::Ui, text: &str, color: Color32) {
    egui::Frame::new()
        .fill(color.gamma_multiply(0.14))
        .stroke(Stroke::new(1.0, color.gamma_multiply(0.6)))
        .corner_radius(4)
        .inner_margin(egui::Margin::symmetric(6, 1))
        .show(ui, |ui| {
            ui.label(RichText::new(text).font(FontId::monospace(10.5)).strong().color(color));
        });
}

/// Segmented battery bar with a soft glow under the lit segments.
fn segment_bar(ui: &mut egui::Ui, percent: u8, color: Color32, lit: bool) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 12.0), egui::Sense::hover());
    let gap = 3.0;
    let seg_w = (rect.width() - gap * (SEGMENTS as f32 - 1.0)) / SEGMENTS as f32;
    let filled = ((percent as f32 / 100.0) * SEGMENTS as f32).ceil() as usize;
    let painter = ui.painter();

    if lit && filled > 0 {
        let glow_w = filled as f32 * (seg_w + gap) - gap;
        let glow = egui::Rect::from_min_size(rect.min, Vec2::new(glow_w, rect.height())).expand(3.0);
        painter.rect_filled(glow, 5.0, color.gamma_multiply(0.10));
    }
    for i in 0..SEGMENTS {
        let x = rect.min.x + i as f32 * (seg_w + gap);
        let seg = egui::Rect::from_min_size(egui::pos2(x, rect.min.y), Vec2::new(seg_w, rect.height()));
        let c = if lit && i < filled { color } else { SEGMENT_OFF };
        painter.rect_filled(seg, 2.0, c);
    }
}

fn draw_card(ui: &mut egui::Ui, card: &Card) {
    // The card border takes the battery colour, so low batteries stand out.
    let (accent, big_text) = match &card.device.status {
        Status::Ok(r) if card.present => (level_color(r.percent), Some(percent_text(r))),
        _ => (DIM, None),
    };
    let border = if card.present { accent.gamma_multiply(0.55) } else { CARD_BORDER };

    let response = egui::Frame::new()
        .fill(CARD)
        .stroke(Stroke::new(1.0, border))
        .corner_radius(8)
        .inner_margin(egui::Margin { left: 16, right: 12, top: 10, bottom: 10 })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            let dim_all = !card.present;
            ui.horizontal(|ui| {
                // Icon tile
                egui::Frame::new()
                    .fill(Color32::from_rgb(11, 14, 21))
                    .stroke(Stroke::new(1.0, CARD_BORDER))
                    .corner_radius(8)
                    .inner_margin(egui::Margin::same(6))
                    .show(ui, |ui| {
                        let c = if dim_all { DIM } else { accent };
                        ui.label(RichText::new(icon(card.device.kind)).size(22.0).color(c));
                    });
                ui.vertical(|ui| {
                    let name_color = if dim_all { DIM } else { TEXT };
                    ui.label(RichText::new(card.device.name.to_uppercase()).font(FontId::proportional(14.0)).strong().color(name_color));
                    ui.label(RichText::new(card.device.source.to_uppercase()).font(FontId::monospace(10.0)).color(if dim_all { DIM } else { ACCENT.gamma_multiply(0.8) }));
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(t) = &big_text {
                        ui.label(RichText::new(t).font(FontId::monospace(28.0)).strong().color(accent));
                    }
                });
            });
            ui.add_space(6.0);

            match &card.device.status {
                Status::Ok(r) => {
                    segment_bar(ui, r.percent, level_color(r.percent), card.present);
                    let mut chips: Vec<(String, Color32)> = Vec::new();
                    if r.charging {
                        chips.push(("CHARGING".into(), GREEN));
                    }
                    if let Some(mv) = r.voltage_mv {
                        chips.push((format!("{:.2} V", mv as f32 / 1000.0), ACCENT));
                    }
                    if r.estimated {
                        chips.push(("ESTIMATE".into(), DIM));
                    }
                    // No chips = no empty row under the bar.
                    if !chips.is_empty() {
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            for (text, color) in &chips {
                                chip(ui, text, *color);
                            }
                        });
                    }
                }
                Status::Asleep => {
                    segment_bar(ui, 0, DIM, false);
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        chip(ui, "ASLEEP / OFF", YELLOW);
                        if let Some(r) = &card.last_known {
                            chip(ui, &format!("LAST {}", percent_text(r)), DIM);
                        }
                    });
                }
                Status::Checking => {
                    segment_bar(ui, 0, DIM, false);
                    ui.add_space(6.0);
                    chip(ui, "CHECKING…", ACCENT);
                }
            }
            if !card.present {
                ui.add_space(4.0);
                ui.label(RichText::new(format!("LAST SEEN {}", card.last_seen)).font(FontId::monospace(10.0)).color(DIM));
            }
        });

    // Left accent stripe, painted over the card's left edge.
    let r = response.response.rect;
    let stripe = egui::Rect::from_min_max(r.min, egui::pos2(r.min.x + 4.0, r.max.y));
    let stripe_color = if card.present { accent } else { CARD_BORDER };
    ui.painter().rect(
        stripe,
        egui::CornerRadius { nw: 8, sw: 8, ne: 0, se: 0 },
        stripe_color,
        Stroke::NONE,
        StrokeKind::Inside,
    );
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Charge Deck")
            .with_inner_size([400.0, 600.0])
            .with_min_inner_size([320.0, 260.0]),
        ..Default::default()
    };
    eframe::run_native("Charge Deck", options, Box::new(|cc| Ok(Box::new(ChargeDeck::new(cc)))))
}
