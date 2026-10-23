// No black console window behind the app.
#![windows_subsystem = "windows"]

//! The window only. All device code lives in the library (providers, worker).
//! Look: follows the "Charge Deck" design canvas (see theme.rs and icons.rs).

mod icons;
mod theme;

use charge_deck::model::{Card, DeviceKind, Status};
use charge_deck::worker::{self, SharedState};
use eframe::egui::{self, Align, Color32, Layout, Margin, Rect, RichText, Sense, Shape, Stroke, Ui, pos2, vec2};
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};
use theme::*;

struct ChargeDeck {
    shared: SharedState,
    refresh: Sender<()>,
    always_on_top: bool,
}

impl ChargeDeck {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::install(&cc.egui_ctx);
        let shared = SharedState::default();
        // Design preview: fake devices, no scanning, no hardware access.
        if let Some(mode) = std::env::var_os("CHARGE_DECK_DEMO") {
            let mut s = shared.lock().unwrap_or_else(|e| e.into_inner());
            if mode != "empty" {
                s.deck = demo_deck();
            }
            s.last_updated = Some("2:32 PM".into());
            s.next_scan = Some(Instant::now() + Duration::from_secs(108));
            drop(s);
            let (tx, _rx) = std::sync::mpsc::channel();
            return ChargeDeck { shared, refresh: tx, always_on_top: false };
        }
        // The worker calls this after every update so the UI redraws at once.
        let ctx = cc.egui_ctx.clone();
        let refresh = worker::spawn(shared.clone(), move || ctx.request_repaint());
        ChargeDeck { shared, refresh, always_on_top: false }
    }
}

/// Fake devices that show every card state (for `CHARGE_DECK_DEMO=1`).
fn demo_deck() -> charge_deck::model::Deck {
    use charge_deck::model::{Deck, DeviceBattery, Reading};
    let dev = |id: &str, name: &str, kind, source, status| DeviceBattery {
        id: id.into(),
        name: name.into(),
        kind,
        source,
        status,
    };
    let ok = |p, charging| Status::Ok(Reading::exact(p, charging));
    let mut est = Reading::estimated(47, false);
    est.voltage_mv = Some(3860);
    let mut deck = Deck::default();
    // First scan: everything was awake (so "asleep" and "disconnected" have a last reading).
    deck.apply(
        "demo",
        vec![
            dev("mouse", "ROG Keris II Ace", DeviceKind::Mouse, "ASUS receiver", ok(14, false)),
            dev("kb", "ROG Strix Scope II 96 Wireless", DeviceKind::Keyboard, "ASUS receiver", ok(88, true)),
            dev("hs", "Razer Barracuda X", DeviceKind::Headset, "Razer dongle", Status::Ok(est)),
            dev("sleepy", "ROG Mouse", DeviceKind::Mouse, "ASUS receiver", ok(66, false)),
            dev("gone", "Old Headset", DeviceKind::Headset, "Razer dongle", ok(40, false)),
        ],
        "1:05 PM",
    );
    // Second scan: one device is asleep, one vanished, one is still being checked.
    let mut again: Vec<DeviceBattery> = deck.cards.iter().filter(|c| c.device.id != "gone").map(|c| c.device.clone()).collect();
    for d in again.iter_mut() {
        if d.id == "sleepy" {
            d.status = Status::Asleep;
        }
    }
    again.push(dev("bt", "Bluetooth device", DeviceKind::Other, "Bluetooth", Status::Checking));
    deck.apply("demo", again, "2:32 PM");
    deck
}

/// Everything one card needs to draw itself, worked out from the model.
struct CardView {
    kind: DeviceKind,
    name: String,
    source: String,
    present: bool,
    prefix: &'static str,
    number: String,
    unit: &'static str,
    color: Color32,
    sub: Option<String>,
    status: String,
    status_color: Color32,
    charging: bool,
    asleep: bool,
    edge: Color32,
    segments: [Color32; 10],
}

