//! Line-art icons from the design, painted with code (no image files, no SVG
//! library). Every icon is drawn on a 24x24 grid scaled into the given rect.

use crate::theme::RED;
use charge_deck::model::DeviceKind;
use eframe::egui::{self, Color32, CornerRadius, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, pos2};

/// Maps the 24x24 design grid onto a screen rect.
struct Grid {
    origin: Pos2,
    scale: f32,
}

impl Grid {
    fn new(rect: Rect) -> Self {
        Grid { origin: rect.min, scale: rect.width() / 24.0 }
    }
    fn p(&self, x: f32, y: f32) -> Pos2 {
        pos2(self.origin.x + x * self.scale, self.origin.y + y * self.scale)
    }
    fn rect(&self, x0: f32, y0: f32, x1: f32, y1: f32) -> Rect {
        Rect::from_min_max(self.p(x0, y0), self.p(x1, y1))
    }
    fn stroke(&self, w: f32, c: Color32) -> Stroke {
        Stroke::new(w * self.scale, c)
    }
    fn radius(&self, r: f32) -> CornerRadius {
        CornerRadius::same((r * self.scale).round().clamp(0.0, 255.0) as u8)
    }
    fn line(&self, painter: &Painter, pts: &[(f32, f32)], s: Stroke) {
        let points: Vec<Pos2> = pts.iter().map(|&(x, y)| self.p(x, y)).collect();
        painter.add(Shape::line(points, s));
    }
    /// Points on a circle arc (angles in degrees, screen coordinates: y down).
    fn arc(&self, cx: f32, cy: f32, r: f32, from_deg: f32, to_deg: f32) -> Vec<(f32, f32)> {
        let steps = 24;
        (0..=steps)
            .map(|i| {
                let a = (from_deg + (to_deg - from_deg) * i as f32 / steps as f32).to_radians();
                (cx + r * a.cos(), cy + r * a.sin())
            })
            .collect()
    }
}

/// Device icon: body in `fg`, small details in the brand red.
pub fn device(painter: &Painter, rect: Rect, kind: Option<DeviceKind>, fg: Color32) {
    let g = Grid::new(rect);
    let s = g.stroke(1.4, fg);
    let red = g.stroke(1.6, RED);
    match kind {
        Some(DeviceKind::Keyboard) => {
            painter.rect_stroke(g.rect(1.5, 6.0, 22.5, 17.0), g.radius(2.0), s, StrokeKind::Middle);
            for (y, xs) in [
                (8.8, vec![(4.2, 5.4), (6.9, 8.1), (9.6, 10.8), (12.3, 13.5), (15.0, 16.2), (18.2, 20.2)]),
                (11.5, vec![(4.2, 5.4), (6.9, 8.1), (9.6, 10.8), (12.3, 13.5), (15.0, 16.2), (18.2, 20.2)]),
                (14.2, vec![(4.2, 6.0), (7.5, 15.0), (18.2, 20.2)]),
            ] {
                for (a, b) in xs {
                    g.line(painter, &[(a, y), (b, y)], s);
                }
            }
            g.line(painter, &[(3.5, 19.6), (20.5, 19.6)], red);
        }
        Some(DeviceKind::Mouse) => {
            painter.rect_stroke(g.rect(6.0, 2.5, 18.0, 21.5), g.radius(6.0), s, StrokeKind::Middle);
            g.line(painter, &[(12.0, 2.6), (12.0, 9.6)], s);
            g.line(painter, &[(6.1, 9.6), (17.9, 9.6)], s);
            g.line(painter, &[(4.6, 10.8), (4.6, 12.9)], s);
            g.line(painter, &[(4.6, 14.3), (4.6, 15.9)], s);
            painter.rect_stroke(g.rect(11.0, 4.4, 13.0, 7.8), g.radius(1.0), g.stroke(1.4, RED), StrokeKind::Middle);
            g.line(painter, &[(10.4, 16.4), (12.0, 14.8), (13.6, 16.4)], g.stroke(1.4, RED));
        }
        Some(DeviceKind::Headset) => {
            let mut band = vec![(4.4, 13.0)];
            band.extend(g.arc(12.0, 11.0, 7.6, 180.0, 360.0));
            band.push((19.6, 13.0));
            g.line(painter, &band, s);
            painter.rect_stroke(g.rect(2.4, 12.4, 7.0, 19.6), g.radius(2.0), s, StrokeKind::Middle);
            painter.rect_stroke(g.rect(17.0, 12.4, 21.6, 19.6), g.radius(2.0), s, StrokeKind::Middle);
            g.line(painter, &[(4.7, 19.6), (5.2, 21.0), (7.0, 21.7), (10.0, 21.9)], s);
            painter.circle_stroke(g.p(11.0, 21.9), 0.9 * g.scale, s);
            g.line(painter, &[(4.7, 14.6), (4.7, 17.4)], red);
            g.line(painter, &[(19.3, 14.6), (19.3, 17.4)], red);
        }
        Some(DeviceKind::Controller) => {
            painter.rect_stroke(g.rect(2.0, 6.5, 22.0, 19.0), g.radius(5.5), s, StrokeKind::Middle);
            g.line(painter, &[(6.6, 9.8), (6.6, 13.2)], s);
            g.line(painter, &[(4.9, 11.5), (8.3, 11.5)], s);
            g.line(painter, &[(11.0, 9.2), (13.0, 9.2)], red);
            for (x, y) in [(16.2, 10.4), (18.4, 12.6), (16.2, 12.6), (18.4, 10.4)] {
                painter.circle_filled(g.p(x, y), 1.1 * g.scale, RED);
            }
        }
        // Other / Bluetooth-style rune, in the accent colour.
        Some(DeviceKind::Other) | None => {
            g.line(
                painter,
                &[(7.0, 7.0), (17.0, 17.0), (12.0, 22.0), (12.0, 2.0), (17.0, 7.0), (7.0, 17.0)],
                g.stroke(1.5, RED),
            );
        }
    }
}

