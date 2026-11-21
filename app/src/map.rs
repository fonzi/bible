//! The Map tab: places named in the chapter, on real geography.

use crate::ctx::Ctx;
use adw::prelude::*;
use bible_core::places::{self, geography, Place, ShapeKind};
use bible_core::Ref;
use gtk::{glib, pango};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The default view: the land of Israel.
const HOME: (f64, f64, f64, f64) = (33.6, 29.6, 36.9, 33.6);

#[derive(Clone)]
struct Pin {
    place: Place,
    verses: Vec<u16>,
    /// named in the selected verse
    hot: bool,
    /// verses naming it (book view)
    count: usize,
}

pub struct MapView {
    ctx: Rc<Ctx>,
    pub root: gtk::Box,
    area: gtk::DrawingArea,
    list: gtk::ListBox,
    caption: gtk::Label,
    pins: RefCell<Vec<Pin>>,
    /// centre (lon, lat) and degrees of longitude per 100 px
    centre: Cell<(f64, f64)>,
    scale: Cell<f64>,
    book: RefCell<(String, u16)>,
    /// screen positions of pins, for clicks
    hits: RefCell<Vec<(f64, f64, usize)>>,
    me: RefCell<std::rc::Weak<MapView>>,
}

fn hex(c: u32) -> (f64, f64, f64) {
    (
        ((c >> 16) & 0xff) as f64 / 255.0,
        ((c >> 8) & 0xff) as f64 / 255.0,
        (c & 0xff) as f64 / 255.0,
    )
}

impl MapView {
    pub fn new(ctx: &Rc<Ctx>) -> Rc<MapView> {
        let area = gtk::DrawingArea::new();
        area.set_content_height(340);
        area.set_vexpand(false);
        area.add_css_class("map-canvas");
        let frame = gtk::Frame::new(None);
        frame.set_child(Some(&area));
        frame.add_css_class("map-frame");

        let caption = gtk::Label::new(None);
        caption.set_xalign(0.0);
        caption.set_wrap(true);
        caption.add_css_class("dim-label");
        caption.add_css_class("caption");

        let list = gtk::ListBox::new();
        list.add_css_class("boxed-list");
        list.set_selection_mode(gtk::SelectionMode::None);

        let credit = gtk::Label::new(Some(places::CREDIT));
        credit.set_xalign(0.0);
        credit.set_wrap(true);
        credit.add_css_class("dim-label");
        credit.add_css_class("caption");
        credit.set_margin_top(6);

        let body = gtk::Box::new(gtk::Orientation::Vertical, 10);
        body.set_margin_top(12);
        body.set_margin_bottom(18);
        body.set_margin_start(12);
        body.set_margin_end(12);
        body.append(&frame);
        body.append(&caption);
        body.append(&list);
        body.append(&credit);
        let scroll = gtk::ScrolledWindow::builder()
            .child(&body)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .propagate_natural_height(true)
            .build();
        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        root.append(&scroll);

        let m = Rc::new(MapView {
            ctx: ctx.clone(),
            root,
            area,
            list,
            caption,
            pins: RefCell::new(vec![]),
            centre: Cell::new(((HOME.0 + HOME.2) / 2.0, (HOME.1 + HOME.3) / 2.0)),
            scale: Cell::new(1.0),
            book: RefCell::new((String::new(), 0)),
            hits: RefCell::new(vec![]),
            me: RefCell::new(std::rc::Weak::new()),
        });
        *m.me.borrow_mut() = Rc::downgrade(&m);

        let w = Rc::downgrade(&m);
        m.area.set_draw_func(move |area, cr, wd, ht| {
            if let Some(m) = w.upgrade() {
                m.draw(area, cr, wd as f64, ht as f64);
            }
        });

        // drag to pan
        let drag = gtk::GestureDrag::new();
        let start = Rc::new(Cell::new((0.0, 0.0)));
        let w = Rc::downgrade(&m);
        let s2 = start.clone();
        drag.connect_drag_begin(move |_, _, _| {
            if let Some(m) = w.upgrade() {
                s2.set(m.centre.get());
            }
        });
        let w = Rc::downgrade(&m);
        drag.connect_drag_update(move |_, dx, dy| {
            if let Some(m) = w.upgrade() {
                let (lon, lat) = start.get();
                let k = m.scale.get() / 100.0;
                let coslat = lat.to_radians().cos();
                m.centre.set((lon - dx * k, lat + dy * k * coslat));
                m.area.queue_draw();
            }
        });
        m.area.add_controller(drag);

        // scroll to zoom
        let sc = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
        let w = Rc::downgrade(&m);
        sc.connect_scroll(move |_, _, dy| {
            let Some(m) = w.upgrade() else {
                return glib::Propagation::Proceed;
            };
            m.scale
                .set((m.scale.get() * if dy > 0.0 { 1.25 } else { 0.8 }).clamp(0.05, 25.0));
            m.area.queue_draw();
            glib::Propagation::Stop
        });
        m.area.add_controller(sc);

        // click a pin
        let click = gtk::GestureClick::new();
        let w = Rc::downgrade(&m);
        click.connect_released(move |g, n, x, y| {
            let Some(m) = w.upgrade() else { return };
            if n != 1 {
                return;
            }
            let hit = m
                .hits
                .borrow()
                .iter()
                .filter(|(px, py, _)| (px - x).hypot(py - y) < 12.0)
                .min_by(|a, b| {
                    (a.0 - x)
                        .hypot(a.1 - y)
                        .total_cmp(&(b.0 - x).hypot(b.1 - y))
                })
                .map(|h| h.2);
            if let Some(i) = hit {
                g.set_state(gtk::EventSequenceState::Claimed);
                m.show_pin(i, x, y);
            }
        });
        m.area.add_controller(click);

        let a = m.area.clone();
        adw::StyleManager::default().connect_dark_notify(move |_| a.queue_draw());
        m
    }

