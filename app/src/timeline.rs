//! Timeline: lanes of biblical history, kings, prophets, who was in power,
//! world events and church history, drawn on one zoomable time axis.

use crate::ctx::{sidebar_button, Ctx, Section};
use adw::prelude::*;
use bible_core::timeline::{self, ref_label, span, Event};
use gtk::{gdk, glib, pango};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const FIRST: i32 = -2200;
const LAST: i32 = 2030;
const MARGIN: f64 = 24.0;
const ERA_H: f64 = 30.0;
const AXIS_H: f64 = 26.0;
const LANE_HEAD: f64 = 24.0;
const ROW_H: f64 = 28.0;
const BAR_H: f64 = 22.0;
const LANE_GAP: f64 = 14.0;

fn lane_colour(id: &str) -> (f64, f64, f64) {
    let hex = match id {
        "bible" => 0x3584e4,
        "united" => 0x9141ac,
        "israel" => 0x26a269,
        "judah" => 0xe66100,
        "prophets" => 0xc061cb,
        "powers" => 0x865e3c,
        "rulers" => 0xa51d2d,
        "world" => 0x2190a4,
        "church" => 0xc64600,
        _ => 0x77767b,
    };
    (
        ((hex >> 16) & 0xff) as f64 / 255.0,
        ((hex >> 8) & 0xff) as f64 / 255.0,
        (hex & 0xff) as f64 / 255.0,
    )
}

const ERA_COLOURS: [u32; 6] = [0x99c1f1, 0x8ff0a4, 0xf9f06b, 0xffbe6f, 0xdc8add, 0xcdab8f];

fn rgb(hex: u32) -> (f64, f64, f64) {
    (
        ((hex >> 16) & 0xff) as f64 / 255.0,
        ((hex >> 8) & 0xff) as f64 / 255.0,
        (hex & 0xff) as f64 / 255.0,
    )
}

/// Where each event ended up on the canvas, for clicks.
#[derive(Clone, Copy)]
struct Hit {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    idx: usize,
}

pub struct TimelineView {
    ctx: Rc<Ctx>,
    root: adw::ToolbarView,
    split: adw::OverlaySplitView,
    area: gtk::DrawingArea,
    scroll: gtk::ScrolledWindow,
    /// pixels per year
    zoom: Cell<f64>,
    hits: RefCell<Vec<Hit>>,
    selected: Cell<Option<usize>>,
    /// The passage the reader asked about: (start, end, label)
    marker: RefCell<Option<(i32, i32, String)>>,
    detail: gtk::Box,
    me: RefCell<std::rc::Weak<TimelineView>>,
}

