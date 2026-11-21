//! Decorated initials for classic typography: a framed block in the manner
//! of early printed Bibles, with scrollwork behind the letter. Drawn with
//! cairo, so there are no images to ship; each letter gets its own pattern.

use gtk::prelude::*;
use gtk::{cairo, pango};
use std::f64::consts::{PI, TAU};

/// A tiny deterministic generator, so a letter always looks the same.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % 10_000) as f64 / 10_000.0
    }
}

/// A scroll: a vine that curls into a spiral, with small leaves along it.
fn scroll(cr: &cairo::Context, x: f64, y: f64, r: f64, start: f64, dir: f64, rng: &mut Rng) {
    let turns = 1.4 + rng.next() * 0.8;
    let steps = 48;
    cr.new_path();
    for i in 0..=steps {
        let t = i as f64 / steps as f64;
        let a = start + dir * t * turns * TAU;
        let rr = r * (1.0 - t * 0.85);
        let (px, py) = (x + rr * a.cos(), y + rr * a.sin());
        if i == 0 {
            cr.move_to(px, py);
        } else {
            cr.line_to(px, py);
        }
    }
    let _ = cr.stroke();
    // leaves on the outer turn
    for k in 0..4 {
        let t = 0.08 + k as f64 * 0.12;
        let a = start + dir * t * turns * TAU;
        let rr = r * (1.0 - t * 0.85);
        let (px, py) = (x + rr * a.cos(), y + rr * a.sin());
        let la = a + dir * PI / 2.0 + (rng.next() - 0.5) * 0.6;
        let len = r * (0.28 + rng.next() * 0.12);
        cr.save().ok();
        cr.translate(px, py);
        cr.rotate(la);
        cr.new_path();
        cr.move_to(0.0, 0.0);
        cr.curve_to(len * 0.3, -len * 0.35, len * 0.8, -len * 0.25, len, 0.0);
        cr.curve_to(len * 0.8, len * 0.25, len * 0.3, len * 0.35, 0.0, 0.0);
        let _ = cr.fill();
        cr.restore().ok();
    }
}

fn draw(
    cr: &cairo::Context,
    w: f64,
    h: f64,
    letter: char,
    ink: (f64, f64, f64),
    paper: (f64, f64, f64),
    pl: &pango::Layout,
) {
    let s = w.min(h);
    let (ox, oy) = ((w - s) / 2.0, (h - s) / 2.0);
    cr.translate(ox, oy);
    // block and double frame
    cr.set_source_rgb(ink.0, ink.1, ink.2);
    cr.rectangle(0.0, 0.0, s, s);
    let _ = cr.fill();
    cr.set_source_rgb(paper.0, paper.1, paper.2);
    cr.set_line_width(s * 0.012);
    cr.rectangle(s * 0.045, s * 0.045, s * 0.91, s * 0.91);
    let _ = cr.stroke();
    cr.set_line_width(s * 0.006);
    cr.rectangle(s * 0.075, s * 0.075, s * 0.85, s * 0.85);
    let _ = cr.stroke();
    // scrollwork, clipped inside the frame
    cr.save().ok();
    cr.rectangle(s * 0.085, s * 0.085, s * 0.83, s * 0.83);
    cr.clip();
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15 ^ (letter as u64).wrapping_mul(0x100_0000_01B3));
    cr.set_line_width(s * 0.022);
    let n = 8;
    for i in 0..n {
        let a = i as f64 / n as f64 * TAU + rng.next() * 0.4;
        let d = s * (0.22 + rng.next() * 0.16);
        let (x, y) = (s / 2.0 + d * a.cos(), s / 2.0 + d * a.sin());
        let r = s * (0.13 + rng.next() * 0.07);
        scroll(
            cr,
            x,
            y,
            r,
            a + PI,
            if i % 2 == 0 { 1.0 } else { -1.0 },
            &mut rng,
        );
    }
    // small dots fill the gaps, like the punched ground of a woodcut
    for _ in 0..40 {
        let (x, y) = (s * (0.1 + rng.next() * 0.8), s * (0.1 + rng.next() * 0.8));
        cr.arc(x, y, s * 0.011, 0.0, TAU);
        let _ = cr.fill();
    }
    cr.restore().ok();
    // the letter: paper-coloured with an ink outline so it stands off the vines
    let mut fd = pango::FontDescription::from_string(
        "UnifrakturMaguntia, EB Garamond, Cardo, Linux Libertine O, serif",
    );
    fd.set_weight(pango::Weight::Bold);
    fd.set_absolute_size(s * 0.72 * pango::SCALE as f64);
    pl.set_font_description(Some(&fd));
    pl.set_text(&letter.to_string());
    let (_, logical) = pl.pixel_extents();
    let (lw, lh) = (logical.width() as f64, logical.height() as f64);
    cr.move_to(
        (s - lw) / 2.0 - logical.x() as f64,
        (s - lh) / 2.0 - logical.y() as f64,
    );
    pangocairo::functions::layout_path(cr, pl);
    cr.set_source_rgb(ink.0, ink.1, ink.2);
    cr.set_line_width(s * 0.05);
    cr.set_line_join(cairo::LineJoin::Round);
    let _ = cr.stroke_preserve();
    cr.set_source_rgb(paper.0, paper.1, paper.2);
    let _ = cr.fill();
}

/// A decorated initial `size` pixels square.
pub fn ornate_initial(letter: char, size: i32, dark: bool) -> gtk::DrawingArea {
    let area = gtk::DrawingArea::new();
    area.set_content_width(size);
    area.set_content_height(size);
    area.set_valign(gtk::Align::Start);
    area.add_css_class("ornate-initial");
    let (ink, paper) = if dark {
        ((0.92, 0.87, 0.78), (0.16, 0.14, 0.10))
    } else {
        ((0.16, 0.12, 0.08), (0.96, 0.93, 0.85))
    };
    let up = letter.to_uppercase().next().unwrap_or(letter);
    area.set_draw_func(move |a, cr, w, h| {
        let pl = a.create_pango_layout(None);
        draw(cr, w as f64, h as f64, up, ink, paper, &pl);
    });
    area
}
