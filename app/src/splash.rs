//! An optional splash screen while the library loads: a borderless,
//! transparent window in one of four styles. Off unless chosen in Preferences.

use gtk::prelude::*;
use gtk::{cairo, glib, pango};
use std::f64::consts::{PI, TAU};

pub const STYLES: [(&str, &str); 4] = [
    ("minimal", "Minimal GNOME"),
    ("illuminated", "Illuminated page"),
    ("rose", "Rose window"),
    ("verse", "Verse reveal"),
];

fn rgb(hex: u32) -> (f64, f64, f64) {
    (((hex >> 16) & 0xff) as f64 / 255.0, ((hex >> 8) & 0xff) as f64 / 255.0, (hex & 0xff) as f64 / 255.0)
}

fn rounded(cr: &cairo::Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    cr.new_sub_path();
    cr.arc(x + w - r, y + r, r, -PI / 2.0, 0.0);
    cr.arc(x + w - r, y + h - r, r, 0.0, PI / 2.0);
    cr.arc(x + r, y + h - r, r, PI / 2.0, PI);
    cr.arc(x + r, y + r, r, PI, 1.5 * PI);
    cr.close_path();
}

fn text(cr: &cairo::Context, pl: &pango::Layout, s: &str, font: &str, cx: f64, y: f64, col: (f64, f64, f64, f64)) {
    pl.set_font_description(Some(&pango::FontDescription::from_string(font)));
    pl.set_text(s);
    let (w, _) = pl.pixel_size();
    cr.set_source_rgba(col.0, col.1, col.2, col.3);
    cr.move_to(cx - w as f64 / 2.0, y);
    pangocairo::functions::show_layout(cr, pl);
}