    /// Show the places named across a whole book (for its introduction):
    /// the most-mentioned ones, without the list below the map.
    pub fn set_book(&self, book: &str) {
        let pins: Vec<Pin> = places::places()
            .in_book(book)
            .into_iter()
            .take(30)
            .map(|(p, n)| Pin {
                place: p.clone(),
                verses: vec![],
                hot: false,
                count: n,
            })
            .collect();
        let n = pins.len();
        *self.pins.borrow_mut() = pins;
        *self.book.borrow_mut() = (book.to_string(), 0);
        self.fit();
        self.list.set_visible(false);
        self.caption.set_text(&if n == 0 {
            "No places are named in this book.".to_string()
        } else {
            format!("The {n} places named most often in this book. Drag to move, scroll to zoom.")
        });
        self.area.queue_draw();
    }

    /// Show the places of a chapter; `verse` marks those named in the selected verse.
    pub fn set_chapter(&self, book: &str, chapter: u16, verse: u16) {
        let same = { *self.book.borrow() == (book.to_string(), chapter) };
        let mut pins: Vec<Pin> = places::places()
            .in_chapter(book, chapter)
            .into_iter()
            .map(|(p, vs)| Pin {
                place: p.clone(),
                verses: vs.to_vec(),
                hot: verse > 0 && vs.contains(&verse),
                count: vs.len(),
            })
            .collect();
        pins.sort_by_key(|p| p.verses.first().copied().unwrap_or(0));
        *self.pins.borrow_mut() = pins;
        if !same {
            *self.book.borrow_mut() = (book.to_string(), chapter);
            self.fit();
        }
        self.fill_list(verse);
        self.area.queue_draw();
    }

    fn fit(&self) {
        let pins = self.pins.borrow();
        let (mut x0, mut y0, mut x1, mut y1) = if pins.is_empty() {
            HOME
        } else {
            (f64::MAX, f64::MAX, f64::MIN, f64::MIN)
        };
        for p in pins.iter() {
            x0 = x0.min(p.place.lon);
            x1 = x1.max(p.place.lon);
            y0 = y0.min(p.place.lat);
            y1 = y1.max(p.place.lat);
        }
        // never closer than about 150 km across, so there is context
        let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
        let w = (x1 - x0).max(1.6) * 1.35;
        let h = (y1 - y0).max(1.2) * 1.35;
        let width = self.area.width().max(380) as f64;
        let height = self.area.height().max(340) as f64;
        let coslat = cy.to_radians().cos();
        // degrees of longitude per 100 px that fit both directions
        let s = (w / width * 100.0).max(h / coslat / height * 100.0);
        self.centre.set((cx, cy));
        self.scale.set(s);
    }

