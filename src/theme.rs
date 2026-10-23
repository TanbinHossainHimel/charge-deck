//! Colours, fonts and small drawing helpers for the window.
//! The look follows the "Charge Deck" design canvas: near-black, one red
//! accent, chamfered (corner-cut) shapes, slanted battery segments.

use eframe::egui::{
    self, Color32, FontData, FontDefinitions, FontFamily, FontId, Galley, Margin, Pos2, Rect, Response, Sense, Shape,
    Stroke, Ui, Vec2, pos2, vec2,
};
use std::sync::Arc;

// ---- Palette ----
pub const BG: Color32 = Color32::from_rgb(0x08, 0x09, 0x0B);
pub const BAR_BG: Color32 = Color32::from_rgb(0x05, 0x06, 0x07); // title bar + footer
pub const BAR_LINE: Color32 = Color32::from_rgb(0x1A, 0x1D, 0x22);
pub const CARD: Color32 = Color32::from_rgb(0x10, 0x12, 0x16);
pub const CARD_EDGE: Color32 = Color32::from_rgb(0x22, 0x26, 0x2D);
pub const TILE: Color32 = Color32::from_rgb(0x18, 0x1B, 0x21);
pub const CHIP: Color32 = Color32::from_rgb(0x1A, 0x1D, 0x23);
pub const CHIP_TEXT: Color32 = Color32::from_rgb(0xB4, 0xBA, 0xC4);
pub const SEG_OFF: Color32 = Color32::from_rgb(0x1E, 0x22, 0x28);
pub const SEG_SCAN: Color32 = Color32::from_rgb(0x3A, 0x3F, 0x48);
pub const SEG_GREY: Color32 = Color32::from_rgb(0x4A, 0x50, 0x5A);
pub const STAT_TOP: Color32 = Color32::from_rgb(0x2A, 0x2F, 0x37);
pub const TEXT: Color32 = Color32::from_rgb(0xF2, 0xF3, 0xF5);
pub const TEXT_SOFT: Color32 = Color32::from_rgb(0xD5, 0xDA, 0xE3);
pub const MUTED: Color32 = Color32::from_rgb(0x9A, 0xA0, 0xAA);
pub const RED: Color32 = Color32::from_rgb(0xFF, 0x2D, 0x46); // brand accent
pub const RED_SOFT: Color32 = Color32::from_rgb(0xFF, 0x54, 0x68);
pub const RED_HOVER: Color32 = Color32::from_rgb(0xFF, 0x4B, 0x60);
pub const GREEN: Color32 = Color32::from_rgb(0x3D, 0xF5, 0x8A);
pub const AMBER: Color32 = Color32::from_rgb(0xFF, 0xC2, 0x33);
pub const LOW: Color32 = Color32::from_rgb(0xFF, 0x40, 0x58);

/// green > 50 %, amber 20-50 %, red < 20 %
pub fn level_color(percent: u8) -> Color32 {
    match percent {
        51..=100 => GREEN,
        20..=50 => AMBER,
        _ => LOW,
    }
}

// ---- Fonts ----
// Rajdhani (SIL Open Font License) is bundled into the exe, because the app
// makes no network requests, so it cannot download fonts.

pub fn display(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("display".into()))
}

pub fn semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("semibold".into()))
}

pub fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "rajdhani-bold".into(),
        Arc::new(FontData::from_static(include_bytes!("../assets/fonts/Rajdhani-Bold.ttf"))),
    );
    fonts.font_data.insert(
        "rajdhani-semibold".into(),
        Arc::new(FontData::from_static(include_bytes!("../assets/fonts/Rajdhani-SemiBold.ttf"))),
    );
    // Fall back to egui's built-in fonts for glyphs Rajdhani lacks.
    let fallback = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    for (name, face) in [("display", "rajdhani-bold"), ("semibold", "rajdhani-semibold")] {
        let mut list = vec![face.to_string()];
        list.extend(fallback.clone());
        fonts.families.insert(FontFamily::Name(name.into()), list);
    }
    ctx.set_fonts(fonts);

    let mut v = egui::Visuals::dark();
    v.panel_fill = BG;
    v.window_fill = BG;
    v.extreme_bg_color = BG;
    v.override_text_color = Some(TEXT);
    v.selection.bg_fill = RED;
    v.selection.stroke = Stroke::new(1.5, TEXT);
    v.widgets.inactive.bg_fill = CARD;
    v.widgets.inactive.weak_bg_fill = CARD;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0x3A, 0x3F, 0x48));
    v.widgets.hovered.bg_fill = CARD;
    v.widgets.hovered.weak_bg_fill = CARD;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, RED_SOFT);
    v.widgets.active.bg_stroke = Stroke::new(1.0, RED);
    ctx.set_visuals(v);
}