fn lit_segments(percent: u8, on: Color32) -> [Color32; 10] {
    let filled = (percent as f32 / 10.0).round() as usize;
    std::array::from_fn(|i| if i < filled { on } else { SEG_OFF })
}

fn view(card: &Card) -> CardView {
    let mut v = CardView {
        kind: card.device.kind,
        name: card.device.name.clone(),
        source: card.device.source.to_uppercase(),
        present: card.present,
        prefix: "",
        number: "—".into(),
        unit: "",
        color: MUTED,
        sub: None,
        status: String::new(),
        status_color: MUTED,
        charging: false,
        asleep: false,
        edge: CARD_EDGE,
        segments: [SEG_OFF; 10],
    };

    if !card.present {
        // Disconnected this session: grey, with the last reading and time.
        v.status = format!("Last seen {}", card.last_seen);
        if let Some(r) = &card.last_known {
            v.prefix = if r.estimated { "≈" } else { "" };
            v.number = r.percent.to_string();
            v.unit = "%";
            v.segments = lit_segments(r.percent, SEG_GREY);
        }
        return v;
    }

    match &card.device.status {
        Status::Ok(r) => {
            let low = r.percent < 20;
            v.prefix = if r.estimated { "≈" } else { "" };
            v.number = r.percent.to_string();
            v.unit = "%";
            v.color = level_color(r.percent);
            v.sub = r.voltage_mv.map(|mv| format!("{:.2} V", mv as f32 / 1000.0));
            v.charging = r.charging;
            v.segments = lit_segments(r.percent, v.color);
            if r.charging {
                v.status = "Charging".into();
                v.status_color = GREEN;
            } else if low {
                v.status = "Low — charge soon".into();
                v.status_color = LOW;
            } else {
                v.status = "Connected".into();
            }
            if low {
                v.edge = RED;
            }
        }
        Status::Asleep => {
            v.asleep = true;
            v.status = "Asleep / off".into();
            v.sub = card.last_known.as_ref().map(|r| format!("last {}{}%", if r.estimated { "~" } else { "" }, r.percent));
        }
        Status::Checking => {
            v.number = "··".into();
            v.color = Color32::from_rgb(0x5B, 0x62, 0x70);
            v.status = "Checking…".into();
            v.segments = std::array::from_fn(|i| if (3..=5).contains(&i) { SEG_SCAN } else { SEG_OFF });
        }
    }
    v
}

impl eframe::App for ChargeDeck {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Copy what we need and release the lock before drawing,
        // so the worker thread is never blocked by the UI.
        let (cards, last_updated, scanning, next_scan) = {
            let s = self.shared.lock().unwrap_or_else(|e| e.into_inner());
            let cards: Vec<Card> = s.deck.sorted().into_iter().cloned().collect();
            (cards, s.last_updated.clone(), s.scanning, s.next_scan)
        };

        egui::Panel::top("titlebar").frame(egui::Frame::new().fill(BAR_BG)).show(ui, |ui| title_bar(ui));

        egui::Panel::bottom("footer")
            .frame(egui::Frame::new().fill(BAR_BG).inner_margin(Margin::symmetric(20, 10)))
            .show(ui, |ui| self.footer(ui, last_updated.as_deref(), scanning));

        egui::CentralPanel::default().frame(egui::Frame::new().fill(BG).inner_margin(Margin::symmetric(20, 0))).show(
            ui,
            |ui| {
                if cards.is_empty() {
                    self.empty_state(ui, scanning);
                } else {
                    self.device_list(ui, &cards, scanning, next_scan);
                }
            },
        );

        // 1 px window outline (the window has no native frame).
        let outline = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::NULL.with("outline")));
        outline.rect_stroke(ui.ctx().content_rect(), 0.0, Stroke::new(1.0, CARD_EDGE), egui::StrokeKind::Inside);

        // Keep the "Next scan" countdown ticking; otherwise wake up rarely.
        let wake = if next_scan.is_some() { Duration::from_secs(1) } else { Duration::from_secs(30) };
        ui.ctx().request_repaint_after(wake);
    }
}