impl TimelineView {
    pub fn new(ctx: &Rc<Ctx>) -> Rc<TimelineView> {
        let area = gtk::DrawingArea::new();
        let scroll = gtk::ScrolledWindow::builder()
            .child(&area)
            .vexpand(true)
            .hexpand(true)
            .build();

        let detail = gtk::Box::new(gtk::Orientation::Vertical, 10);
        detail.set_margin_top(18);
        detail.set_margin_bottom(18);
        detail.set_margin_start(18);
        detail.set_margin_end(18);
        let detail_scroll = gtk::ScrolledWindow::builder()
            .child(&detail)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build();
        let panel = adw::ToolbarView::new();
        let phb = adw::HeaderBar::new();
        phb.set_show_title(false);
        phb.set_show_end_title_buttons(false);
        phb.set_show_start_title_buttons(false);
        let close = gtk::Button::from_icon_name("window-close-symbolic");
        close.add_css_class("flat");
        close.set_tooltip_text(Some(crate::i18n::t("Close").as_str()));
        phb.pack_end(&close);
        panel.add_top_bar(&phb);
        panel.set_content(Some(&detail_scroll));

        let split = adw::OverlaySplitView::new();
        split.set_sidebar_position(gtk::PackType::End);
        split.set_content(Some(&scroll));
        split.set_sidebar(Some(&panel));
        split.set_show_sidebar(false);
        split.set_max_sidebar_width(380.0);
        split.set_min_sidebar_width(300.0);

        let root = adw::ToolbarView::new();
        let hb = adw::HeaderBar::new();
        hb.pack_start(&sidebar_button(ctx));
        hb.set_title_widget(Some(&adw::WindowTitle::new("Timeline", "")));
        let zoom_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        zoom_box.add_css_class("linked");
        let zout = gtk::Button::from_icon_name("zoom-out-symbolic");
        zout.set_tooltip_text(Some(crate::i18n::t("Zoom out (Ctrl+scroll)").as_str()));
        let zin = gtk::Button::from_icon_name("zoom-in-symbolic");
        zin.set_tooltip_text(Some(crate::i18n::t("Zoom in (Ctrl+scroll)").as_str()));
        zoom_box.append(&zout);
        zoom_box.append(&zin);
        let jump = gtk::MenuButton::new();
        jump.set_label(crate::i18n::t("Jump to").as_str());
        jump.set_tooltip_text(Some(crate::i18n::t("Jump to a period").as_str()));
        hb.pack_end(&zoom_box);
        hb.pack_end(&jump);
        root.add_top_bar(&hb);
        root.set_content(Some(&split));

        let t = Rc::new(TimelineView {
            ctx: ctx.clone(),
            root,
            split,
            area,
            scroll,
            zoom: Cell::new(3.0),
            hits: RefCell::new(vec![]),
            selected: Cell::new(None),
            marker: RefCell::new(None),
            detail,
            me: RefCell::new(std::rc::Weak::new()),
        });
        *t.me.borrow_mut() = Rc::downgrade(&t);
        let w = Rc::downgrade(&t);
        close.connect_clicked(move |_| {
            if let Some(t) = w.upgrade() {
                t.selected.set(None);
                t.split.set_show_sidebar(false);
                t.area.queue_draw();
            }
        });

        // eras menu
        let pop = gtk::Popover::new();
        let list = gtk::ListBox::new();
        list.set_selection_mode(gtk::SelectionMode::None);
        list.add_css_class("navigation-sidebar");
        for era in &timeline::timeline().eras {
            let row = gtk::Box::new(gtk::Orientation::Vertical, 2);
            row.set_margin_top(4);
            row.set_margin_bottom(4);
            let a = gtk::Label::new(Some(&era.title));
            a.set_xalign(0.0);
            let b = gtk::Label::new(Some(&span(era.start, era.end, true)));
            b.set_xalign(0.0);
            b.add_css_class("dim-label");
            b.add_css_class("caption");
            row.append(&a);
            row.append(&b);
            list.append(&row);
        }
        let sc = gtk::ScrolledWindow::builder()
            .child(&list)
            .propagate_natural_height(true)
            .max_content_height(480)
            .min_content_width(240)
            .build();
        pop.set_child(Some(&sc));
        jump.set_popover(Some(&pop));
        let w = Rc::downgrade(&t);
        list.connect_row_activated(move |_, row| {
            pop.popdown();
            if let (Some(t), Some(era)) = (
                w.upgrade(),
                timeline::timeline().eras.get(row.index() as usize),
            ) {
                t.scroll_to(era.start, era.end);
            }
        });

        let w = Rc::downgrade(&t);
        zin.connect_clicked(move |_| {
            if let Some(t) = w.upgrade() {
                t.zoom_by(1.5, None);
            }
        });
        let w = Rc::downgrade(&t);
        zout.connect_clicked(move |_| {
            if let Some(t) = w.upgrade() {
                t.zoom_by(1.0 / 1.5, None);
            }
        });

        let w = Rc::downgrade(&t);
        t.area.set_draw_func(move |area, cr, width, height| {
            if let Some(t) = w.upgrade() {
                t.draw(area, cr, width as f64, height as f64);
            }
        });
        // lane titles stay pinned to the left edge while scrolling
        let a = t.area.clone();
        t.scroll
            .hadjustment()
            .connect_value_changed(move |_| a.queue_draw());

        let click = gtk::GestureClick::new();
        let w = Rc::downgrade(&t);
        click.connect_released(move |_, _, x, y| {
            let Some(t) = w.upgrade() else { return };
            let hit = t
                .hits
                .borrow()
                .iter()
                .find(|h| x >= h.x && x <= h.x + h.w && y >= h.y && y <= h.y + h.h)
                .copied();
            match hit {
                Some(h) => t.select(h.idx),
                None => {
                    t.selected.set(None);
                    t.split.set_show_sidebar(false);
                    t.area.queue_draw();
                }
            }
        });
        t.area.add_controller(click);

        t.area.set_has_tooltip(true);
        let w = Rc::downgrade(&t);
        t.area.connect_query_tooltip(move |_, x, y, _, tip| {
            let Some(t) = w.upgrade() else { return false };
            let hit = t
                .hits
                .borrow()
                .iter()
                .find(|h| {
                    x as f64 >= h.x
                        && x as f64 <= h.x + h.w
                        && y as f64 >= h.y
                        && y as f64 <= h.y + h.h
                })
                .copied();
            let Some(h) = hit else { return false };
            let e = &timeline::timeline().events[h.idx];
            tip.set_markup(Some(&format!(
                "<b>{}</b>\n{}",
                glib::markup_escape_text(&e.title),
                span(e.start, e.end(), e.approx)
            )));
            true
        });

        let motion = gtk::EventControllerMotion::new();
        let w = Rc::downgrade(&t);
        motion.connect_motion(move |_, x, y| {
            let Some(t) = w.upgrade() else { return };
            let over = t
                .hits
                .borrow()
                .iter()
                .any(|h| x >= h.x && x <= h.x + h.w && y >= h.y && y <= h.y + h.h);
            t.area
                .set_cursor_from_name(if over { Some("pointer") } else { None });
        });
        t.area.add_controller(motion);

        // Ctrl+scroll zooms around the pointer
        let sc = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
        let w = Rc::downgrade(&t);
        let pointer = Rc::new(Cell::new(0.0f64));
        let p2 = pointer.clone();
        let m2 = gtk::EventControllerMotion::new();
        m2.connect_motion(move |_, x, _| p2.set(x));
        t.area.add_controller(m2);
        sc.connect_scroll(move |c, _, dy| {
            let Some(t) = w.upgrade() else {
                return glib::Propagation::Proceed;
            };
            if !c
                .current_event_state()
                .contains(gdk::ModifierType::CONTROL_MASK)
            {
                return glib::Propagation::Proceed;
            }
            t.zoom_by(if dy < 0.0 { 1.25 } else { 0.8 }, Some(pointer.get()));
            glib::Propagation::Stop
        });
        sc.set_propagation_phase(gtk::PropagationPhase::Capture);
        t.scroll.add_controller(sc);

        let w = Rc::downgrade(&t);
        ctx.set_open_time(move |start, end, label| {
            if let Some(t) = w.upgrade() {
                *t.marker.borrow_mut() = Some((start, end, label.to_string()));
                t.area.queue_draw();
                let (s, e) = (start, end);
                let tw = Rc::downgrade(&t);
                // wait for the section to be laid out before scrolling
                glib::idle_add_local_once(move || {
                    if let Some(t) = tw.upgrade() {
                        t.scroll_to(s, e);
                    }
                });
            }
        });

        let a = t.area.clone();
        adw::StyleManager::default().connect_dark_notify(move |_| a.queue_draw());
        t.resize();
        t
    }