// ---- Text with letter spacing (egui has none built in) ----

pub struct Tracked {
    galleys: Vec<Arc<Galley>>,
    tracking: f32,
    pub size: Vec2,
}

pub fn layout_tracked(ui: &Ui, text: &str, font: FontId, color: Color32, tracking: f32) -> Tracked {
    let galleys: Vec<Arc<Galley>> =
        text.chars().map(|c| ui.painter().layout_no_wrap(c.to_string(), font.clone(), color)).collect();
    let width: f32 = galleys.iter().map(|g| g.size().x + tracking).sum::<f32>() - if galleys.is_empty() { 0.0 } else { tracking };
    let height = galleys.iter().map(|g| g.size().y).fold(0.0, f32::max);
    Tracked { galleys, tracking, size: vec2(width.max(0.0), height) }
}

pub fn paint_tracked(ui: &Ui, top_left: Pos2, t: &Tracked) {
    let mut x = top_left.x;
    for g in &t.galleys {
        ui.painter().galley(pos2(x, top_left.y), g.clone(), TEXT);
        x += g.size().x + t.tracking;
    }
}

/// Text with letter spacing placed in the layout like a label.
pub fn tracked(ui: &mut Ui, text: &str, font: FontId, color: Color32, tracking: f32) -> Response {
    let t = layout_tracked(ui, text, font, color, tracking);
    let (rect, resp) = ui.allocate_exact_size(t.size, Sense::hover());
    paint_tracked(ui, rect.min, &t);
    resp
}

// ---- Chamfered shapes: top-left and bottom-right corners are cut ----

pub fn chamfer(rect: Rect, tl: f32, br: f32) -> Vec<Pos2> {
    vec![
        pos2(rect.min.x + tl, rect.min.y),
        pos2(rect.max.x, rect.min.y),
        pos2(rect.max.x, rect.max.y - br),
        pos2(rect.max.x - br, rect.max.y),
        pos2(rect.min.x, rect.max.y),
        pos2(rect.min.x, rect.min.y + tl),
    ]
}

pub fn chamfer_shape(rect: Rect, cut: f32, fill: Color32) -> Shape {
    Shape::convex_polygon(chamfer(rect, cut, cut), fill, Stroke::NONE)
}

/// A chamfered box with an optional 1 px edge colour. The background shapes are
/// reserved first and filled in once the content size is known.
pub fn chamfer_box(
    ui: &mut Ui,
    edge: Option<Color32>,
    fill: Color32,
    cut: f32,
    margin: Margin,
    add_contents: impl FnOnce(&mut Ui),
) -> Rect {
    let idx = ui.painter().add(Shape::Noop);
    let inner = egui::Frame::new().inner_margin(margin).show(ui, |ui| {
        ui.set_width(ui.available_width());
        add_contents(ui);
    });
    let rect = inner.response.rect;
    let mut shapes = Vec::new();
    let body = match edge {
        Some(e) => {
            shapes.push(chamfer_shape(rect, cut, e));
            rect.shrink(1.0)
        }
        None => rect,
    };
    shapes.push(chamfer_shape(body, (cut - 0.5).max(0.0), fill));
    ui.painter().set(idx, Shape::Vec(shapes));
    rect
}

/// The slanted 10-segment battery bar. `colors` has one colour per segment.
pub fn segment_bar(ui: &mut Ui, colors: &[Color32; 10]) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 8.0), Sense::hover());
    let gap = 4.0;
    let pad = 3.0;
    let seg_w = (rect.width() - pad * 2.0 - gap * 9.0) / 10.0;
    let lean = 8.0 * 24f32.to_radians().tan() / 2.0; // skewX(-24deg): top leans right
    for (i, color) in colors.iter().enumerate() {
        let x = rect.min.x + pad + i as f32 * (seg_w + gap);
        let pts = vec![
            pos2(x + lean, rect.min.y),
            pos2(x + seg_w + lean, rect.min.y),
            pos2(x + seg_w - lean, rect.max.y),
            pos2(x - lean, rect.max.y),
        ];
        ui.painter().add(Shape::convex_polygon(pts, *color, Stroke::NONE));
    }
}