impl ChargeDeck {
    fn device_list(&mut self, ui: &mut Ui, cards: &[Card], scanning: bool, next_scan: Option<Instant>) {
        ui.add_space(22.0);
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                tracked(ui, "WIRELESS GEAR", display(12.0), RED_SOFT, 3.0);
                tracked(ui, "DEVICES", display(32.0), TEXT, 1.0);
                let need = cards.iter().filter(|c| c.present && matches!(&c.device.status, Status::Ok(r) if r.percent < 20)).count();
                let present = cards.iter().filter(|c| c.present).count();
                let summary = format!("{present} devices · {}", if need > 0 { format!("{need} needs charging") } else { "all good".into() });
                ui.label(RichText::new(summary).size(13.0).color(MUTED));
            });
            ui.with_layout(Layout::right_to_left(Align::BOTTOM), |ui| {
                let label = if scanning { "SCANNING" } else { "REFRESH" };
                if red_button(ui, label, true, 132.0, 44.0).clicked() && !scanning {
                    let _ = self.refresh.send(());
                }
            });
        });
        ui.add_space(14.0);

        // Stat tiles
        let lowest = cards
            .iter()
            .filter(|c| c.present)
            .filter_map(|c| match &c.device.status {
                Status::Ok(r) => Some(r.percent),
                _ => None,
            })
            .min();
        let charging = cards.iter().filter(|c| c.present && matches!(&c.device.status, Status::Ok(r) if r.charging)).count();
        let next = match (scanning, next_scan) {
            (false, Some(t)) => {
                let s = t.saturating_duration_since(Instant::now()).as_secs();
                format!("{}:{:02}", s / 60, s % 60)
            }
            _ => "…".into(),
        };
        let tile_w = (ui.available_width() - 16.0) / 3.0;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            stat_tile(ui, tile_w, "LOWEST", &lowest.map(|p| format!("{p}%")).unwrap_or_else(|| "—".into()), RED, RED_SOFT);
            stat_tile(ui, tile_w, "CHARGING", &charging.to_string(), STAT_TOP, TEXT);
            stat_tile(ui, tile_w, "NEXT SCAN", &next, STAT_TOP, TEXT);
        });
        ui.add_space(16.0);

        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            // `cards` is already sorted with disconnected devices last.
            let (live, gone): (Vec<&Card>, Vec<&Card>) = cards.iter().partition(|c| c.present);
            for card in live {
                draw_card(ui, &view(card));
                ui.add_space(10.0);
            }
            if !gone.is_empty() {
                ui.add_space(8.0);
                tracked(ui, "DISCONNECTED THIS SESSION", display(12.0), MUTED, 2.5);
                ui.add_space(10.0);
                for card in gone {
                    draw_card(ui, &view(card));
                    ui.add_space(10.0);
                }
            }
        });
    }

    fn footer(&mut self, ui: &mut Ui, last_updated: Option<&str>, scanning: bool) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                let first = match (scanning, last_updated) {
                    (true, _) => "Scanning…".to_string(),
                    (false, Some(t)) => format!("Last updated {t}"),
                    (false, None) => "Not scanned yet".to_string(),
                };
                ui.label(RichText::new(first).size(12.0).color(TEXT_SOFT));
                ui.label(RichText::new("Auto-refresh every 2 min").size(12.0).color(MUTED));
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                // Plain window flag only: no overlay, no drawing inside games.
                let label = RichText::new("Always on top").size(12.0).strong().color(TEXT_SOFT);
                if ui.checkbox(&mut self.always_on_top, label).changed() {
                    let level = if self.always_on_top { egui::WindowLevel::AlwaysOnTop } else { egui::WindowLevel::Normal };
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::WindowLevel(level));
                }
            });
        });
    }

    fn empty_state(&mut self, ui: &mut Ui, scanning: bool) {
        ui.add_space(ui.available_height() * 0.1);
        ui.vertical_centered(|ui| {
            // Badge
            let (rect, _) = ui.allocate_exact_size(vec2(94.0, 94.0), Sense::hover());
            ui.painter().add(chamfer_shape(rect, 18.0, RED));
            ui.painter().add(chamfer_shape(rect.shrink(1.0), 17.5, CARD));
            icons::battery(ui.painter(), Rect::from_center_size(rect.center(), vec2(44.0, 44.0)), RED);
            ui.add_space(22.0);

            let (eyebrow, title, body) = if scanning {
                ("SCANNING", "LOOKING FOR DEVICES", "Checking Bluetooth, controllers and USB receivers…")
            } else {
                (
                    "SCAN COMPLETE",
                    "NO DEVICES FOUND YET",
                    "Turn your wireless devices on and plug in their USB receivers, then scan again.",
                )
            };
            tracked(ui, eyebrow, display(12.0), RED_SOFT, 3.0);
            ui.add_space(8.0);
            tracked(ui, title, display(28.0), TEXT, 1.0);
            ui.add_space(8.0);
            ui.scope(|ui| {
                ui.set_max_width(340.0);
                ui.label(RichText::new(body).size(14.0).color(MUTED));
            });
            if !scanning {
                ui.add_space(22.0);
                if red_button(ui, "SCAN AGAIN", true, 168.0, 46.0).clicked() {
                    let _ = self.refresh.send(());
                }
            }
            ui.add_space(30.0);
            chamfer_box(ui, Some(CARD_EDGE), CARD, 14.0, Margin::symmetric(18, 16), |ui| {
                ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
                    tracked(ui, "CHARGE DECK READS FROM", display(13.0), MUTED, 2.0);
                    ui.add_space(10.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                        for name in ["BLUETOOTH", "GAME CONTROLLERS", "ASUS ROG", "RAZER", "LOGITECH"] {
                            chip(ui, name, TEXT_SOFT);
                        }
                    });
                    ui.add_space(10.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        let soft = |t: &str| RichText::new(t.to_string()).size(13.0).color(MUTED);
                        ui.label(soft("Device not listed? Run"));
                        ui.label(RichText::new("charge-deck-probe.exe").monospace().size(12.0).color(TEXT).background_color(BG));
                        ui.label(soft("and share its output in a GitHub issue."));
                    });
                });
            });
        });
    }
}