    fn x_of(&self, year: i32) -> f64 {
        MARGIN + (year - FIRST) as f64 * self.zoom.get()
    }

    fn year_of(&self, x: f64) -> f64 {
        FIRST as f64 + (x - MARGIN) / self.zoom.get()
    }

    fn zoom_by(&self, f: f64, anchor: Option<f64>) {
        let adj = self.scroll.hadjustment();
        let anchor_x = anchor.unwrap_or(adj.value() + adj.page_size() / 2.0);
        let year = self.year_of(anchor_x);
        let offset = anchor_x - adj.value();
        self.zoom.set((self.zoom.get() * f).clamp(0.25, 40.0));
        self.resize();
        let nx = self.x_of(year.round() as i32);
        let adj2 = adj.clone();
        // the adjustment's upper bound only grows after the next layout
        glib::idle_add_local_once(move || adj2.set_value(nx - offset));
    }

    fn scroll_to(&self, start: i32, end: i32) {
        let adj = self.scroll.hadjustment();
        let page = adj.page_size().max(600.0);
        // zoom so the span fills about half the view, within reason
        let years = (end - start).max(1) as f64;
        let want = (page * 0.5 / years).clamp(1.5, 30.0);
        if (want / self.zoom.get() - 1.0).abs() > 0.3 && years > 5.0 {
            self.zoom.set(want);
            self.resize();
        }
        let mid = (self.x_of(start) + self.x_of(end)) / 2.0;
        let adj2 = adj.clone();
        glib::idle_add_local_once(move || adj2.set_value(mid - adj2.page_size() / 2.0));
    }

