//! The loading animation, used wherever the app waits: a turning cross, a
//! Chi-Rho, an ichthys swimming a circle, or a turning Trinity knot, drawn
//! in gold with cairo. The user picks one in Preferences.

use gtk::prelude::*;
use std::cell::Cell;
use std::f64::consts::{PI, TAU};
use std::rc::Rc;

/// (id, name shown in Preferences)
pub const STYLES: &[(&str, &str)] = &[
    ("cross", "Turning cross"),
    ("chirho", "Chi-Rho"),
    ("ichthys", "Ichthys (fish)"),
    ("triquetra", "Trinity knot"),
];

thread_local! {
    static STYLE: Cell<usize> = const { Cell::new(0) };
}

/// Set the style used by every loader (called from the config).
pub fn set_style(id: &str) {
    STYLE.with(|s| s.set(STYLES.iter().position(|x| x.0 == id).unwrap_or(0)));
}

fn gold(k: f64) -> (f64, f64, f64) {
    // from deep amber to pale gold
    let k = k.clamp(0.0, 1.0);
    let a = (0.73, 0.46, 0.09);
    let b = (0.98, 0.78, 0.46);
    (a.0 + (b.0 - a.0) * k, a.1 + (b.1 - a.1) * k, a.2 + (b.2 - a.2) * k)
}

type P3 = [f64; 3];

fn rot(p: P3, a: f64, b: f64) -> P3 {
    let [x, y, z] = p;
    let (c, s) = (a.cos(), a.sin());
    let (x, z) = (x * c + z * s, -x * s + z * c);
    let (c, s) = (b.cos(), b.sin());
    let (y, z) = (y * c - z * s, y * s + z * c);
    [x, y, z]
}

fn cuboid(x: (f64, f64), y: (f64, f64), z: (f64, f64)) -> Vec<[P3; 4]> {
    let v = [
        [x.0, y.0, z.0], [x.1, y.0, z.0], [x.1, y.1, z.0], [x.0, y.1, z.0],
        [x.0, y.0, z.1], [x.1, y.0, z.1], [x.1, y.1, z.1], [x.0, y.1, z.1],
    ];
    [[0, 1, 2, 3], [5, 4, 7, 6], [4, 0, 3, 7], [1, 5, 6, 2], [4, 5, 1, 0], [3, 2, 6, 7]]
        .iter()
        .map(|f| [v[f[0]], v[f[1]], v[f[2]], v[f[3]]])
        .collect()
}

fn draw_cross(cr: &gtk::cairo::Context, t: f64, r: f64) {
    let mut faces = cuboid((-0.12, 0.12), (-0.75, 0.75), (-0.12, 0.12));
    faces.extend(cuboid((-0.5, 0.5), (-0.42, -0.18), (-0.12, 0.12)));
    let light = [0.5, -0.4, 0.75];
    let mut shown: Vec<(f64, [P3; 4], f64)> = faces
        .iter()
        .filter_map(|f| {
            let q = f.map(|p| rot(p, t * 1.1, 0.25));
            let u = [q[1][0] - q[0][0], q[1][1] - q[0][1], q[1][2] - q[0][2]];
            let w = [q[2][0] - q[0][0], q[2][1] - q[0][1], q[2][2] - q[0][2]];
            let n = [u[1] * w[2] - u[2] * w[1], u[2] * w[0] - u[0] * w[2], u[0] * w[1] - u[1] * w[0]];
            let m = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-9);
            if n[2] / m < 0.0 {
                return None;
            }
            let d = (n[0] * light[0] + n[1] * light[1] + n[2] * light[2]) / m;
            let z = q.iter().map(|p| p[2]).sum::<f64>() / 4.0;
            Some((z, q, d))
        })
        .collect();
    shown.sort_by(|a, b| a.0.total_cmp(&b.0));
    for (_, q, d) in shown {
        for (i, p) in q.iter().enumerate() {
            let k = 3.0 / (3.0 - p[2]);
            let (x, y) = (p[0] * r * k, p[1] * r * k);
            if i == 0 { cr.move_to(x, y) } else { cr.line_to(x, y) }
        }
        cr.close_path();
        let (rr, g, b) = gold(0.25 + 0.75 * d);
        cr.set_source_rgb(rr, g, b);
        let _ = cr.fill();
    }
}

fn draw_chirho(cr: &gtk::cairo::Context, t: f64, r: f64) {
    let s = (t * 1.3).cos();
    let sx = if s.abs() < 0.08 { 0.08 * s.signum() } else { s };
    let k = r / 66.0;
    cr.save().ok();
    cr.scale(sx * k, k);
    cr.set_line_cap(gtk::cairo::LineCap::Round);
    cr.set_line_width(9.0);
    let (a, g, b) = gold(0.4 + 0.6 * s.abs());
    cr.set_source_rgb(a, g, b);
    cr.move_to(-34.0, -38.0);
    cr.line_to(34.0, 38.0);
    cr.move_to(34.0, -38.0);
    cr.line_to(-34.0, 38.0);
    let _ = cr.stroke();
    let (a, g, b) = gold(0.2 + 0.8 * s.abs());
    cr.set_source_rgb(a, g, b);
    cr.move_to(0.0, 52.0);
    cr.line_to(0.0, -50.0);
    let _ = cr.stroke();
    cr.move_to(0.0, -50.0);
    cr.curve_to(34.0, -52.0, 34.0, -14.0, 0.0, -16.0);
    let _ = cr.stroke();
    cr.restore().ok();
    cr.set_source_rgba(0.52, 0.31, 0.04, 0.35);
    cr.set_line_width(2.0);
    cr.arc(0.0, 0.0, r, 0.0, TAU);
    let _ = cr.stroke();
}