// ---- Pieces ----

fn title_bar(ui: &mut Ui) {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 40.0), Sense::click_and_drag());
    let ctx = ui.ctx().clone();
    // The window has no native title bar, so this strip drags the window.
    if resp.drag_started() {
        ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }
    // Use what the window reports; if it reports nothing, use what we last asked for.
    let memory_id = egui::Id::NULL.with("maximized");
    let remembered = ctx.data(|d| d.get_temp::<bool>(memory_id)).unwrap_or(false);
    let maximized = ui.input(|i| i.viewport().maximized).unwrap_or(remembered);
    let toggle_maximize = |ctx: &egui::Context| {
        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
        ctx.data_mut(|d| d.insert_temp(memory_id, !maximized));
    };
    if resp.double_clicked() {
        toggle_maximize(&ctx);
    }
    let p = ui.painter().clone();
    p.line_segment([rect.left_bottom(), rect.right_bottom()], Stroke::new(1.0, BAR_LINE));

    // Logo badge
    let badge = Rect::from_min_size(pos2(rect.min.x + 14.0, rect.center().y - 12.0), vec2(24.0, 24.0));
    p.add(chamfer_shape(badge, 6.0, RED));
    icons::bolt(&p, Rect::from_center_size(badge.center(), vec2(14.0, 14.0)), BG);

    // Wordmark
    let word = layout_tracked(ui, "CHARGE DECK", display(15.0), TEXT, 2.5);
    let word_pos = pos2(badge.max.x + 10.0, rect.center().y - word.size.y / 2.0);
    paint_tracked(ui, word_pos, &word);

    // Three slanted slashes
    let mut x = word_pos.x + word.size.x + 14.0;
    for alpha in [1.0, 0.55, 0.25] {
        let y = rect.center().y;
        let lean = 4.0 * 30f32.to_radians().tan() / 2.0;
        let pts = vec![pos2(x + lean, y - 2.0), pos2(x + 10.0 + lean, y - 2.0), pos2(x + 10.0 - lean, y + 2.0), pos2(x - lean, y + 2.0)];
        p.add(Shape::convex_polygon(pts, RED.gamma_multiply(alpha), Stroke::NONE));
        x += 13.0;
    }

    // Window buttons: minimize, maximize, close
    for (n, which) in [(2.0f32, 0u8), (1.0, 1), (0.0, 2)] {
        let b = Rect::from_min_size(pos2(rect.max.x - 46.0 * (n + 1.0), rect.min.y), vec2(46.0, 40.0));
        let r = ui.interact(b, egui::Id::NULL.with("winbtn").with(which), Sense::click());
        if r.hovered() {
            let hover = if which == 2 { Color32::from_rgb(0xC4, 0x2B, 0x3A) } else { BAR_LINE };
            p.rect_filled(b, 0.0, hover);
        }
        let color = if r.hovered() && which == 2 { TEXT } else { MUTED };
        icons::window_control(&p, Rect::from_center_size(b.center(), vec2(14.0, 14.0)), which, color);
        if r.clicked() {
            match which {
                0 => ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true)),
                1 => toggle_maximize(&ctx),
                _ => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            }
        }
    }
}