/// t runs 0..1 through the animation.
fn draw(style: &str, cr: &cairo::Context, w: f64, h: f64, t: f64, verse: &(String, String), pl: &pango::Layout) {
    cr.set_operator(cairo::Operator::Source);
    cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
    let _ = cr.paint();
    cr.set_operator(cairo::Operator::Over);
    let (cx, cy) = (w / 2.0, h / 2.0);
    match style {
        "illuminated" => {
            let (pw, ph) = (300.0, 210.0);
            let (x, y) = (cx - pw / 2.0, cy - ph / 2.0);
            let paper = rgb(0xf3ead3);
            let ink = rgb(0x2c2116);
            cr.set_source_rgba(paper.0, paper.1, paper.2, 0.97);
            rounded(cr, x, y, pw, ph, 8.0);
            let _ = cr.fill_preserve();
            cr.set_source_rgba(0.55, 0.42, 0.25, 0.7);
            cr.set_line_width(1.2);
            let _ = cr.stroke();
            // running head between rules
            cr.set_source_rgba(ink.0, ink.1, ink.2, 0.6);
            cr.rectangle(x + 24.0, y + 22.0, pw - 48.0, 0.8);
            cr.rectangle(x + 24.0, y + 46.0, pw - 48.0, 0.8);
            let _ = cr.fill();
            text(cr, pl, "The Holy Bible", "EB Garamond, Cardo, serif Italic 13", cx, y + 25.0, (ink.0, ink.1, ink.2, 1.0));
            // the initial, its scrollwork growing in
            cr.save().ok();
            cr.translate(x + 30.0, y + 66.0);
            let s = 86.0;
            cr.set_source_rgb(ink.0, ink.1, ink.2);
            cr.rectangle(0.0, 0.0, s, s);
            let _ = cr.fill();
            cr.set_source_rgb(paper.0, paper.1, paper.2);
            cr.set_line_width(1.2);
            cr.rectangle(4.0, 4.0, s - 8.0, s - 8.0);
            let _ = cr.stroke();
            cr.set_line_width(2.0);
            for k in 0..6 {
                let a = k as f64 / 6.0 * TAU;
                let (px, py) = (s / 2.0 + 24.0 * a.cos(), s / 2.0 + 24.0 * a.sin());
                cr.new_path();
                let turns = t.min(1.0) * 1.5;
                for i in 0..=30 {
                    let u = i as f64 / 30.0;
                    let ang = a + PI + u * turns * TAU;
                    let r = 13.0 * (1.0 - u * 0.8);
                    let (qx, qy) = (px + r * ang.cos(), py + r * ang.sin());
                    if i == 0 {
                        cr.move_to(qx, qy)
                    } else {
                        cr.line_to(qx, qy)
                    }
                }
                let _ = cr.stroke();
            }
            pl.set_font_description(Some(&pango::FontDescription::from_string("EB Garamond, Cardo, serif Bold 54")));
            pl.set_text("B");
            let (lw, lh) = pl.pixel_size();
            cr.move_to((s - lw as f64) / 2.0, (s - lh as f64) / 2.0);
            pangocairo::functions::layout_path(cr, pl);
            cr.set_source_rgb(ink.0, ink.1, ink.2);
            cr.set_line_width(3.0);
            let _ = cr.stroke_preserve();
            cr.set_source_rgb(paper.0, paper.1, paper.2);
            let _ = cr.fill();
            cr.restore().ok();
            pl.set_font_description(Some(&pango::FontDescription::from_string("EB Garamond, Cardo, serif 15")));
            pl.set_width(150 * pango::SCALE);
            pl.set_wrap(pango::WrapMode::Word);
            pl.set_text("ible — offline study, notes and history");
            cr.set_source_rgb(ink.0, ink.1, ink.2);
            cr.move_to(x + 130.0, y + 72.0);
            pangocairo::functions::show_layout(cr, pl);
            pl.set_width(-1);
            text(cr, pl, "❦", "EB Garamond, serif 18", cx, y + ph - 40.0, (ink.0, ink.1, ink.2, 0.7));
        }
        "rose" => {
            let r = 120.0;
            cr.set_source_rgba(0.106, 0.141, 0.2, 0.6);
            cr.arc(cx, cy - 10.0, r + 10.0, 0.0, TAU);
            let _ = cr.fill();
            let colours = [0xc64600, 0xe5a50a, 0x26a269, 0x1c71d8, 0x9141ac, 0xa51d2d, 0xe66100, 0x2190a4];
            // panes light one after another
            for (i, c) in colours.iter().enumerate() {
                let lit = ((t * 1.4 - i as f64 / 10.0) * 4.0).clamp(0.15, 1.0);
                let (r0, g0, b0) = rgb(*c);
                cr.set_source_rgba(r0, g0, b0, lit);
                let a0 = -PI / 2.0 + i as f64 / 8.0 * TAU;
                cr.move_to(cx, cy - 10.0);
                cr.arc(cx, cy - 10.0, r, a0, a0 + TAU / 8.0);
                cr.close_path();
                let _ = cr.fill();
            }
            cr.set_source_rgb(0.106, 0.141, 0.2);
            cr.set_line_width(5.0);
            cr.arc(cx, cy - 10.0, r, 0.0, TAU);
            let _ = cr.stroke();
            cr.arc(cx, cy - 10.0, r * 0.58, 0.0, TAU);
            let _ = cr.stroke();
            for i in 0..8 {
                let a = -PI / 2.0 + i as f64 / 8.0 * TAU;
                cr.move_to(cx, cy - 10.0);
                cr.line_to(cx + r * a.cos(), cy - 10.0 + r * a.sin());
                let _ = cr.stroke();
            }
            cr.set_source_rgb(0.965, 0.96, 0.957);
            cr.arc(cx, cy - 10.0, 36.0, 0.0, TAU);
            let _ = cr.fill();
            cr.set_source_rgb(0.106, 0.141, 0.2);
            cr.rectangle(cx - 5.0, cy - 34.0, 10.0, 48.0);
            cr.rectangle(cx - 18.0, cy - 22.0, 36.0, 10.0);
            let _ = cr.fill();
            text(cr, pl, "B I B L E", "Cantarell, sans Bold 14", cx, cy + r + 14.0, (0.965, 0.96, 0.957, 0.95));
        }
        "verse" => {
            let a = (t * 2.0).min(1.0);
            pl.set_font_description(Some(&pango::FontDescription::from_string("EB Garamond, Cardo, serif 26")));
            pl.set_width(((w - 60.0) * pango::SCALE as f64) as i32);
            pl.set_wrap(pango::WrapMode::Word);
            pl.set_alignment(pango::Alignment::Center);
            pl.set_text(&verse.0);
            let (_, th) = pl.pixel_size();
            cr.set_source_rgba(0.98, 0.96, 0.91, a);
            cr.move_to(30.0, cy + 4.0 - th as f64);
            pangocairo::functions::show_layout(cr, pl);
            pl.set_width(-1);
            pl.set_alignment(pango::Alignment::Left);
            cr.set_source_rgba(0.98, 0.96, 0.91, a * 0.6);
            let lw = 80.0 * a;
            cr.rectangle(cx - lw / 2.0, cy + 14.0, lw, 1.2);
            let _ = cr.fill();
            text(cr, pl, &verse.1.to_uppercase(), "Cantarell, sans 12", cx, cy + 24.0, (0.98, 0.96, 0.91, a * 0.85));
        }
        _ => {
            // minimal: a frosted pill with the icon, name and three pulsing dots
            let (pw, ph) = (250.0, 92.0);
            let (x, y) = (cx - pw / 2.0, cy - ph / 2.0);
            cr.set_source_rgba(0.98, 0.98, 0.98, 0.9);
            rounded(cr, x, y, pw, ph, 26.0);
            let _ = cr.fill();
            let blue = rgb(0x3584e4);
            cr.set_source_rgb(blue.0, blue.1, blue.2);
            rounded(cr, x + 22.0, y + 20.0, 52.0, 52.0, 14.0);
            let _ = cr.fill();
            // an open book on the icon
            cr.set_source_rgb(1.0, 1.0, 1.0);
            cr.set_line_width(2.6);
            cr.move_to(x + 48.0, y + 34.0);
            cr.curve_to(x + 42.0, y + 30.0, x + 36.0, y + 30.0, x + 32.0, y + 32.0);
            cr.line_to(x + 32.0, y + 58.0);
            cr.curve_to(x + 36.0, y + 56.0, x + 42.0, y + 56.0, x + 48.0, y + 60.0);
            cr.curve_to(x + 54.0, y + 56.0, x + 60.0, y + 56.0, x + 64.0, y + 58.0);
            cr.line_to(x + 64.0, y + 32.0);
            cr.curve_to(x + 60.0, y + 30.0, x + 54.0, y + 30.0, x + 48.0, y + 34.0);
            cr.line_to(x + 48.0, y + 60.0);
            let _ = cr.stroke();
            pl.set_font_description(Some(&pango::FontDescription::from_string("Cantarell, sans Bold 19")));
            pl.set_text("Bible");
            cr.set_source_rgb(0.141, 0.122, 0.192);
            cr.move_to(x + 92.0, y + 22.0);
            pangocairo::functions::show_layout(cr, pl);
            for i in 0..3 {
                let phase = ((t * 3.0 - i as f64 * 0.2).rem_euclid(1.0) * TAU).sin() * 0.5 + 0.5;
                cr.set_source_rgba(blue.0, blue.1, blue.2, 0.3 + 0.7 * phase);
                cr.arc(x + 97.0 + i as f64 * 14.0, y + 62.0, 4.0, 0.0, TAU);
                let _ = cr.fill();
            }
        }
    }
}