fn draw_fish(cr: &gtk::cairo::Context, t: f64, r: f64) {
    let k = r / 66.0;
    let orbit = 48.0 * k;
    cr.set_source_rgba(0.52, 0.31, 0.04, 0.25);
    cr.set_line_width(2.0);
    cr.set_dash(&[3.0, 6.0], 0.0);
    cr.arc(0.0, 0.0, orbit, 0.0, TAU);
    let _ = cr.stroke();
    cr.set_dash(&[], 0.0);
    let a = t * 1.2;
    cr.save().ok();
    cr.translate(orbit * a.cos(), orbit * a.sin());
    cr.rotate(a + PI / 2.0);
    cr.scale(k * (1.0 + 0.06 * (t * 8.0).sin()), k);
    cr.set_line_width(5.0);
    cr.set_line_cap(gtk::cairo::LineCap::Round);
    let (rr, g, b) = gold(0.8);
    cr.set_source_rgb(rr, g, b);
    cr.move_to(-26.0, 8.0);
    cr.curve_to(-12.0, -15.0, 8.0, -15.0, 22.0, 0.0);
    cr.curve_to(8.0, 15.0, -12.0, 15.0, -26.0, -8.0);
    let _ = cr.stroke();
    cr.restore().ok();
}

fn draw_knot(cr: &gtk::cairo::Context, t: f64, r: f64) {
    let pts: Vec<P3> = (0..=240)
        .map(|i| {
            let u = i as f64 / 240.0 * TAU;
            let p = [
                (u.sin() + 2.0 * (2.0 * u).sin()) / 3.1,
                (u.cos() - 2.0 * (2.0 * u).cos()) / 3.1,
                -(3.0 * u).sin() / 3.0,
            ];
            rot(p, t * 0.9, 0.35 * (t * 0.5).sin())
        })
        .collect();
    let mut segs: Vec<(P3, P3)> = pts.windows(2).map(|w| (w[0], w[1])).collect();
    segs.sort_by(|a, b| (a.0[2] + a.1[2]).total_cmp(&(b.0[2] + b.1[2])));
    cr.set_line_cap(gtk::cairo::LineCap::Round);
    let k = r / 66.0;
    for (p, q) in segs {
        let z = (p[2] + q[2]) / 2.0;
        let (rr, g, b) = gold(0.35 + 0.6 * (z + 1.0) / 2.0);
        cr.set_source_rgb(rr, g, b);
        cr.set_line_width((7.0 + 3.0 * z) * k);
        cr.move_to(p[0] * r * 0.94, p[1] * r * 0.94);
        cr.line_to(q[0] * r * 0.94, q[1] * r * 0.94);
        let _ = cr.stroke();
    }
}

/// Draw one frame of a style centred at the origin, `r` its radius.
pub fn draw(cr: &gtk::cairo::Context, style: usize, t: f64, r: f64) {
    match style {
        1 => draw_chirho(cr, t, r),
        2 => draw_fish(cr, t, r),
        3 => draw_knot(cr, t, r),
        _ => draw_cross(cr, t, r),
    }
}

/// A loader of the given size. It animates only while visible; `fixed`
/// pins a style (for previews), otherwise the user's choice is used.
pub fn new_with(size: i32, fixed: Option<usize>) -> gtk::DrawingArea {
    let area = gtk::DrawingArea::new();
    area.set_content_width(size);
    area.set_content_height(size);
    area.set_halign(gtk::Align::Center);
    area.set_valign(gtk::Align::Center);
    area.set_accessible_role(gtk::AccessibleRole::ProgressBar);
    area.update_property(&[gtk::accessible::Property::Label("Loading")]);
    let start = Rc::new(Cell::new(None::<i64>));
    let st = start.clone();
    area.set_draw_func(move |a, cr, w, h| {
        let now = a.frame_clock().map(|c| c.frame_time()).unwrap_or(0);
        let t0 = st.get().unwrap_or_else(|| {
            st.set(Some(now));
            now
        });
        let t = (now - t0) as f64 / 1e6;
        cr.translate(w as f64 / 2.0, h as f64 / 2.0);
        let style = fixed.unwrap_or_else(|| STYLE.with(|s| s.get()));
        draw(cr, style, t, w.min(h) as f64 / 2.0 - 2.0);
    });
    area.add_tick_callback(|a, _| {
        if a.is_mapped() {
            a.queue_draw();
        }
        glib::ControlFlow::Continue
    });
    area
}

pub fn new(size: i32) -> gtk::DrawingArea {
    new_with(size, None)
}

use gtk::glib;

#[cfg(test)]
mod tests {
    #[test]
    fn styles_have_unique_ids() {
        let mut ids: Vec<&str> = super::STYLES.iter().map(|s| s.0).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), super::STYLES.len());
    }

    #[test]
    fn every_style_draws() {
        let surf = gtk::cairo::ImageSurface::create(gtk::cairo::Format::ARgb32, 64, 64).unwrap();
        for style in 0..super::STYLES.len() {
            let cr = gtk::cairo::Context::new(&surf).unwrap();
            cr.translate(32.0, 32.0);
            for t in [0.0, 0.7, 1.9, 5.3] {
                super::draw(&cr, style, t, 30.0);
            }
        }
        // something was painted
        surf.flush();
        drop(surf);
    }
}