/// The red chamfered action button.
fn red_button(ui: &mut Ui, label: &str, with_icon: bool, width: f32, height: f32) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    let fill = if resp.is_pointer_button_down_on() {
        Color32::from_rgb(0xD9, 0x25, 0x3C)
    } else if resp.hovered() {
        RED_HOVER
    } else {
        RED
    };
    ui.painter().add(chamfer_shape(rect, 10.0, fill));
    let text = layout_tracked(ui, label, display(15.0), Color32::WHITE, 1.5);
    let icon_w = if with_icon { 16.0 + 8.0 } else { 0.0 };
    let start = rect.center().x - (icon_w + text.size.x) / 2.0;
    if with_icon {
        icons::refresh(ui.painter(), Rect::from_center_size(pos2(start + 8.0, rect.center().y), vec2(16.0, 16.0)), Color32::WHITE);
    }
    paint_tracked(ui, pos2(start + icon_w, rect.center().y - text.size.y / 2.0), &text);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn stat_tile(ui: &mut Ui, width: f32, label: &str, value: &str, top: Color32, value_color: Color32) {
    let (rect, _) = ui.allocate_exact_size(vec2(width, 62.0), Sense::hover());
    ui.painter().rect_filled(rect, 0.0, CARD);
    ui.painter().rect_filled(Rect::from_min_size(rect.min, vec2(width, 2.0)), 0.0, top);
    let l = layout_tracked(ui, label, display(11.0), MUTED, 1.5);
    paint_tracked(ui, pos2(rect.min.x + 12.0, rect.min.y + 10.0), &l);
    let v = layout_tracked(ui, value, display(22.0), value_color, 0.0);
    paint_tracked(ui, pos2(rect.min.x + 12.0, rect.min.y + 28.0), &v);
}

