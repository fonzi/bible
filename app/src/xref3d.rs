//! Cross-references in three dimensions, as a cube or a globe you can turn.
//!
//! Cube: across is where a cross-reference starts, down is where it lands
//! (as in the flat matrix), and depth is how far back in time it reaches:
//! the citing book's date of writing minus the cited book's.
//!
//! Globe: the Old Testament spirals over the northern half and the New
//! Testament over the southern half. Links within a Testament stay near
//! its ring; links between them cut through the middle.

use gtk::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Shape {
    Cube,
    Globe,
}

/// Bins per axis: fine enough to show structure, coarse enough to turn smoothly.
const BINS: usize = 160;
const RING_BINS: usize = 240;

/// Books in the traditional order of writing, earliest first (approximate
/// dates). The cube gives each its own depth.
const WRITTEN: [&str; 66] = [
    "Job", "Gen", "Exod", "Lev", "Num", "Deut", "Josh", "Judg", "Ruth", "Ps", "Song", "Prov",
    "Eccl", "1Sam", "2Sam", "Obad", "Joel", "Jonah", "Amos", "Hos", "Mic", "Isa", "Zeph", "Nah",
    "Hab", "Jer", "Lam", "Ezek", "1Kgs", "2Kgs", "Dan", "Hag", "Zech", "Esth", "1Chr", "2Chr",
    "Ezra", "Neh", "Mal", "Jas", "Gal", "1Thess", "2Thess", "1Cor", "2Cor", "Rom", "Mark", "Eph",
    "Col", "Phlm", "Phil", "Matt", "Luke", "Acts", "1Tim", "Titus", "1Pet", "2Tim", "2Pet", "Heb",
    "Jude", "John", "1John", "2John", "3John", "Rev",
];

/// Depth for a book, 0 (earliest written) to 1 (latest).
pub fn written_depth(book: &str) -> f64 {
    WRITTEN.iter().position(|b| *b == book).unwrap_or(65) as f64 / 65.0
}


/// A point or line in the model, with its weight and colour position.
struct Dot {
    p: [f64; 3],
    w: f64,
    t: f64,
}
struct Line {
    a: [f64; 3],
    b: [f64; 3],
    w: f64,
    t: f64,
}

#[derive(Default)]
struct Model {
    dots: Vec<Dot>,
    lines: Vec<Line>,
    /// labels placed in the model: position and text
    labels: Vec<([f64; 3], String)>,
}

pub struct Space {
    pub area: gtk::DrawingArea,
    shape: Cell<Shape>,
    yaw: Cell<f64>,
    pitch: Cell<f64>,
    zoom: Cell<f64>,
    drag_from: Cell<(f64, f64)>,
    model: RefCell<Model>,
    /// chapters (book, chapter, verses) and links (chapter a, chapter b, count)
    data: RefCell<(Vec<(String, u16, u16)>, Vec<(u16, u16, u16)>)>,
}

const HOME: (f64, f64, f64) = (-0.85, 0.45, 0.8);

impl Space {
    pub fn new() -> Rc<Space> {
        let area = gtk::DrawingArea::new();
        area.set_hexpand(true);
        area.set_vexpand(true);
        area.set_focusable(true);
        let s = Rc::new(Space {
            area: area.clone(),
            shape: Cell::new(Shape::Cube),
            yaw: Cell::new(HOME.0),
            pitch: Cell::new(HOME.1),
            zoom: Cell::new(HOME.2),
            drag_from: Cell::new((0.0, 0.0)),
            model: RefCell::new(Model::default()),
            data: RefCell::new((vec![], vec![])),
        });
        let w = Rc::downgrade(&s);
        area.set_draw_func(move |_, cr, wd, ht| {
            if let Some(s) = w.upgrade() {
                s.draw(cr, wd as f64, ht as f64);
            }
        });
        // drag to turn
        let drag = gtk::GestureDrag::new();
        let w = Rc::downgrade(&s);
        drag.connect_drag_begin(move |_, _, _| {
            if let Some(s) = w.upgrade() {
                s.drag_from.set((s.yaw.get(), s.pitch.get()));
            }
        });
        let w = Rc::downgrade(&s);
        drag.connect_drag_update(move |_, dx, dy| {
            if let Some(s) = w.upgrade() {
                let (y0, p0) = s.drag_from.get();
                s.yaw.set(y0 + dx * 0.008);
                s.pitch.set((p0 + dy * 0.008).clamp(-1.5, 1.5));
                s.area.queue_draw();
            }
        });
        area.add_controller(drag);
        // scroll to zoom
        let scroll = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
        let w = Rc::downgrade(&s);
        scroll.connect_scroll(move |_, _, dy| {
            if let Some(s) = w.upgrade() {
                s.zoom.set((s.zoom.get() * 1.15f64.powf(-dy)).clamp(0.4, 6.0));
                s.area.queue_draw();
            }
            gtk::glib::Propagation::Stop
        });
        area.add_controller(scroll);
        area.set_tooltip_text(Some("Drag to turn, scroll to zoom"));
        s
    }