    fn resize(&self) {
        let w = self.x_of(LAST) + MARGIN;
        self.area.set_content_width(w as i32);
        let h = self.layout(None).1;
        self.area.set_content_height(h as i32);
        self.area.queue_draw();
    }

    /// Pack each lane's events into rows so labels don't overlap.
    /// Returns per-event (x, y, w) and total height.
    fn layout(&self, pl: Option<&pango::Layout>) -> (Vec<(usize, f64, f64, f64, bool)>, f64) {
        let tl = timeline::timeline();
        let tmp;
        let pl = match pl {
            Some(p) => p,
            None => {
                tmp = self.area.create_pango_layout(None);
                &tmp
            }
        };
        let mut out = Vec::new();
        let mut y = ERA_H + AXIS_H + 8.0;
        for lane in &tl.lanes {
            y += LANE_HEAD;
            let mut rows: Vec<f64> = vec![];
            let mut idxs: Vec<usize> = tl
                .events
                .iter()
                .enumerate()
                .filter(|(_, e)| e.lane == lane.id)
                .map(|(i, _)| i)
                .collect();
            idxs.sort_by_key(|i| tl.events[*i].start);
            for i in idxs {
                let e = &tl.events[i];
                let x0 = self.x_of(e.start);
                let x1 = self.x_of(e.end()).max(x0 + 6.0);
                pl.set_text(&e.title);
                let tw = pl.pixel_size().0 as f64 + 12.0;
                let is_span = e.end.is_some() && x1 - x0 > 8.0;
                // text goes inside a wide bar, otherwise after it
                let right = if is_span && x1 - x0 >= tw {
                    x1
                } else {
                    x1 + tw + 4.0
                };
                let row = match rows.iter().position(|r| *r + 6.0 <= x0) {
                    Some(r) => r,
                    None => {
                        rows.push(f64::MIN);
                        rows.len() - 1
                    }
                };
                rows[row] = right;
                out.push((i, x0, y + row as f64 * ROW_H, x1 - x0, is_span));
            }
            y += rows.len().max(1) as f64 * ROW_H + LANE_GAP;
        }
        (out, y + 12.0)
    }