/// Small slanted-corner label, e.g. a source or a brand name.
fn chip(ui: &mut Ui, text: &str, color: Color32) {
    let t = layout_tracked(ui, text, display(11.0), color, 1.0);
    let size = t.size + vec2(16.0, 6.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter().add(chamfer_shape(rect, 5.0, CHIP));
    paint_tracked(ui, rect.min + vec2(8.0, 3.0), &t);
}

fn draw_card(ui: &mut Ui, v: &CardView) {
    ui.scope(|ui| {
        if !v.present {
            ui.set_opacity(0.5);
        }
        chamfer_box(ui, Some(v.edge), CARD, 14.0, Margin::symmetric(16, 14), |ui| {
            let full = ui.available_width();
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 14.0;
                // Icon tile
                let (tile, _) = ui.allocate_exact_size(vec2(56.0, 56.0), Sense::hover());
                ui.painter().add(chamfer_shape(tile, 10.0, TILE));
                icons::device(ui.painter(), Rect::from_center_size(tile.center(), vec2(34.0, 34.0)), Some(v.kind), TEXT_SOFT);

                // Name, source chip, status
                ui.vertical(|ui| {
                    ui.set_max_width((full - 56.0 - 14.0 * 2.0 - 96.0).max(60.0));
                    ui.add(egui::Label::new(RichText::new(&v.name).font(semibold(18.0)).color(TEXT)).truncate());
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        chip(ui, &v.source, CHIP_TEXT);
                        status_line(ui, v);
                    });
                });

                // Number block, right aligned
                ui.with_layout(Layout::top_down(Align::RIGHT), |ui| {
                    number_block(ui, v);
                });
            });
            ui.add_space(12.0);
            segment_bar(ui, &v.segments);
        });
    });
}

fn status_line(ui: &mut Ui, v: &CardView) {
    if v.status.is_empty() {
        return;
    }
    let icon_rect = |ui: &mut Ui, size: f32| ui.allocate_exact_size(vec2(size, size), Sense::hover()).0;
    ui.spacing_mut().item_spacing.x = 5.0;
    if v.charging {
        let r = icon_rect(ui, 13.0);
        icons::bolt(ui.painter(), r, v.status_color);
    } else if v.asleep {
        let r = icon_rect(ui, 13.0);
        icons::moon(ui.painter(), r, v.status_color);
    } else {
        // small diamond
        let r = icon_rect(ui, 8.0);
        let c = r.center();
        let pts = vec![pos2(c.x, c.y - 4.0), pos2(c.x + 4.0, c.y), pos2(c.x, c.y + 4.0), pos2(c.x - 4.0, c.y)];
        ui.painter().add(Shape::convex_polygon(pts, v.status_color, Stroke::NONE));
    }
    tracked(ui, &v.status.to_uppercase(), display(12.0), v.status_color, 0.8);
}

fn number_block(ui: &mut Ui, v: &CardView) {
    // "≈" comes from the default font: Rajdhani draws it like a plain "=".
    let prefix = layout_tracked(ui, v.prefix, egui::FontId::proportional(18.0), MUTED, 0.0);
    let number = layout_tracked(ui, &v.number, display(34.0), v.color, 0.0);
    let unit = layout_tracked(ui, v.unit, display(16.0), v.color, 0.0);
    let gap = 2.0;
    let width = prefix.size.x + number.size.x + unit.size.x + gap * 2.0;
    let (rect, _) = ui.allocate_exact_size(vec2(width, number.size.y), Sense::hover());
    // Bottom-align the three pieces so the small ones sit on the number's baseline.
    let mut x = rect.min.x;
    for t in [&prefix, &number, &unit] {
        paint_tracked(ui, pos2(x, rect.max.y - t.size.y), t);
        x += t.size.x + gap;
    }
    if let Some(sub) = &v.sub {
        let s = layout_tracked(ui, &sub.to_uppercase(), display(13.0), MUTED, 0.5);
        let (r, _) = ui.allocate_exact_size(s.size, Sense::hover());
        // right-align under the number
        paint_tracked(ui, pos2(rect.max.x - s.size.x, r.min.y), &s);
    }
}

fn main() -> eframe::Result {
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Charge Deck")
        // The design draws its own title bar, so no native one.
        .with_decorations(false)
        .with_inner_size([480.0, 800.0])
        .with_min_inner_size([380.0, 460.0]);
    if std::env::var_os("CHARGE_DECK_DEMO").is_some() {
        // Taller window so all demo cards are visible at once.
        viewport = viewport.with_inner_size([480.0, 1040.0]).with_position([20.0, 0.0]);
    }
    let options = eframe::NativeOptions { viewport, ..Default::default() };
    eframe::run_native("Charge Deck", options, Box::new(|cc| Ok(Box::new(ChargeDeck::new(cc)))))
}