    pub fn reset(&self) {
        self.yaw.set(HOME.0);
        self.pitch.set(HOME.1);
        // the globe has no corners, so it can open larger than the cube
        self.zoom.set(if self.shape.get() == Shape::Globe { 1.35 } else { HOME.2 });
        self.area.queue_draw();
    }

    pub fn zoom_by(&self, f: f64) {
        self.zoom.set((self.zoom.get() * f).clamp(0.4, 6.0));
        self.area.queue_draw();
    }

    pub fn set_data(&self, chs: Vec<(String, u16, u16)>, links: Vec<(u16, u16, u16)>) {
        *self.data.borrow_mut() = (chs, links);
        self.rebuild();
    }

    pub fn set_shape(&self, shape: Shape) {
        if self.shape.get() != shape {
            self.shape.set(shape);
            self.rebuild();
        }
        self.reset();
    }

    fn rebuild(&self) {
        let data = self.data.borrow();
        let m = match self.shape.get() {
            Shape::Cube => cube(&data.0, &data.1),
            Shape::Globe => globe(&data.0, &data.1),
        };
        *self.model.borrow_mut() = m;
        self.area.queue_draw();
    }

    fn project(&self, p: [f64; 3], w: f64, h: f64) -> (f64, f64, f64) {
        let (sy, cy) = self.yaw.get().sin_cos();
        let (sp, cp) = self.pitch.get().sin_cos();
        // turn round the vertical, then tilt
        let x = p[0] * cy + p[2] * sy;
        let z = -p[0] * sy + p[2] * cy;
        let y = p[1] * cp - z * sp;
        let z = p[1] * sp + z * cp;
        let f = 3.2;
        let k = f / (f + z) * self.zoom.get() * w.min(h) * 0.27;
        (w / 2.0 + x * k, h / 2.0 - y * k, z)
    }