    fn draw(&self, area: &gtk::DrawingArea, cr: &gtk::cairo::Context, width: f64, height: f64) {
        let tl = timeline::timeline();
        let dark = adw::StyleManager::default().is_dark();
        let fg = if dark {
            (1.0, 1.0, 1.0)
        } else {
            (0.0, 0.0, 0.0)
        };
        // GNOME's default accent; libadwaita 1.5 has no accent colour API
        let (ar, ag, ab) = rgb(0x3584e4);
        let pl = area.create_pango_layout(None);
        let mut font = pango::FontDescription::from_string("Sans 9.5");
        pl.set_font_description(Some(&font));
        let left = self.scroll.hadjustment().value();
        let view_w = self.scroll.hadjustment().page_size();
        let (first_vis, last_vis) = (
            self.year_of(left - 200.0),
            self.year_of(left + view_w + 200.0),
        );

        // eras
        for (i, era) in tl.eras.iter().enumerate() {
            let x0 = self.x_of(era.start);
            let x1 = self.x_of(era.end);
            let (r, g, b) = rgb(ERA_COLOURS[i % ERA_COLOURS.len()]);
            cr.set_source_rgba(r, g, b, if dark { 0.35 } else { 0.55 });
            cr.rectangle(x0, 0.0, x1 - x0, ERA_H);
            let _ = cr.fill();
            // a faint wash down the whole canvas
            cr.set_source_rgba(r, g, b, if dark { 0.05 } else { 0.08 });
            cr.rectangle(x0, ERA_H, x1 - x0, height - ERA_H);
            let _ = cr.fill();
            pl.set_text(&era.title);
            font.set_weight(pango::Weight::Bold);
            pl.set_font_description(Some(&font));
            let (tw, th) = pl.pixel_size();
            if (tw as f64) < x1 - x0 - 8.0 {
                // keep the title in view while its era is on screen
                let tx = (x0 + 6.0).max(left + 8.0).min(x1 - tw as f64 - 6.0);
                cr.set_source_rgba(fg.0, fg.1, fg.2, 0.85);
                cr.move_to(tx, (ERA_H - th as f64) / 2.0);
                pangocairo::functions::show_layout(cr, &pl);
            }
        }
        font.set_weight(pango::Weight::Normal);
        pl.set_font_description(Some(&font));

        // axis
        let z = self.zoom.get();
        let step = [10, 25, 50, 100, 250, 500]
            .into_iter()
            .find(|s| *s as f64 * z >= 70.0)
            .unwrap_or(500);
        let mut yr = (FIRST / step) * step;
        while yr <= LAST {
            if (yr as f64) >= first_vis && (yr as f64) <= last_vis {
                let x = self.x_of(yr);
                cr.set_source_rgba(fg.0, fg.1, fg.2, 0.12);
                cr.rectangle(x, ERA_H, 1.0, height - ERA_H);
                let _ = cr.fill();
                let label = if yr == 0 {
                    "BC | AD".to_string()
                } else {
                    timeline::year(yr, false)
                };
                pl.set_text(&label);
                let (tw, _) = pl.pixel_size();
                cr.set_source_rgba(fg.0, fg.1, fg.2, 0.55);
                cr.move_to(x - tw as f64 / 2.0, ERA_H + 5.0);
                pangocairo::functions::show_layout(cr, &pl);
            }
            yr += step;
        }

        // the passage being read
        if let Some((s, e, label)) = self.marker.borrow().as_ref() {
            let x0 = self.x_of(*s);
            let x1 = self.x_of(*e).max(x0 + 3.0);
            cr.set_source_rgba(ar, ag, ab, 0.16);
            cr.rectangle(x0, ERA_H, x1 - x0, height - ERA_H);
            let _ = cr.fill();
            cr.set_source_rgba(ar, ag, ab, 0.9);
            cr.rectangle(x0, ERA_H, 2.0, height - ERA_H);
            let _ = cr.fill();
            pl.set_markup(&format!(
                "<b>{}</b>",
                glib::markup_escape_text(&format!("You are reading {label}"))
            ));
            let (tw, th) = pl.pixel_size();
            let tx = x0 + 6.0;
            cr.set_source_rgba(ar, ag, ab, 1.0);
            rounded(
                cr,
                tx - 4.0,
                ERA_H + AXIS_H - 6.0,
                tw as f64 + 8.0,
                th as f64 + 4.0,
                6.0,
            );
            let _ = cr.fill();
            cr.set_source_rgb(1.0, 1.0, 1.0);
            cr.move_to(tx, ERA_H + AXIS_H - 4.0);
            pangocairo::functions::show_layout(cr, &pl);
            pl.set_attributes(None);
        }

        // lanes
        let (items, _) = self.layout(Some(&pl));
        let mut hits = Vec::with_capacity(items.len());
        let mut lane_y: Vec<(String, f64)> = vec![];
        for (i, x, y, w, is_span) in &items {
            let e: &Event = &tl.events[*i];
            if !lane_y.iter().any(|(l, _)| l == &e.lane) {
                lane_y.push((e.lane.clone(), *y));
            }
            let (r, g, b) = lane_colour(&e.lane);
            let selected = self.selected.get() == Some(*i);
            let by = y + (ROW_H - BAR_H) / 2.0;
            pl.set_text(&e.title);
            let (tw, th) = pl.pixel_size();
            let ty = by + (BAR_H - th as f64) / 2.0;
            if *is_span {
                cr.set_source_rgba(r, g, b, if e.approx { 0.55 } else { 0.85 });
                rounded(cr, *x, by, *w, BAR_H, 5.0);
                let _ = cr.fill();
                if selected {
                    cr.set_source_rgb(ar, ag, ab);
                    cr.set_line_width(2.5);
                    rounded(cr, *x - 1.5, by - 1.5, *w + 3.0, BAR_H + 3.0, 6.0);
                    let _ = cr.stroke();
                }
                if (tw as f64) + 12.0 <= *w {
                    cr.set_source_rgb(1.0, 1.0, 1.0);
                    let tx = (x + 6.0).max(left + 6.0).min(x + w - tw as f64 - 6.0);
                    cr.move_to(tx, ty);
                } else {
                    cr.set_source_rgba(fg.0, fg.1, fg.2, 0.85);
                    cr.move_to(x + w + 5.0, ty);
                }
                pangocairo::functions::show_layout(cr, &pl);
                hits.push(Hit {
                    x: *x,
                    y: by,
                    w: w.max(tw as f64 + 12.0),
                    h: BAR_H,
                    idx: *i,
                });
            } else {
                let cx = *x + 3.0;
                let cy = by + BAR_H / 2.0;
                cr.set_source_rgb(r, g, b);
                cr.arc(
                    cx,
                    cy,
                    if selected { 7.0 } else { 5.5 },
                    0.0,
                    std::f64::consts::TAU,
                );
                let _ = cr.fill();
                if selected {
                    cr.set_source_rgb(ar, ag, ab);
                    cr.set_line_width(2.5);
                    cr.arc(cx, cy, 9.0, 0.0, std::f64::consts::TAU);
                    let _ = cr.stroke();
                }
                cr.set_source_rgba(fg.0, fg.1, fg.2, 0.85);
                cr.move_to(cx + 10.0, ty);
                pangocairo::functions::show_layout(cr, &pl);
                hits.push(Hit {
                    x: cx - 8.0,
                    y: by,
                    w: tw as f64 + 22.0,
                    h: BAR_H,
                    idx: *i,
                });
            }
        }
        // lane titles, pinned left
        font.set_weight(pango::Weight::Bold);
        font.set_size((8.5 * pango::SCALE as f64) as i32);
        pl.set_font_description(Some(&font));
        for (lane, y) in lane_y {
            let Some(l) = tl.lane(&lane) else { continue };
            pl.set_text(&l.title.to_uppercase());
            let (tw, th) = pl.pixel_size();
            let tx = left + 10.0;
            let ty = y - LANE_HEAD + 4.0;
            let bg = if dark {
                (0.14, 0.14, 0.14)
            } else {
                (0.98, 0.98, 0.98)
            };
            cr.set_source_rgba(bg.0, bg.1, bg.2, 0.85);
            rounded(
                cr,
                tx - 6.0,
                ty - 2.0,
                tw as f64 + 12.0,
                th as f64 + 4.0,
                5.0,
            );
            let _ = cr.fill();
            let (r, g, b) = lane_colour(&lane);
            cr.set_source_rgb(r, g, b);
            cr.move_to(tx, ty);
            pangocairo::functions::show_layout(cr, &pl);
        }
        let _ = width;
        *self.hits.borrow_mut() = hits;
    }