    fn project(&self, lon: f64, lat: f64, w: f64, h: f64) -> (f64, f64) {
        let (cx, cy) = self.centre.get();
        let k = 100.0 / self.scale.get();
        let coslat = cy.to_radians().cos();
        (w / 2.0 + (lon - cx) * k, h / 2.0 - (lat - cy) * k / coslat)
    }

    fn draw(&self, area: &gtk::DrawingArea, cr: &gtk::cairo::Context, w: f64, h: f64) {
        let dark = adw::StyleManager::default().is_dark();
        let (sea, land, river, ink) = if dark {
            (0x1c2633, 0x30302e, 0x3d5a7a, 0xffffff)
        } else {
            (0xcfe2f3, 0xf3efe4, 0x8db4dc, 0x241f31)
        };
        let (r, g, b) = hex(sea);
        cr.set_source_rgb(r, g, b);
        cr.paint().ok();

        let path = |pts: &[(f64, f64)]| {
            for (i, (lon, lat)) in pts.iter().enumerate() {
                let (x, y) = self.project(*lon, *lat, w, h);
                if i == 0 {
                    cr.move_to(x, y);
                } else {
                    cr.line_to(x, y);
                }
            }
        };
        let geo = geography();
        // land with its holes
        cr.set_fill_rule(gtk::cairo::FillRule::EvenOdd);
        let (r, g, b) = hex(land);
        cr.set_source_rgb(r, g, b);
        for s in geo
            .iter()
            .filter(|s| matches!(s.kind, ShapeKind::Land | ShapeKind::Hole))
        {
            path(&s.pts);
            cr.close_path();
        }
        cr.fill().ok();
        // coast line
        let (ir, ig, ib) = hex(ink);
        let (rr, rg, rb) = hex(river);
        let (sr, sg, sb) = hex(sea);
        for s in geo {
            match s.kind {
                ShapeKind::Water => {
                    path(&s.pts);
                    cr.close_path();
                    cr.set_source_rgb(sr, sg, sb);
                    cr.fill_preserve().ok();
                    cr.set_source_rgb(rr, rg, rb);
                    cr.set_line_width(0.8);
                    cr.stroke().ok();
                }
                ShapeKind::River => {
                    path(&s.pts);
                    cr.set_source_rgb(rr, rg, rb);
                    cr.set_line_width(1.4);
                    cr.stroke().ok();
                }
                _ => {}
            }
        }

        let pl = area.create_pango_layout(None);
        let mut font = pango::FontDescription::from_string("Sans 8.5");
        pl.set_font_description(Some(&font));
        let mut taken: Vec<(f64, f64, f64, f64)> = vec![];
        let free = |taken: &Vec<(f64, f64, f64, f64)>, r: (f64, f64, f64, f64)| {
            !taken
                .iter()
                .any(|t| r.0 < t.0 + t.2 && r.0 + r.2 > t.0 && r.1 < t.1 + t.3 && r.1 + r.3 > t.1)
        };

        let pins = self.pins.borrow();
        // well-known places for orientation, faint
        let mut landmarks: Vec<&Place> = places::places()
            .list
            .iter()
            .filter(|p| p.refs.len() >= 120)
            .collect();
        landmarks.sort_by_key(|p| std::cmp::Reverse(p.refs.len()));
        // reserve the pins' own spots first
        for p in pins.iter() {
            let (x, y) = self.project(p.place.lon, p.place.lat, w, h);
            taken.push((x - 5.0, y - 5.0, 10.0, 10.0));
        }
        for p in landmarks {
            if pins.iter().any(|q| q.place.name == p.name) {
                continue;
            }
            let (x, y) = self.project(p.lon, p.lat, w, h);
            if x < 0.0 || y < 0.0 || x > w || y > h {
                continue;
            }
            pl.set_text(&p.name);
            let (tw, th) = pl.pixel_size();
            let rect = (x + 5.0, y - th as f64 / 2.0, tw as f64, th as f64);
            if !free(&taken, rect) {
                continue;
            }
            taken.push(rect);
            cr.set_source_rgba(ir, ig, ib, 0.35);
            cr.arc(x, y, 2.2, 0.0, std::f64::consts::TAU);
            cr.fill().ok();
            cr.move_to(rect.0, rect.1);
            pangocairo::functions::show_layout(cr, &pl);
        }

        // the chapter's places
        let mut hits = vec![];
        let accent = hex(0x3584e4);
        let hot = hex(0xe01b24);
        font.set_weight(pango::Weight::Bold);
        font.set_size((9.0 * pango::SCALE as f64) as i32);
        pl.set_font_description(Some(&font));
        // hot pins last, so they sit on top
        let mut order: Vec<usize> = (0..pins.len()).collect();
        order.sort_by_key(|i| pins[*i].hot);
        for i in order {
            let p = &pins[i];
            let (x, y) = self.project(p.place.lon, p.place.lat, w, h);
            hits.push((x, y, i));
            let (r, g, b) = if p.hot { hot } else { accent };
            cr.set_source_rgb(1.0, 1.0, 1.0);
            cr.arc(
                x,
                y,
                if p.hot { 7.0 } else { 5.5 },
                0.0,
                std::f64::consts::TAU,
            );
            cr.fill().ok();
            cr.set_source_rgb(r, g, b);
            cr.arc(
                x,
                y,
                if p.hot { 5.5 } else { 4.0 },
                0.0,
                std::f64::consts::TAU,
            );
            cr.fill().ok();
            pl.set_text(&p.place.name);
            let (tw, th) = pl.pixel_size();
            let (tw, th) = (tw as f64, th as f64);
            // try right, left, above, below
            let spots = [
                (x + 8.0, y - th / 2.0),
                (x - 8.0 - tw, y - th / 2.0),
                (x - tw / 2.0, y - 8.0 - th),
                (x - tw / 2.0, y + 8.0),
            ];
            let spot = spots
                .iter()
                .find(|s| free(&taken, (s.0, s.1, tw, th)))
                .copied();
            if let Some((tx, ty)) = spot.or(if p.hot { Some(spots[0]) } else { None }) {
                taken.push((tx, ty, tw, th));
                // a halo so labels read over coastlines
                cr.move_to(tx, ty);
                pangocairo::functions::layout_path(cr, &pl);
                let (lr, lg, lb) = hex(land);
                cr.set_source_rgba(lr, lg, lb, 0.9);
                cr.set_line_width(3.0);
                cr.stroke().ok();
                cr.set_source_rgb(ir, ig, ib);
                cr.move_to(tx, ty);
                pangocairo::functions::show_layout(cr, &pl);
            }
        }
        *self.hits.borrow_mut() = hits;

        // scale bar
        let km_per_px = self.scale.get() / 100.0 * 111.32 * self.centre.get().1.to_radians().cos();
        let nice = [5.0, 10.0, 20.0, 50.0, 100.0, 200.0, 500.0, 1000.0]
            .into_iter()
            .find(|k| k / km_per_px >= 60.0)
            .unwrap_or(1000.0);
        let len = nice / km_per_px;
        cr.set_source_rgba(ir, ig, ib, 0.7);
        cr.set_line_width(2.0);
        cr.move_to(12.0, h - 14.0);
        cr.line_to(12.0 + len, h - 14.0);
        cr.stroke().ok();
        font.set_weight(pango::Weight::Normal);
        font.set_size((8.0 * pango::SCALE as f64) as i32);
        pl.set_font_description(Some(&font));
        pl.set_text(&format!("{} km", nice as i32));
        cr.move_to(12.0, h - 30.0);
        pangocairo::functions::show_layout(cr, &pl);
    }