    fn draw(&self, cr: &gtk::cairo::Context, w: f64, h: f64) {
        let dark = adw::StyleManager::default().is_dark();
        let fg = if dark { 1.0 } else { 0.0 };
        let m = self.model.borrow();
        cr.set_line_cap(gtk::cairo::LineCap::Round);
        // frame: the cube's edges or the globe's two rings and equator
        cr.set_source_rgba(fg, fg, fg, 0.22);
        cr.set_line_width(1.0);
        match self.shape.get() {
            Shape::Cube => {
                let c = [-1.0, 1.0];
                for &a in &c {
                    for &b in &c {
                        for axis in 0..3 {
                            let mut p = [0.0; 3];
                            let mut q = [0.0; 3];
                            let (i, j) = ((axis + 1) % 3, (axis + 2) % 3);
                            p[axis] = -1.0;
                            q[axis] = 1.0;
                            p[i] = a;
                            q[i] = a;
                            p[j] = b;
                            q[j] = b;
                            let (x0, y0, _) = self.project(p, w, h);
                            let (x1, y1, _) = self.project(q, w, h);
                            cr.move_to(x0, y0);
                            cr.line_to(x1, y1);
                        }
                    }
                }
                let _ = cr.stroke();
            }
            Shape::Globe => {
                // a few lines of latitude and longitude, the equator firmer
                for lat in [-1.0, -0.5, 0.0, 0.5, 1.0] {
                    cr.set_source_rgba(fg, fg, fg, if lat == 0.0 { 0.3 } else { 0.12 });
                    for i in 0..=96 {
                        let lon = i as f64 / 96.0 * std::f64::consts::TAU;
                        let (x, y, _) = self.project(sphere(lat, lon), w, h);
                        if i == 0 {
                            cr.move_to(x, y);
                        } else {
                            cr.line_to(x, y);
                        }
                    }
                    let _ = cr.stroke();
                }
                cr.set_source_rgba(fg, fg, fg, 0.12);
                for m in 0..6 {
                    let lon = m as f64 / 6.0 * std::f64::consts::PI;
                    for i in 0..=96 {
                        let lat = i as f64 / 96.0 * std::f64::consts::TAU;
                        let p = [lat.cos() * lon.cos(), lat.sin(), lat.cos() * lon.sin()];
                        let (x, y, _) = self.project(p, w, h);
                        if i == 0 {
                            cr.move_to(x, y);
                        } else {
                            cr.line_to(x, y);
                        }
                    }
                    let _ = cr.stroke();
                }
            }
        }
        // links, nearest last so they sit on top
        let mut order: Vec<(f64, usize)> = Vec::new();
        cr.set_operator(gtk::cairo::Operator::Over);
        match self.shape.get() {
            Shape::Cube => {
                for (i, d) in m.dots.iter().enumerate() {
                    order.push((self.project(d.p, w, h).2, i));
                }
                order.sort_by(|a, b| b.0.total_cmp(&a.0));
                let size = (w.min(h) / BINS as f64 * 0.32 * self.zoom.get()).max(1.0);
                for (_, i) in order {
                    let d = &m.dots[i];
                    let (x, y, _) = self.project(d.p, w, h);
                    let (r, g, b) = hue(d.t);
                    cr.set_source_rgba(r, g, b, (0.08 + d.w * 0.7).min(1.0));
                    cr.rectangle(x - size / 2.0, y - size / 2.0, size, size);
                    let _ = cr.fill();
                }
            }
            Shape::Globe => {
                for (i, l) in m.lines.iter().enumerate() {
                    let mid = [
                        (l.a[0] + l.b[0]) / 2.0,
                        (l.a[1] + l.b[1]) / 2.0,
                        (l.a[2] + l.b[2]) / 2.0,
                    ];
                    order.push((self.project(mid, w, h).2, i));
                }
                order.sort_by(|a, b| b.0.total_cmp(&a.0));
                for (_, i) in order {
                    let l = &m.lines[i];
                    let (x0, y0, _) = self.project(l.a, w, h);
                    let (x1, y1, _) = self.project(l.b, w, h);
                    let (r, g, b) = hue(l.t);
                    cr.set_source_rgba(r, g, b, 0.03 + l.w * l.w * 0.45);
                    cr.set_line_width(0.5 + l.w * 1.4);
                    cr.move_to(x0, y0);
                    cr.line_to(x1, y1);
                    let _ = cr.stroke();
                }
            }
        }
        cr.set_operator(gtk::cairo::Operator::Over);
        // labels
        cr.select_font_face("Sans", gtk::cairo::FontSlant::Normal, gtk::cairo::FontWeight::Normal);
        cr.set_font_size(12.0);
        for (p, text) in &m.labels {
            let (x, y, z) = self.project(*p, w, h);
            let a = if z > 0.6 { 0.45 } else { 0.85 };
            cr.set_source_rgba(fg, fg, fg, a);
            if let Ok(ext) = cr.text_extents(text) {
                cr.move_to(x - ext.width() / 2.0, y);
                let _ = cr.show_text(text);
            }
        }
    }
}

fn sphere(lat: f64, lon: f64) -> [f64; 3] {
    [lat.cos() * lon.cos(), lat.sin(), lat.cos() * lon.sin()]
}

/// Red for near neighbours through to violet for the far ends of the Bible.
fn hue(t: f64) -> (f64, f64, f64) {
    let h = t.clamp(0.0, 1.0) * 0.78;
    let i = (h * 6.0).floor();
    let f = h * 6.0 - i;
    let (v, s) = (0.92, 0.85);
    let (p, q, u) = (v * (1.0 - s), v * (1.0 - s * f), v * (1.0 - s * (1.0 - f)));
    match i as i32 % 6 {
        0 => (v, u, p),
        1 => (q, v, p),
        2 => (p, v, u),
        3 => (p, q, v),
        4 => (u, p, v),
        _ => (v, p, q),
    }
}