/// Show the splash; `done` runs when it has faded, at least `ms` later.
pub fn show(app: &adw::Application, style: &str, verse: (String, String), ms: u64, done: impl FnOnce() + 'static) {
    let area = gtk::DrawingArea::new();
    area.set_content_width(520);
    area.set_content_height(360);
    let win = gtk::Window::builder().application(app).decorated(false).resizable(false).child(&area).title("Bible").build();
    win.add_css_class("splash");
    let start = std::time::Instant::now();
    let style = style.to_string();
    let a2 = area.clone();
    area.set_draw_func(move |a, cr, w, h| {
        let t = start.elapsed().as_secs_f64() / (ms as f64 / 1000.0);
        let pl = a.create_pango_layout(None);
        draw(&style, cr, w as f64, h as f64, t, &verse, &pl);
    });
    let tick = a2.add_tick_callback(|a, _| {
        a.queue_draw();
        glib::ControlFlow::Continue
    });
    win.present();
    let w = win.clone();
    let done = std::cell::RefCell::new(Some(done));
    glib::timeout_add_local_once(std::time::Duration::from_millis(ms), move || {
        if let Some(f) = done.borrow_mut().take() {
            f();
        }
        // fade out over the main window
        let w2 = w.clone();
        let mut step = 0;
        glib::timeout_add_local(std::time::Duration::from_millis(16), move || {
            step += 1;
            w2.set_opacity(1.0 - step as f64 / 15.0);
            if step >= 15 {
                w2.close();
                return glib::ControlFlow::Break;
            }
            glib::ControlFlow::Continue
        });
        tick.remove();
    });
}

/// A short verse for the "verse" style: today's passage if it fits, else Genesis 1:3.
pub fn verse_of_the_day() -> (String, String) {
    let fallback = ("Let there be light".to_string(), "Genesis 1:3".to_string());
    let today = chrono::Local::now().date_naive();
    let trad = bible_core::churchyear::Tradition::from_name(&crate::config::Config::load().tradition);
    let (_, osis) = bible_core::daily::passage(today, trad);
    let Some((book, ch, v, _)) = bible_core::daily::verses(osis) else { return fallback };
    let Some(m) = crate::searchprovider::bible() else { return fallback };
    let Some(t) = m.verse_plain(&book, ch, v) else { return fallback };
    let t = t.trim().trim_end_matches([',', ';', ':']).to_string();
    if t.chars().count() > 70 {
        return fallback;
    }
    let name = bible_core::Ref::new(&book, ch, v).to_string();
    (t, name)
}