    fn select(&self, idx: usize) {
        self.selected.set(Some(idx));
        self.area.queue_draw();
        self.fill_detail(idx);
        self.split.set_show_sidebar(true);
    }

    fn fill_detail(&self, idx: usize) {
        let tl = timeline::timeline();
        let e = &tl.events[idx];
        while let Some(c) = self.detail.first_child() {
            self.detail.remove(&c);
        }
        let lane = tl.lane(&e.lane);
        let kicker = gtk::Label::new(Some(
            &lane.map(|l| l.title.to_uppercase()).unwrap_or_default(),
        ));
        kicker.set_xalign(0.0);
        kicker.add_css_class("caption-heading");
        kicker.add_css_class("dim-label");
        let title = gtk::Label::new(Some(&e.title));
        title.set_xalign(0.0);
        title.set_wrap(true);
        title.add_css_class("title-2");
        let date = gtk::Label::new(Some(&span(e.start, e.end(), e.approx)));
        date.set_xalign(0.0);
        date.add_css_class("heading");
        self.detail.append(&kicker);
        self.detail.append(&title);
        self.detail.append(&date);
        if let Some(era) = tl.era_at(e.start) {
            let l = gtk::Label::new(Some(&format!("Period: {}", era.title)));
            l.set_xalign(0.0);
            l.add_css_class("dim-label");
            self.detail.append(&l);
        }
        if !e.summary.is_empty() {
            let s = gtk::Label::new(Some(&e.summary));
            s.set_xalign(0.0);
            s.set_wrap(true);
            s.set_margin_top(6);
            s.add_css_class("body");
            self.detail.append(&s);
        }
        if !e.refs.is_empty() {
            let h = gtk::Label::new(Some(crate::i18n::t("In the Bible").as_str()));
            h.set_xalign(0.0);
            h.set_margin_top(10);
            h.add_css_class("heading");
            self.detail.append(&h);
            let list = gtk::ListBox::new();
            list.add_css_class("boxed-list");
            list.set_selection_mode(gtk::SelectionMode::None);
            for r in &e.refs {
                let Some((rf, label)) = ref_label(r) else {
                    continue;
                };
                let row = adw::ActionRow::builder()
                    .title(label.as_str())
                    .activatable(true)
                    .build();
                row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
                let ctx = self.ctx.clone();
                row.connect_activated(move |_| ctx.open_ref(&rf));
                list.append(&row);
            }
            self.detail.append(&list);
        }
        // what else was happening
        let around: Vec<&Event> = tl
            .events
            .iter()
            .enumerate()
            .filter(|(i, o)| {
                *i != idx
                    && o.lane != e.lane
                    && o.start <= e.end()
                    && o.end() >= e.start
                    && o.end() - o.start < 1500
            })
            .map(|(_, o)| o)
            .take(8)
            .collect();
        if !around.is_empty() {
            let h = gtk::Label::new(Some(crate::i18n::t("At the same time").as_str()));
            h.set_xalign(0.0);
            h.set_margin_top(10);
            h.add_css_class("heading");
            self.detail.append(&h);
            let list = gtk::ListBox::new();
            list.add_css_class("boxed-list");
            list.set_selection_mode(gtk::SelectionMode::None);
            for o in around {
                let lane = tl
                    .lane(&o.lane)
                    .map(|l| l.title.clone())
                    .unwrap_or_default();
                let row = adw::ActionRow::builder()
                    .title(glib::markup_escape_text(&o.title).as_str())
                    .subtitle(
                        glib::markup_escape_text(&format!(
                            "{lane} · {}",
                            span(o.start, o.end(), o.approx)
                        ))
                        .as_str(),
                    )
                    .activatable(true)
                    .build();
                let oi = tl
                    .events
                    .iter()
                    .position(|x| std::ptr::eq(x, o))
                    .unwrap_or(0);
                let w = self.me.borrow().clone();
                row.connect_activated(move |_| {
                    if let Some(t) = w.upgrade() {
                        t.select(oi);
                        let ev = &timeline::timeline().events[oi];
                        t.scroll_to(ev.start, ev.end());
                    }
                });
                list.append(&row);
            }
            self.detail.append(&list);
        }
        let mut src = lane.map(|l| l.source.clone()).unwrap_or_default();
        if let Some(n) = &e.note {
            src = format!("{n} {src}");
        }
        if !src.is_empty() {
            let l = gtk::Label::new(Some(&src));
            l.set_xalign(0.0);
            l.set_wrap(true);
            l.set_margin_top(12);
            l.add_css_class("caption");
            l.add_css_class("dim-label");
            self.detail.append(&l);
        }
    }
}