    fn fill_list(&self, verse: u16) {
        while let Some(c) = self.list.first_child() {
            self.list.remove(&c);
        }
        let pins = self.pins.borrow();
        let (book, chapter) = self.book.borrow().clone();
        if pins.is_empty() {
            self.caption.set_text(
                "No places are named in this chapter. Drag to move around, scroll to zoom.",
            );
            self.list.set_visible(false);
            return;
        }
        let hot = pins.iter().filter(|p| p.hot).count();
        self.caption.set_text(&match (pins.len(), hot) {
            (1, _) => "1 place named in this chapter.".to_string(),
            (n, 0) => format!("{n} places named in this chapter."),
            (n, h) => format!("{n} places named in this chapter, {h} in verse {verse} (red)."),
        });
        self.list.set_visible(true);
        for (i, p) in pins.iter().enumerate() {
            let vs: Vec<String> = p.verses.iter().map(|v| v.to_string()).collect();
            let sub = format!(
                "{}{}v. {}",
                p.place.brief,
                if p.place.brief.is_empty() { "" } else { " · " },
                vs.join(", ")
            );
            let row = adw::ActionRow::builder()
                .title(glib::markup_escape_text(&p.place.name).as_str())
                .subtitle(glib::markup_escape_text(&sub).as_str())
                .subtitle_lines(2)
                .activatable(true)
                .build();
            let dot = gtk::Image::from_icon_name("mark-location-symbolic");
            if p.hot {
                dot.add_css_class("error");
            } else {
                dot.add_css_class("accent");
            }
            row.add_prefix(&dot);
            let w = self.me.borrow().clone();
            let first = p.verses.first().copied().unwrap_or(1);
            let (bk, ch) = (book.clone(), chapter);
            let goto = gtk::Button::with_label(&format!("v. {first}"));
            goto.add_css_class("flat");
            goto.set_valign(gtk::Align::Center);
            goto.set_tooltip_text(Some("Go to the verse"));
            let ctx = self.ctx.clone();
            goto.connect_clicked(move |_| ctx.open_ref(&Ref::new(&bk, ch, first)));
            row.add_suffix(&goto);
            row.connect_activated(move |_| {
                if let Some(m) = w.upgrade() {
                    m.focus(i);
                }
            });
            self.list.append(&row);
        }
    }

