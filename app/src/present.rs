//! Presentation mode: verses full screen, one at a time, for a room or a
//! second display. Arrow keys, space or a click move on; Esc closes.

use crate::ctx::Ctx;
use adw::prelude::*;
use bible_core::Ref;
use gtk::{gdk, glib};
use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

pub fn present(ctx: &Rc<Ctx>, m: Arc<bible_sword::Module>, book: &str, chapter: u16, verse: u16) {
    let verses: Vec<(u16, String)> = m
        .chapter(book, chapter)
        .into_iter()
        .filter_map(|(v, _)| {
            m.verse_plain(book, chapter, v)
                .map(|t| (v, t.trim().to_string()))
        })
        .filter(|(_, t)| !t.is_empty())
        .collect();
    if verses.is_empty() {
        return;
    }
    let start = verses.iter().position(|(v, _)| *v == verse).unwrap_or(0);
    let idx = Rc::new(Cell::new(start));

    let text = gtk::Label::new(None);
    text.set_wrap(true);
    text.set_justify(gtk::Justification::Center);
    text.set_max_width_chars(40);
    text.add_css_class("present-text");
    let reference = gtk::Label::new(None);
    reference.add_css_class("present-ref");
    let hint = gtk::Label::new(Some("← → or click to move · Esc to close"));
    hint.add_css_class("present-hint");
    let col = gtk::Box::new(gtk::Orientation::Vertical, 28);
    col.set_valign(gtk::Align::Center);
    col.set_halign(gtk::Align::Center);
    col.set_margin_start(64);
    col.set_margin_end(64);
    col.append(&text);
    col.append(&reference);
    let overlay = gtk::Overlay::new();
    overlay.set_child(Some(&col));
    hint.set_valign(gtk::Align::End);
    hint.set_margin_bottom(24);
    overlay.add_overlay(&hint);
    overlay.add_css_class("present");

    let win = gtk::Window::builder()
        .title("Presentation")
        .decorated(false)
        .child(&overlay)
        .build();
    if let Some(parent) = ctx.window.get() {
        win.set_application(parent.application().as_ref());
    }
    let name = Ref::new(book, chapter, 0).book_name();
    let abbr = m.abbreviation();
    let show = {
        let (text, reference, idx) = (text.clone(), reference.clone(), idx.clone());
        let verses = verses.clone();
        move || {
            let (v, t) = &verses[idx.get()];
            text.set_text(t);
            // long verses get a smaller size so they still fit
            text.remove_css_class("present-long");
            if t.chars().count() > 260 {
                text.add_css_class("present-long");
            }
            reference.set_text(&format!("{name} {chapter}:{v} · {abbr}"));
        }
    };
    show();
    let step = {
        let idx = idx.clone();
        let n = verses.len();
        let show = show.clone();
        move |d: i32| {
            let i = (idx.get() as i32 + d).clamp(0, n as i32 - 1) as usize;
            if i != idx.get() {
                idx.set(i);
                show();
            }
        }
    };
    let keys = gtk::EventControllerKey::new();
    let w = win.downgrade();
    let s2 = step.clone();
    keys.connect_key_pressed(move |_, key, _, _| {
        match key {
            gdk::Key::Escape | gdk::Key::q => {
                if let Some(w) = w.upgrade() {
                    w.close();
                }
            }
            gdk::Key::Right
            | gdk::Key::Down
            | gdk::Key::space
            | gdk::Key::Page_Down
            | gdk::Key::Return => s2(1),
            gdk::Key::Left | gdk::Key::Up | gdk::Key::BackSpace | gdk::Key::Page_Up => s2(-1),
            gdk::Key::Home => s2(-10_000),
            gdk::Key::End => s2(10_000),
            _ => return glib::Propagation::Proceed,
        }
        glib::Propagation::Stop
    });
    win.add_controller(keys);
    let click = gtk::GestureClick::new();
    click.set_button(0);
    click.connect_released(move |g, _, _, _| step(if g.current_button() == 3 { -1 } else { 1 }));
    overlay.add_controller(click);
    // the hint fades after a few seconds
    let h = hint.clone();
    glib::timeout_add_local_once(std::time::Duration::from_secs(4), move || {
        h.set_opacity(0.0)
    });

    // prefer a second display when there is one
    let display = gtk::prelude::WidgetExt::display(&win);
    let monitors = display.monitors();
    if monitors.n_items() > 1 {
        if let Some(mon) = monitors
            .item(monitors.n_items() - 1)
            .and_downcast::<gdk::Monitor>()
        {
            win.fullscreen_on_monitor(&mon);
        }
    } else {
        win.fullscreen();
    }
    win.present();
}