/// Filled lightning bolt (charging, logo).
pub fn bolt(painter: &Painter, rect: Rect, color: Color32) {
    let g = Grid::new(rect);
    // The bolt is not convex, and egui's polygon anti-aliasing spikes at its
    // sharp corners, so it is drawn as a plain triangle mesh instead.
    let (p0, p1, p2, p3, p4, p5) = ((13.0, 2.0), (4.0, 14.0), (11.0, 14.0), (10.0, 22.0), (19.0, 10.0), (12.0, 10.0));
    let mut mesh = egui::Mesh::default();
    for tri in [[p0, p1, p5], [p1, p2, p5], [p2, p3, p5], [p3, p4, p5]] {
        let first = mesh.vertices.len() as u32;
        for (x, y) in tri {
            mesh.colored_vertex(g.p(x, y), color);
        }
        mesh.add_triangle(first, first + 1, first + 2);
    }
    painter.add(Shape::mesh(mesh));
}

/// Moon (asleep).
pub fn moon(painter: &Painter, rect: Rect, color: Color32) {
    let g = Grid::new(rect);
    let s = g.stroke(2.0, color);
    let mut pts = g.arc(12.0, 12.0, 9.0, 5.0, 265.0);
    // Inner curve back to the start, approximated by a quadratic curve.
    let (a, c, b) = ((11.2f32, 3.0f32), (12.5f32, 11.8f32), (21.0f32, 12.8f32));
    for i in 0..=12 {
        let t = i as f32 / 12.0;
        pts.push((
            (1.0 - t).powi(2) * a.0 + 2.0 * (1.0 - t) * t * c.0 + t * t * b.0,
            (1.0 - t).powi(2) * a.1 + 2.0 * (1.0 - t) * t * c.1 + t * t * b.1,
        ));
    }
    g.line(painter, &pts, s);
}

/// Circular refresh arrow.
pub fn refresh(painter: &Painter, rect: Rect, color: Color32) {
    let g = Grid::new(rect);
    let s = g.stroke(2.2, color);
    let mut pts = g.arc(12.0, 12.0, 9.0, 0.0, 315.0);
    pts.push((21.0, 8.0));
    g.line(painter, &pts, s);
    g.line(painter, &[(21.0, 3.0), (21.0, 8.0), (16.0, 8.0)], s);
}

/// Battery outline with a small bolt (empty-state badge).
pub fn battery(painter: &Painter, rect: Rect, color: Color32) {
    let g = Grid::new(rect);
    let s = g.stroke(1.5, color);
    painter.rect_stroke(g.rect(2.0, 7.0, 19.0, 17.0), g.radius(1.5), s, StrokeKind::Middle);
    g.line(painter, &[(22.0, 11.0), (22.0, 13.0)], s);
    g.line(painter, &[(11.5, 9.0), (8.5, 12.5), (11.5, 12.5), (10.5, 15.0)], s);
}

/// Window control glyphs: 0 = minimize, 1 = maximize, 2 = close.
pub fn window_control(painter: &Painter, rect: Rect, which: u8, color: Color32) {
    let g = Grid::new(rect);
    let s = g.stroke(1.75, color);
    match which {
        0 => g.line(painter, &[(5.0, 12.0), (19.0, 12.0)], s),
        1 => {
            painter.rect_stroke(g.rect(4.0, 4.0, 20.0, 20.0), CornerRadius::ZERO, s, StrokeKind::Middle);
        }
        _ => {
            g.line(painter, &[(6.0, 6.0), (18.0, 18.0)], s);
            g.line(painter, &[(18.0, 6.0), (6.0, 18.0)], s);
        }
    }
}