    /// Centre on a pin and open its card.
    fn focus(&self, i: usize) {
        let Some(p) = self.pins.borrow().get(i).cloned() else {
            return;
        };
        self.centre.set((p.place.lon, p.place.lat));
        self.scale.set(self.scale.get().min(1.2));
        self.area.queue_draw();
        let (w, h) = (self.area.width() as f64, self.area.height() as f64);
        self.show_pin(i, w / 2.0, h / 2.0);
    }

    fn show_pin(&self, i: usize, x: f64, y: f64) {
        let Some(p) = self.pins.borrow().get(i).cloned() else {
            return;
        };
        let (book, chapter) = self.book.borrow().clone();
        let pop = gtk::Popover::new();
        pop.set_parent(&self.area);
        pop.set_pointing_to(Some(&gtk::gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
        let b = gtk::Box::new(gtk::Orientation::Vertical, 6);
        b.set_margin_top(6);
        b.set_margin_bottom(6);
        b.set_margin_start(6);
        b.set_margin_end(6);
        b.set_size_request(240, -1);
        let t = gtk::Label::new(Some(&p.place.name));
        t.add_css_class("heading");
        t.set_xalign(0.0);
        b.append(&t);
        if !p.place.brief.is_empty() {
            let d = gtk::Label::new(Some(&p.place.brief));
            d.set_wrap(true);
            d.set_xalign(0.0);
            d.set_max_width_chars(34);
            b.append(&d);
        }
        let n = gtk::Label::new(Some(&format!(
            "Named in {} verses of the Bible",
            p.place.refs.len()
        )));
        n.add_css_class("dim-label");
        n.add_css_class("caption");
        n.set_xalign(0.0);
        b.append(&n);
        if chapter == 0 && p.count > 0 {
            let c = gtk::Label::new(Some(&format!(
                "{} verse{} in this book",
                p.count,
                if p.count == 1 { "" } else { "s" }
            )));
            c.add_css_class("caption");
            c.set_xalign(0.0);
            b.append(&c);
        }
        let flow = gtk::FlowBox::new();
        flow.set_selection_mode(gtk::SelectionMode::None);
        flow.set_max_children_per_line(6);
        for v in &p.verses {
            let btn = gtk::Button::with_label(&format!("{chapter}:{v}"));
            btn.add_css_class("flat");
            let ctx = self.ctx.clone();
            let (bk, v) = (book.clone(), *v);
            let popw = pop.downgrade();
            btn.connect_clicked(move |_| {
                if let Some(p) = popw.upgrade() {
                    p.popdown();
                }
                ctx.open_ref(&Ref::new(&bk, chapter, v));
            });
            flow.insert(&btn, -1);
        }
        b.append(&flow);
        let search = gtk::Button::with_label("Every mention");
        search.add_css_class("pill");
        let ctx = self.ctx.clone();
        let name = p.place.name.clone();
        let popw = pop.downgrade();
        search.connect_clicked(move |_| {
            if let Some(p) = popw.upgrade() {
                p.popdown();
            }
            ctx.open_search(&name);
        });
        b.append(&search);
        pop.set_child(Some(&b));
        pop.connect_closed(|p| {
            let p = p.clone();
            glib::idle_add_local_once(move || p.unparent());
        });
        pop.popup();
    }
}