/// The index of the first New Testament chapter.
fn nt_start(chs: &[(String, u16, u16)]) -> usize {
    chs.iter().position(|c| c.0 == "Matt").unwrap_or(chs.len())
}

fn cube(chs: &[(String, u16, u16)], links: &[(u16, u16, u16)]) -> Model {
    let n = chs.len().max(1);
    let bin = |c: u16| (c as usize * BINS / n).min(BINS - 1);
    // (from bin, to bin, reach) -> count. Depth is how far back in time the
    // reference reaches: the citing book's date of writing minus the cited one's
    let mut cells: std::collections::HashMap<(usize, usize, i16), u32> = Default::default();
    for &(a, b, k) in links {
        for (from, to) in [(a, b), (b, a)] {
            let (Some(fb), Some(tb)) = (chs.get(from as usize), chs.get(to as usize)) else { continue };
            let d = ((written_depth(&fb.0) - written_depth(&tb.0)) * 65.0).round() as i16;
            *cells.entry((bin(from), bin(to), d)).or_default() += k as u32;
        }
    }
    let max = cells.values().copied().max().unwrap_or(1) as f64;
    let coord = |i: usize| i as f64 / (BINS - 1) as f64 * 2.0 - 1.0;
    // a small, fixed spread in depth turns each book's sheet into a cloud
    let spread = |x: usize, y: usize| {
        let h = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
        ((h >> 40) as f64 / (1u64 << 24) as f64 - 0.5) * 0.028
    };
    let dots = cells
        .into_iter()
        .map(|((x, y, z), c)| Dot {
            p: [coord(x), -coord(y), (z as f64 / 65.0 + spread(x, y)).clamp(-1.0, 1.0)],
            w: (c as f64).ln_1p() / max.ln_1p(),
            t: (x as f64 - y as f64).abs() / BINS as f64,
        })
        .collect();
    let mut labels = Vec::new();
    for (book, name) in [("Gen", "Genesis"), ("Ps", "Psalms"), ("Matt", "Matthew"), ("Rev", "Revelation")] {
        if let Some(i) = chs.iter().position(|c| c.0 == book) {
            let u = coord(bin(i as u16));
            labels.push(([u, 1.12, -1.0], name.to_string()));
            // the two axes share Genesis's corner
            if book != "Gen" {
                labels.push(([-1.18, -u, -1.0], name.to_string()));
            }
        }
    }
    labels.push(([0.55, 1.3, -1.0], "cited from →".into()));
    labels.push(([-1.45, 0.0, -1.0], "cited ↓".into()));
    labels.push(([1.12, -1.12, -1.0], "cites later writings".into()));
    labels.push(([1.12, -1.12, 0.0], "same time".into()));
    labels.push(([1.12, -1.12, 1.0], "reaches far back".into()));
    Model { dots, lines: vec![], labels }
}