fn rounded(cr: &gtk::cairo::Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    let r = r.min(w / 2.0).min(h / 2.0);
    cr.new_sub_path();
    cr.arc(x + w - r, y + r, r, -std::f64::consts::FRAC_PI_2, 0.0);
    cr.arc(x + w - r, y + h - r, r, 0.0, std::f64::consts::FRAC_PI_2);
    cr.arc(
        x + r,
        y + h - r,
        r,
        std::f64::consts::FRAC_PI_2,
        std::f64::consts::PI,
    );
    cr.arc(
        x + r,
        y + r,
        r,
        std::f64::consts::PI,
        1.5 * std::f64::consts::PI,
    );
    cr.close_path();
}

impl Section for TimelineView {
    fn id(&self) -> &'static str {
        "timeline"
    }
    fn title(&self) -> &'static str { crate::i18n::ts("Timeline") }
    fn icon(&self) -> &'static str {
        "document-open-recent-symbolic"
    }
    fn widget(&self) -> gtk::Widget {
        self.root.clone().upcast()
    }
    fn shown(&self) {
        if self.marker.borrow().is_none() && self.scroll.hadjustment().value() == 0.0 {
            let w = self.me.borrow().clone();
            glib::idle_add_local_once(move || {
                if let Some(t) = w.upgrade() {
                    t.zoom.set(3.0);
                    t.resize();
                    let adj = t.scroll.hadjustment();
                    let x = t.x_of(-1050);
                    glib::idle_add_local_once(move || adj.set_value(x));
                }
            });
        }
    }
}