fn globe(chs: &[(String, u16, u16)], links: &[(u16, u16, u16)]) -> Model {
    let nt = nt_start(chs);
    let n_ot = nt.max(1);
    let n_nt = (chs.len() - nt).max(1);
    // each chapter's place: (old?, bin round its ring)
    let place = |c: u16| -> (bool, usize) {
        let c = c as usize;
        if c < nt {
            (true, c * RING_BINS / n_ot)
        } else {
            (false, (c - nt) * RING_BINS / n_nt)
        }
    };
    // each Testament spirals over its half: Genesis by the North Pole down
    // to Malachi at the equator, Matthew at the equator down to Revelation
    // by the South Pole
    let pos = |(old, b): (bool, usize)| {
        let u = b as f64 / RING_BINS as f64;
        let lat = if old { 1.48 - u * 1.40 } else { -0.08 - u * 1.40 };
        sphere(lat, u * std::f64::consts::TAU)
    };
    let mut pairs: std::collections::HashMap<((bool, usize), (bool, usize)), u32> = Default::default();
    for &(a, b, k) in links {
        let (pa, pb) = (place(a), place(b));
        // short links hug the surface and only draw the spiral; keep the
        // ones that cross the globe
        let (p, q) = (pos(pa), pos(pb));
        if p[0] * q[0] + p[1] * q[1] + p[2] * q[2] > 0.84 {
            continue;
        }
        let key = if pa <= pb { (pa, pb) } else { (pb, pa) };
        *pairs.entry(key).or_default() += k as u32;
    }
    let max = pairs.values().copied().max().unwrap_or(1) as f64;
    let total = chs.len().max(1) as f64;
    // the faintest pairs only blur the picture
    let floor = (max.ln_1p() * 0.3).exp_m1();
    let lines = pairs
        .into_iter()
        .filter(|(_, c)| *c as f64 >= floor)
        .map(|((a, b), c)| {
            let ga = if a.0 { a.1 as f64 / RING_BINS as f64 * n_ot as f64 } else { n_ot as f64 + a.1 as f64 / RING_BINS as f64 * n_nt as f64 };
            let gb = if b.0 { b.1 as f64 / RING_BINS as f64 * n_ot as f64 } else { n_ot as f64 + b.1 as f64 / RING_BINS as f64 * n_nt as f64 };
            Line {
                a: pos(a),
                b: pos(b),
                w: (c as f64).ln_1p() / max.ln_1p(),
                t: (ga - gb).abs() / total,
            }
        })
        .collect();
    let mut labels = Vec::new();
    for (book, name) in [
        ("Gen", "Genesis"),
        ("Ps", "Psalms"),
        ("Isa", "Isaiah"),
        ("Matt", "Matthew"),
        ("Acts", "Acts"),
        ("Rom", "Romans"),
        ("Rev", "Revelation"),
    ] {
        if let Some(i) = chs.iter().position(|c| c.0 == book) {
            let p = pos(place(i as u16));
            labels.push(([p[0] * 1.12, p[1] * 1.12, p[2] * 1.12], name.to_string()));
        }
    }
    labels.push(([0.0, 1.25, 0.0], "Old Testament".into()));
    labels.push(([0.0, -1.3, 0.0], "New Testament".into()));
    Model { dots: vec![], lines, labels }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> (Vec<(String, u16, u16)>, Vec<(u16, u16, u16)>) {
        let chs = vec![
            ("Gen".to_string(), 1, 31),
            ("Ps".to_string(), 1, 6),
            ("Isa".to_string(), 1, 31),
            ("Matt".to_string(), 1, 25),
            ("Rom".to_string(), 1, 32),
        ];
        let links = vec![(0, 3, 4), (2, 3, 10), (1, 4, 2), (3, 4, 1)];
        (chs, links)
    }

    #[test]
    fn every_book_has_an_age() {
        assert_eq!(written_depth("Job"), 0.0);
        assert_eq!(written_depth("Rev"), 1.0);
        let mut w = WRITTEN.to_vec();
        w.sort();
        w.dedup();
        assert_eq!(w.len(), 66);
    }

    #[test]
    fn cube_points_stay_inside_the_cube() {
        let (chs, links) = sample();
        let m = cube(&chs, &links);
        assert!(!m.dots.is_empty());
        for d in &m.dots {
            assert!(d.p.iter().all(|v| (-1.0..=1.0).contains(v)), "{:?}", d.p);
            assert!((0.0..=1.0).contains(&d.w));
        }
        // Romans citing Psalms reaches back; Psalms citing Romans reaches forward
        assert!(m.dots.iter().any(|d| d.p[2] > 0.3));
        assert!(m.dots.iter().any(|d| d.p[2] < -0.3));
    }

    #[test]
    fn globe_puts_the_testaments_on_their_halves() {
        let (chs, links) = sample();
        let m = globe(&chs, &links);
        assert!(!m.lines.is_empty());
        for l in &m.lines {
            for p in [l.a, l.b] {
                let r = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
                assert!((r - 1.0).abs() < 1e-9);
                assert!(p[1].abs() > 0.05, "off the equator, in its own half");
            }
        }
        // Isaiah to Matthew crosses from north to south
        assert!(m.lines.iter().any(|l| l.a[1] > 0.0 && l.b[1] < 0.0 || l.a[1] < 0.0 && l.b[1] > 0.0));
    }
}
