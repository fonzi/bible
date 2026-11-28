//! The cross-reference rainbow: every cross-reference drawn as an arc between
//! the two passages, over a bar for each chapter of the Bible. Zoomable down
//! to single verses, plus two experimental views (a circle and a matrix) that
//! bring out the repeating, fractal-like patterns. Built from OpenBible.info's
//! cross-references placed on the chosen Bible's chapters, cached once.

use crate::ctx::{page, Ctx, Section};
use adw::prelude::*;
use bible_core::Ref;
use bible_sword::Module;
use gtk::{cairo, gio, glib, pango};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

/// Chapters of the 66-book canon in order: (book, chapter, verses).
use crate::crossrefs_data::*;
fn help_heading(t: &str) -> gtk::Label {
    let l = gtk::Label::new(Some(t));
    l.add_css_class("heading");
    l.set_xalign(0.0);
    l.set_margin_top(6);
    l
}

fn help_line(name: &str, what: &str) -> gtk::Label {
    let l = gtk::Label::new(None);
    l.set_markup(&format!(
        "<b>{}</b>\n{}",
        glib::markup_escape_text(name),
        glib::markup_escape_text(what)
    ));
    l.set_wrap(true);
    l.set_xalign(0.0);
    l.set_width_chars(36);
    l.set_max_width_chars(36);
    l
}

fn rounded(cr: &cairo::Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    use std::f64::consts::{FRAC_PI_2, PI};
    cr.new_sub_path();
    cr.arc(x + w - r, y + r, r, -FRAC_PI_2, 0.0);
    cr.arc(x + w - r, y + h - r, r, 0.0, FRAC_PI_2);
    cr.arc(x + r, y + h - r, r, FRAC_PI_2, PI);
    cr.arc(x + r, y + r, r, PI, 3.0 * FRAC_PI_2);
    cr.close_path();
}

/// A short note on a bucket label, shown on hover.
fn bucket_help(name: &str) -> Option<&'static str> {
    Some(match name {
        // age
        "Before 1200 BC" => "The oldest layer by traditional dating: Job and the five books of Moses, and Joshua, from the time of the exodus and conquest.",
        "United kingdom, 1050–930 BC" => "Written under Samuel, Saul, David and Solomon, when Israel was one kingdom: the Psalms, the wisdom books and the early histories.",
        "Divided kingdom, 930–586 BC" => "After Solomon the kingdom split into Israel (north) and Judah (south). The prophets who warned both before Assyria and Babylon conquered them.",
        "Exile, 586–538 BC" => "Written after Babylon destroyed Jerusalem and the temple and took Judah’s leaders into exile.",
        "Persian period, 538–400 BC" => "Persia let the exiles return and rebuild the temple. The last books of the Old Testament were written then.",
        "Roman era, AD 45–95" => "The whole New Testament, written within about fifty years under Roman rule, in the order scholars traditionally date it: James and Galatians first, John’s writings last.",
        // writers
        "Moses" => "Jewish and Christian tradition credits Moses with the Torah, the first five books.",
        "Joshua" => "The Talmud (Bava Batra 15a) credits Joshua with the book named after him.",
        "Samuel" => "The Talmud credits the prophet Samuel with Judges, Ruth and the first book of Samuel.",
        "Nathan and Gad" => "1 Chronicles 29:29 says the prophets Nathan and Gad recorded David’s reign, so tradition credits them with finishing Samuel’s work.",
        "Jeremiah" => "The Talmud credits Jeremiah with Kings and Lamentations as well as his own book.",
        "Ezra" => "Tradition credits Ezra the scribe with Chronicles and the book of Ezra.",
        "Nehemiah" => "Nehemiah’s memoir, written in the first person.",
        "David and others" => "About half the psalms are titled “of David”; others are by Asaph, the sons of Korah, Solomon, Moses, Heman, Ethan and unnamed writers.",
        "Solomon" => "Proverbs, Ecclesiastes and the Song of Songs are linked to Solomon by their opening lines.",
        "Isaiah" => "Isaiah son of Amoz, prophet in Jerusalem in the 700s BC.",
        "Ezekiel" => "A priest taken to Babylon in 597 BC, who prophesied among the exiles.",
        "Daniel" => "Daniel, a young noble taken to Babylon, who served its kings and then Persia’s.",
        "The Twelve" => "The twelve shorter prophetic books, Hosea to Malachi. In the Hebrew Bible they fit on one scroll and count as one book; “minor” refers to their length, not their importance.",
        "Unknown" => "Books whose writer isn’t named and on whom tradition doesn’t agree: Esther and Job.",
        "Matthew" => "Matthew (Levi), the tax collector who became one of the Twelve apostles.",
        "Mark" => "John Mark, who early writers say set down Peter’s preaching in Rome.",
        "Peter" => "Simon Peter, leader of the Twelve apostles.",
        "Luke" => "Luke the physician, Paul’s companion, wrote both his gospel and its sequel, Acts, for a man named Theophilus.",
        "John" => "John son of Zebedee, the “beloved disciple”, traditionally wrote the fourth gospel, three letters and Revelation.",
        "Paul" => "Paul (Saul of Tarsus), the Pharisee who became the apostle to the Gentiles. Thirteen letters carry his name.",
        "Hebrews (unknown)" => "Long counted with Paul’s letters, but it names no author; Origen said only God knows who wrote it. Apollos and Barnabas are other guesses.",
        "James" => "James, the brother of Jesus and leader of the church in Jerusalem.",
        "Jude" => "Jude (Judas), another brother of Jesus and of James.",
        // language
        "Hebrew" => "Biblical Hebrew, the language of ancient Israel, written right to left in consonants only; the vowel marks were added in the Middle Ages.",
        "Hebrew and Aramaic" => "Mostly Hebrew, but Ezra 4:8–6:18 and 7:12–26 and Daniel 2:4–7:28 are in Aramaic, the shared language of the Babylonian and Persian empires. (Jeremiah 10:11 is a single Aramaic verse.)",
        "Koine Greek" => "The everyday “common” Greek spoken around the eastern Mediterranean after Alexander the Great, the language the whole New Testament was written in.",
        // culture
        "Ancient Near East" => "The world of the patriarchs, between Mesopotamia, Canaan and Egypt, before Israel was a nation.",
        "Egypt and the wilderness" => "Israel’s slavery in Egypt, the exodus and forty years in the Sinai desert.",
        "Israel among the Canaanites" => "Settling the land among Canaanite peoples and the Philistines, up to the kingdom of David and Solomon.",
        "Israel and Judah under Assyria" => "The divided kingdoms in the shadow of the Assyrian empire, which destroyed Israel in 722 BC.",
        "Judah under Babylon" => "Judah’s last years as Babylon rose, and the fall of Jerusalem in 586 BC.",
        "Exiles in Babylon and Persia" => "Jews living as a minority in the courts and cities of Babylon and Persia.",
        "Persian Judea" => "The return from exile to a small Persian province and the rebuilding of Jerusalem.",
        "Roman Judea and Galilee" => "The Jewish homeland under Rome and the Herods: the setting of Jesus’ life, and letters to Jewish Christians.",
        "The Greco-Roman world" => "The cities of the Roman Empire, from Jerusalem to Rome, where the church spread among Jews and Gentiles.",
        // genre
        "Law" => "Torah: God’s instruction to Israel, with the story of how it was given. In Literary form, chapters that are mainly commandments and regulations.",
        "History" => "Israel’s story from the conquest of Canaan to the return from exile.",
        "Poetry and wisdom" => "Hebrew poetry, built on parallel lines, and wisdom: how to live well and why the righteous suffer.",
        "Major prophets" => "The long prophetic books (plus Lamentations, traditionally by Jeremiah).",
        "Minor prophets" => "The twelve shorter prophetic books; “minor” means short, not less important.",
        "Apocalyptic" => "Visions of the end of history in vivid symbols: beasts, numbers, heavenly courts.",
        "Gospels" => "Four accounts of the life, death and resurrection of Jesus.",
        "Church history" => "Acts: how the church spread from Jerusalem to Rome.",
        "Paul’s letters" => "Letters from Paul to churches and friends, ordered in the Bible longest first.",
        "General letters" => "Letters to the wider church by Peter, John, James, Jude and the author of Hebrews.",
        // Hebrew Bible
        "Torah" => "Torah means “instruction”: the five books of Moses.",
        "Nevi’im (Prophets)" => "The Former Prophets (Joshua to Kings) and the Latter Prophets (Isaiah, Jeremiah, Ezekiel and the Twelve).",
        "Ketuvim (Writings)" => "Psalms, wisdom, the five festival scrolls (Song, Ruth, Lamentations, Ecclesiastes, Esther), Daniel, Ezra–Nehemiah and Chronicles.",
        "New Testament" => "The 27 books of the New Testament in their usual order.",
        // literary form
        "Songs and poems" => "Psalms and hymns, laments and love poems, plus songs inside the stories: Moses and Miriam at the sea, Deborah, Hannah, Mary’s Magnificat, the hymns of Revelation.",
        "Parables" => "Chapters with parables: short stories that make one sharp point, like the Good Samaritan, the Prodigal Son and the Sower, and earlier ones told by Jotham, Nathan and Ezekiel.",
        "Prayers" => "Chapters built around a long prayer, like Solomon at the temple, Daniel’s confession and Jesus’ prayer in John 17.",
        "Sermons and speeches" => "Long addresses: Moses in Deuteronomy, the Sermon on the Mount, Jesus’ farewell discourse, Peter, Stephen and Paul in Acts.",
        "Wisdom sayings" => "Proverbs and the reflections of Ecclesiastes.",
        "Prophecy" => "Prophets’ oracles: warnings, calls to return and promises of restoration.",
        "Visions" => "Dreams and visions: Isaiah in the temple, Ezekiel’s wheels and dry bones, Daniel’s beasts, Zechariah’s night visions, Revelation.",
        "Genealogies" => "Family trees, from Adam to Noah to the lists of returning exiles and the ancestry of Jesus.",
        "Letters" => "Letters written to churches and to people.",
        "Narrative" => "Stories: everything else told as events, from creation to Paul in Rome.",
        _ => return None,
    })
}

/// Where each chapter sits along the drawing; shared with the drawing thread.
#[derive(Default)]
struct Geo {
    /// verses in each chapter (by chapter index)
    verses: Vec<u16>,
    /// slot along the drawing of each chapter
    slot_of: Vec<u16>,
}

impl Geo {
    /// Position along the drawing, 0..1, of a chapter (and verse).
    fn u(&self, ch: u16, verse: Option<u16>) -> f64 {
        let n = self.verses.len().max(1) as f64;
        let frac = match (verse, self.verses.get(ch as usize)) {
            (Some(v), Some(vs)) if *vs > 0 => (v as f64 - 0.5) / *vs as f64,
            _ => 0.5,
        };
        let slot = self.slot_of.get(ch as usize).copied().unwrap_or(ch);
        (slot as f64 + frac.clamp(0.0, 1.0)) / n
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Rainbow,
    Circle,
    Matrix,
}

/// Zoom and pan: screen = world × s + (ox, oy).
#[derive(Clone, Copy, PartialEq)]
struct View {
    s: f64,
    ox: f64,
    oy: f64,
}

impl View {
    const HOME: View = View { s: 1.0, ox: 0.0, oy: 0.0 };
}

struct Cached {
    w: i32,
    h: i32,
    mode: Mode,
    order: u32,
    view: View,
    surf: cairo::ImageSurface,
}

pub struct CrossRefs {
    ctx: Rc<Ctx>,
    root: adw::ToolbarView,
    title: adw::WindowTitle,
    stack: gtk::Stack,
    area: gtk::DrawingArea,
    /// the cube and globe, shown in place of the flat drawing
    space: Rc<crate::xref3d::Space>,
    pics: gtk::Stack,
    is_3d: Cell<bool>,
    status: adw::StatusPage,
    progress: gtk::ProgressBar,
    info: gtk::Label,
    /// (book, chapter, verses) for each bar
    chs: RefCell<Vec<(String, u16, u16)>>,
    vlinks: RefCell<Arc<Vec<VLink>>>,
    links: RefCell<Arc<Vec<(u16, u16, u16)>>>,
    geo: RefCell<Arc<Geo>>,
    /// a drawing is being made off the main thread
    inflight: Cell<bool>,
    again: Cell<bool>,
    busy: gtk::Box,
    source: RefCell<String>,
    help: gtk::ToggleButton,
    cache: RefCell<Option<Cached>>,
    rerender: RefCell<Option<glib::SourceId>>,
    mode: Cell<Mode>,
    view: Cell<View>,
    pointer: Cell<(f64, f64)>,
    drag_from: Cell<View>,
    dragged: Cell<bool>,
    /// selected chapter, and verse when zoomed in far enough to pick one
    selected: Cell<Option<(u16, Option<u16>)>>,
    /// book layout: chapter → slot along the drawing, and back
    order: Cell<u32>,
    slot_of: RefCell<Vec<u16>>,
    /// bucket starts (0..1) and names, for the bucketed orders
    groups: RefCell<Vec<(f64, String)>>,
    chap_at: RefCell<Vec<u16>>,
    /// zoom to apply once the drawing has a size: (position, zoom)
    pending: Cell<Option<(f64, f64)>>,
    modes: gtk::DropDown,
    read: gtk::Button,
    /// explanations of the current order's groups, in the side panel
    groups_box: gtk::Box,
    /// where labels were drawn, for hover explanations
    labels: RefCell<Vec<(f64, f64, f64, f64, String)>>,
    module: RefCell<String>,
    ver_dd: gtk::DropDown,
    ver_names: RefCell<Vec<String>>,
    ver_filling: Cell<bool>,
    gen: Cell<u64>,
    me: RefCell<std::rc::Weak<CrossRefs>>,
}

const BAR_MAX: f64 = 60.0;
const MARGIN: f64 = 16.0;
const MAX_ZOOM: f64 = 200.0;

impl CrossRefs {
    pub fn new(ctx: &Rc<Ctx>) -> Rc<CrossRefs> {
        let area = gtk::DrawingArea::new();
        area.set_hexpand(true);
        area.set_vexpand(true);
        let info = gtk::Label::new(None);
        info.set_xalign(0.0);
        info.set_wrap(true);
        info.add_css_class("dim-label");
        info.set_margin_start(18);
        info.set_margin_end(18);
        info.set_margin_bottom(12);
        let view = gtk::Box::new(gtk::Orientation::Vertical, 6);
        // a spinner while the drawing is made in the background
        // a large spinner in the middle while the drawing is made
        let spinner = crate::loader::new(96);
        let busy_label = gtk::Label::new(Some("Drawing cross-references…"));
        busy_label.add_css_class("title-4");
        let busy = gtk::Box::new(gtk::Orientation::Vertical, 14);
        busy.append(&spinner);
        busy.append(&busy_label);
        busy.set_halign(gtk::Align::Center);
        busy.set_valign(gtk::Align::Center);
        busy.add_css_class("osd");
        busy.add_css_class("busy-card");
        busy.set_can_target(false);
        busy.set_visible(false);
        let space = crate::xref3d::Space::new();
        let pics = gtk::Stack::new();
        pics.set_transition_type(gtk::StackTransitionType::Crossfade);
        pics.add_named(&area, Some("flat"));
        pics.add_named(&space.area, Some("space"));
        let overlay = gtk::Overlay::new();
        overlay.set_child(Some(&pics));
        overlay.add_overlay(&busy);
        overlay.set_vexpand(true);
        view.append(&overlay);
        // the info line, with a button to read what's selected
        let read = gtk::Button::with_label("Read");
        read.add_css_class("pill");
        read.add_css_class("suggested-action");
        read.set_valign(gtk::Align::Center);
        read.set_margin_end(18);
        read.set_margin_bottom(12);
        read.set_visible(false);
        info.set_hexpand(true);
        // one line, never widening the window; the ⓘ panel holds the explanations
        info.set_max_width_chars(1);
        info.set_wrap(false);
        info.set_ellipsize(gtk::pango::EllipsizeMode::End);
        let foot = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        foot.append(&info);
        foot.append(&read);
        view.append(&foot);
        let progress = gtk::ProgressBar::new();
        progress.set_show_text(true);
        progress.set_size_request(320, -1);
        let status = adw::StatusPage::builder()
            .icon_name("emblem-shared-symbolic")
            .title("Drawing every cross-reference")
            .build();
        let pbox = gtk::Box::new(gtk::Orientation::Vertical, 12);
        pbox.set_halign(gtk::Align::Center);
        pbox.append(&progress);
        status.set_child(Some(&pbox));
        let stack = gtk::Stack::new();
        stack.add_named(&status, Some("status"));
        stack.add_named(&view, Some("view"));
        // the drawing, with the explanations sliding in beside it
        let content = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        stack.set_hexpand(true);
        content.append(&stack);
        let (root, hb) = page(ctx, "Cross-references", "", &content);
        let title = adw::WindowTitle::new("Cross-references", "");
        hb.set_title_widget(Some(&title));
        let back = gtk::Button::from_icon_name("go-previous-symbolic");
        back.set_tooltip_text(Some("Back to the Bible"));
        let c = ctx.clone();
        back.connect_clicked(move |_| c.go("bible"));
        hb.pack_start(&back);
        // which Bible's links to draw, without going back to the reader
        let ver_dd = gtk::DropDown::from_strings(&[]);
        ver_dd.set_tooltip_text(Some("Bible version"));
        hb.pack_start(&ver_dd);
        // views and zoom
        let modes = gtk::DropDown::from_strings(&[
            "Rainbow",
            "Circle",
            "Matrix",
            "Cube (3D)",
            "Globe (3D)",
            "Möbius strip (3D)",
        ]);
        modes.set_tooltip_text(Some("How to draw the cross-references"));
        let zin = gtk::Button::from_icon_name("zoom-in-symbolic");
        zin.set_tooltip_text(Some("Zoom in (or scroll over the drawing)"));
        let zout = gtk::Button::from_icon_name("zoom-out-symbolic");
        zout.set_tooltip_text(Some("Zoom out"));
        let zfit = gtk::Button::from_icon_name("zoom-fit-best-symbolic");
        zfit.set_tooltip_text(Some("Reset: whole Bible, nothing selected (Esc)"));
        let zoom = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        zoom.add_css_class("linked");
        zoom.append(&zout);
        zoom.append(&zfit);
        zoom.append(&zin);
        let orders = gtk::DropDown::from_strings(&ORDERS);
        orders.set_tooltip_text(Some("How to order the books"));
        let help = gtk::ToggleButton::builder()
            .icon_name("help-about-symbolic")
            .tooltip_text("What the views, orders and groups mean")
            .build();
        let text = gtk::Box::new(gtk::Orientation::Vertical, 10);
        text.set_margin_top(18);
        text.set_margin_bottom(18);
        text.set_margin_start(18);
        text.set_margin_end(18);
        text.append(&help_heading("How to use it"));
        for (n, w) in [
            ("Zoom", "Scroll, or use the − ⤢ + buttons. Zoom in far enough and single verses can be picked."),
            ("Move", "Drag the picture."),
            ("Pick", "Click a chapter or verse to see only its links. Esc or ⤢ clears it."),
            ("Read", "Double-click, or use the Read button, to open the passage."),
        ] {
            text.append(&help_line(n, w));
        }
        let groups_box = gtk::Box::new(gtk::Orientation::Vertical, 10);
        text.append(&groups_box);
        text.append(&help_heading("Views"));
        for h in MODE_HELP {
            let (n, w) = h.split_once(": ").unwrap_or((h, ""));
            let mut c = w.chars();
            let w: String = c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default();
            text.append(&help_line(n, &w));
        }
        text.append(&help_heading("Orders"));
        for (n, w) in ORDERS.iter().zip(ORDER_HELP) {
            text.append(&help_line(n, w));
        }
        let sc = gtk::ScrolledWindow::builder()
            .child(&text)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .width_request(340)
            .vexpand(true)
            .build();
        let panel = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        panel.append(&gtk::Separator::new(gtk::Orientation::Vertical));
        panel.append(&sc);
        let revealer = gtk::Revealer::builder()
            .transition_type(gtk::RevealerTransitionType::SlideLeft)
            .child(&panel)
            .reveal_child(false)
            .build();
        help.bind_property("active", &revealer, "reveal-child").sync_create().build();

        content.append(&revealer);
        hb.pack_end(&help);
        hb.pack_end(&zoom);
        hb.pack_end(&orders);
        hb.pack_end(&modes);
        let x = Rc::new(CrossRefs {
            ctx: ctx.clone(),
            root,
            title,
            stack,
            area,
            space: space.clone(),
            pics: pics.clone(),
            is_3d: Cell::new(false),
            status,
            progress,
            info,
            chs: RefCell::new(vec![]),
            vlinks: RefCell::new(Arc::new(vec![])),
            links: RefCell::new(Arc::new(vec![])),
            geo: RefCell::new(Arc::new(Geo::default())),
            inflight: Cell::new(false),
            again: Cell::new(false),
            busy: busy.clone(),
            source: RefCell::new(String::new()),
            help: help.clone(),
            cache: RefCell::new(None),
            rerender: RefCell::new(None),
            mode: Cell::new(Mode::Rainbow),
            view: Cell::new(View::HOME),
            pointer: Cell::new((0.0, 0.0)),
            drag_from: Cell::new(View::HOME),
            dragged: Cell::new(false),
            selected: Cell::new(None),
            order: Cell::new(0),
            slot_of: RefCell::new(vec![]),
            groups: RefCell::new(vec![]),
            chap_at: RefCell::new(vec![]),
            pending: Cell::new(None),
            modes: modes.clone(),
            read: read.clone(),
            groups_box: groups_box.clone(),
            labels: RefCell::new(vec![]),
            module: RefCell::new(String::new()),
            ver_dd: ver_dd.clone(),
            ver_names: RefCell::new(vec![]),
            ver_filling: Cell::new(false),
            gen: Cell::new(0),
            me: Default::default(),
        });
        *x.me.borrow_mut() = Rc::downgrade(&x);
        let w = Rc::downgrade(&x);
        x.ver_dd.connect_selected_notify(move |dd| {
            let Some(x) = w.upgrade() else { return };
            if x.ver_filling.get() {
                return;
            }
            let Some(name) = x.ver_names.borrow().get(dd.selected() as usize).cloned() else { return };
            // the reading Bible changes too, so the reader and this view agree
            x.ctx.config.borrow_mut().bible = name;
            x.ctx.save_config();
            x.ctx.refresh_one("bible");
            x.open();
        });
        let w = Rc::downgrade(&x);
        modes.connect_selected_notify(move |d| {
            let Some(x) = w.upgrade() else { return };
            let shape = match d.selected() {
                3 => Some(crate::xref3d::Shape::Cube),
                4 => Some(crate::xref3d::Shape::Globe),
                5 => Some(crate::xref3d::Shape::Mobius),
                _ => None,
            };
            x.is_3d.set(shape.is_some());
            if let Some(sh) = shape {
                x.space.set_shape(sh);
                x.feed_space();
                x.pics.set_visible_child_name("space");
                x.describe();
                return;
            }
            x.pics.set_visible_child_name("flat");
            x.mode.set(match d.selected() {
                1 => Mode::Circle,
                2 => Mode::Matrix,
                _ => Mode::Rainbow,
            });
            x.view.set(View::HOME);
            x.describe();
            x.area.queue_draw();
        });
        let w = Rc::downgrade(&x);
        orders.connect_selected_notify(move |d| {
            let Some(x) = w.upgrade() else { return };
            x.set_order(d.selected());
        });
        for (b, f) in [(&zin, 2.0), (&zout, 0.5), (&zfit, 0.0)] {
            let w = Rc::downgrade(&x);
            b.connect_clicked(move |_| {
                let Some(x) = w.upgrade() else { return };
                if x.is_3d.get() {
                    if f == 0.0 {
                        x.space.reset();
                    } else {
                        x.space.zoom_by(f);
                    }
                    return;
                }
                if f == 0.0 {
                    x.reset();
                } else {
                    let (ww, hh) = (x.area.width() as f64, x.area.height() as f64);
                    x.zoom_at(f, ww / 2.0, hh / 2.0);
                }
            });
        }
        let w = Rc::downgrade(&x);
        x.area.set_draw_func(move |_, cr, wd, ht| {
            if let Some(x) = w.upgrade() {
                x.draw(cr, wd, ht);
            }
        });
        // scroll to zoom where the pointer is
        let motion = gtk::EventControllerMotion::new();
        let w = Rc::downgrade(&x);
        motion.connect_motion(move |_, px, py| {
            if let Some(x) = w.upgrade() {
                x.pointer.set((px, py));
            }
        });
        x.area.add_controller(motion);
        let scroll = gtk::EventControllerScroll::new(
            gtk::EventControllerScrollFlags::VERTICAL,
        );
        let w = Rc::downgrade(&x);
        scroll.connect_scroll(move |_, _, dy| {
            let Some(x) = w.upgrade() else {
                return glib::Propagation::Proceed;
            };
            let (px, py) = x.pointer.get();
            x.zoom_at(1.4f64.powf(-dy.clamp(-6.0, 6.0)), px, py);
            glib::Propagation::Stop
        });
        x.area.add_controller(scroll);
        // drag to pan
        let drag = gtk::GestureDrag::new();
        let w = Rc::downgrade(&x);
        drag.connect_drag_begin(move |_, _, _| {
            if let Some(x) = w.upgrade() {
                x.drag_from.set(x.view.get());
                x.dragged.set(false);
            }
        });
        let w = Rc::downgrade(&x);
        drag.connect_drag_update(move |_, dx, dy| {
            let Some(x) = w.upgrade() else { return };
            if dx.abs() + dy.abs() > 8.0 {
                x.dragged.set(true);
            }
            if x.dragged.get() {
                let v = x.drag_from.get();
                x.set_view(View {
                    s: v.s,
                    ox: v.ox + dx,
                    oy: v.oy + dy,
                });
            }
        });
        x.area.add_controller(drag);
        // click a chapter (or a verse, zoomed in) to pick out its links
        let click = gtk::GestureClick::new();
        let w = Rc::downgrade(&x);
        click.connect_released(move |_, n, px, py| {
            let Some(x) = w.upgrade() else { return };
            if x.dragged.get() {
                return;
            }
            let Some((i, v)) = x.hit(px, py) else { return };
            if n == 2 {
                if let Some((b, c, _)) = x.chs.borrow().get(i as usize) {
                    x.ctx.open_ref(&Ref::new(b, *c, v.unwrap_or(0)));
                }
                return;
            }
            let pick = Some((i, v));
            x.selected
                .set(if x.selected.get() == pick { None } else { pick });
            x.describe();
            x.area.queue_draw();
        });
        x.area.add_controller(click);
        let w = Rc::downgrade(&x);
        x.read.connect_clicked(move |_| {
            let Some(x) = w.upgrade() else { return };
            if let Some((i, v)) = x.selected.get() {
                if let Some((b, c, _)) = x.chs.borrow().get(i as usize) {
                    x.ctx.open_ref(&Ref::new(b, *c, v.unwrap_or(0)));
                }
            }
        });
        // Esc clears the selection and zoom
        let keys = gtk::EventControllerKey::new();
        let w = Rc::downgrade(&x);
        keys.connect_key_pressed(move |_, key, _, _| {
            if key == gtk::gdk::Key::Escape {
                if let Some(x) = w.upgrade() {
                    x.reset();
                    return glib::Propagation::Stop;
                }
            }
            glib::Propagation::Proceed
        });
        x.root.add_controller(keys);
        x.area.set_has_tooltip(true);
        let w = Rc::downgrade(&x);
        x.area.connect_query_tooltip(move |_, px, py, _, tip| {
            let Some(x) = w.upgrade() else { return false };
            if let Some(t) = x.label_tip(px as f64, py as f64) {
                tip.set_text(Some(&t));
                return true;
            }
            let Some((i, v)) = x.hit(px as f64, py as f64) else {
                return false;
            };
            let chs = x.chs.borrow();
            let Some((b, c, _)) = chs.get(i as usize) else {
                return false;
            };
            let n = x.refs_of(i, v).len();
            let place = match v {
                Some(v) => format!("{} {c}:{v}", Ref::new(b, 1, 0).book_name()),
                None => format!("{} {c}", Ref::new(b, 1, 0).book_name()),
            };
            tip.set_text(Some(&format!(
                "{place} · {n} cross-references\nClick to show them, double-click to read"
            )));
            true
        });
        x
    }

    /// The version menu: every readable Bible, the reading one selected.
    fn fill_versions(&self, current: &str) {
        let lib = self.ctx.lib();
        let mut bibles: Vec<_> = lib
            .of_kind(bible_sword::Kind::Bible)
            .into_iter()
            .filter(|m| m.is_readable())
            .collect();
        bibles.sort_by_key(|m| m.abbreviation().to_lowercase());
        let labels: Vec<String> = bibles.iter().map(|m| m.abbreviation()).collect();
        let lab: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
        self.ver_filling.set(true);
        self.ver_dd.set_model(Some(&gtk::StringList::new(&lab)));
        if let Some(i) = bibles.iter().position(|m| m.name == current) {
            self.ver_dd.set_selected(i as u32);
        }
        *self.ver_names.borrow_mut() = bibles.iter().map(|m| m.name.clone()).collect();
        self.ver_filling.set(false);
    }

    /// Start (or reuse) the drawing for the reading Bible.
    pub fn open(&self) {
        let current = self.ctx.bible().map(|m| m.name.clone()).unwrap_or_default();
        self.fill_versions(&current);
        let Some(m) = self.ctx.bible() else {
            self.status.set_title("No Bible installed");
            self.status
                .set_description(Some("Download a Bible from Downloads first."));
            self.progress.set_visible(false);
            self.stack.set_visible_child_name("status");
            return;
        };
        self.title.set_subtitle(&m.abbreviation());
        if *self.module.borrow() == m.name && !self.vlinks.borrow().is_empty() {
            self.stack.set_visible_child_name("view");
            return;
        }
        *self.module.borrow_mut() = m.name.clone();
        *self.chs.borrow_mut() = chapters(&m);
        self.selected.set(None);
        self.view.set(View::HOME);
        *self.cache.borrow_mut() = None;
        // cached from an earlier run?
        if let Ok(text) = std::fs::read_to_string(cache_path(&m)) {
            let mut lines = text.lines();
            let source = lines.next().unwrap_or("").to_string();
            let links: Vec<VLink> = lines
                .filter_map(|l| {
                    let mut p = l.split('\t').map(|x| x.parse::<u16>().ok());
                    Some([p.next()??, p.next()??, p.next()??, p.next()??])
                })
                .collect();
            self.show(links, &source);
            return;
        }
        self.status.set_title("Drawing every cross-reference");
        self.status.set_description(Some(&format!(
            "Placing them on {} once. This takes a moment and is remembered afterwards.",
            m.description()
        )));
        self.progress.set_visible(true);
        self.progress.set_fraction(0.0);
        self.stack.set_visible_child_name("status");
        let gen = self.gen.get() + 1;
        self.gen.set(gen);
        let (tx, rx) = std::sync::mpsc::channel::<f64>();
        let bar = self.progress.clone();
        let tick = glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            while let Ok(f) = rx.try_recv() {
                bar.set_fraction(f);
                bar.set_text(Some(&format!("{:.0}%", f * 100.0)));
            }
            glib::ControlFlow::Continue
        });
        let w = self.me.borrow().clone();
        let mm: Arc<Module> = m.clone();
        glib::spawn_future_local(async move {
            let res = gio::spawn_blocking(move || {
                let (links, source) = compute(&mm, &|f| {
                    let _ = tx.send(f);
                });
                let path = cache_path(&mm);
                let mut text = format!("{source}\n");
                for l in &links {
                    text.push_str(&format!("{}\t{}\t{}\t{}\n", l[0], l[1], l[2], l[3]));
                }
                if let Some(d) = path.parent() {
                    let _ = std::fs::create_dir_all(d);
                }
                let _ = std::fs::write(&path, text);
                prune_cache(&path, &mm.name);
                (links, source)
            })
            .await;
            tick.remove();
            let Some(x) = w.upgrade() else { return };
            if x.gen.get() != gen {
                return;
            }
            if let Ok((links, source)) = res {
                x.show(links, &source);
            }
        });
    }

    fn show(&self, links: Vec<VLink>, source: &str) {
        if links.is_empty() {
            self.status.set_title("No cross-references for this Bible");
            self.status.set_description(Some(
                "This Bible’s book and chapter layout doesn’t match the built-in cross-references.",
            ));
            self.progress.set_visible(false);
            self.stack.set_visible_child_name("status");
            return;
        }
        *self.links.borrow_mut() = Arc::new(by_chapter(&links));
        *self.vlinks.borrow_mut() = Arc::new(links);
        *self.source.borrow_mut() = source.to_string();
        self.set_order(self.order.get());
        self.feed_space();
        self.stack.set_visible_child_name("view");
        self.area.queue_draw();
        // for screenshots and tests: BIBLE_APP_XREF="rainbow|circle|matrix zoom book.chapter.verse"
        if let Ok(spec) = std::env::var("BIBLE_APP_XREF") {
            let p: Vec<&str> = spec.split_whitespace().collect();
            self.mode.set(match p.first().copied() {
                Some("circle") => Mode::Circle,
                Some("matrix") => Mode::Matrix,
                _ => Mode::Rainbow,
            });
            if let Some(o) = p.get(3).and_then(|o| o.parse::<u32>().ok()) {
                self.set_order(o);
            }
            let zoom: f64 = p.get(1).and_then(|z| z.parse().ok()).unwrap_or(1.0);
            let pick = p.get(2).and_then(|r| bible_core::refs::parse_osis(r)).and_then(|r| {
                let i = self.chs.borrow().iter().position(|(b, c, _)| *b == r.book && *c == r.chapter)?;
                Some((i as u16, (r.verse > 0).then_some(r.verse)))
            });
            self.selected.set(pick);
            self.modes.set_selected(match p.first().copied() {
                Some("cube") => 3,
                Some("globe") => 4,
                Some("mobius") => 5,
                _ => self.mode.get() as u32,
            });
            let u = pick.map(|(i, v)| self.u(i, v)).unwrap_or(0.5);
            self.pending.set(Some((u, zoom)));
            self.describe();
            self.area.queue_draw();
        }
    }

    // ---- geometry -------------------------------------------------------

    fn n(&self) -> f64 {
        self.chs.borrow().len().max(1) as f64
    }

    /// Lay the books out in one of the ORDERS.
    fn set_order(&self, kind: u32) {
        self.order.set(kind);
        let chs = self.chs.borrow();
        let mut books: Vec<String> = vec![];
        for (b, _, _) in chs.iter() {
            if books.last() != Some(b) {
                books.push(b.clone());
            }
        }
        let canon: HashMap<String, usize> =
            books.iter().enumerate().map(|(i, b)| (b.clone(), i)).collect();
        let rank_from = |list: &[&str]| -> HashMap<String, usize> {
            list.iter().enumerate().map(|(i, b)| (b.to_string(), i)).collect()
        };
        let mut key: HashMap<String, i64> = HashMap::new();
        let mut group: HashMap<String, &'static str> = HashMap::new();
        let verses = |b: &str| -> i64 { chs.iter().filter(|c| c.0 == b).map(|c| c.2 as i64).sum() };
        let refs_per_book = || {
            let mut count: HashMap<String, i64> = HashMap::new();
            for l in self.vlinks.borrow().iter() {
                for c in [l[0], l[2]] {
                    if let Some((b, _, _)) = chs.get(c as usize) {
                        *count.entry(b.clone()).or_default() += 1;
                    }
                }
            }
            count
        };
        if let Some(bk) = buckets(kind) {
            let mut rank = 0i64;
            for (name, list) in bk {
                for b in *list {
                    key.insert(b.to_string(), rank);
                    group.insert(b.to_string(), name);
                    rank += 1;
                }
            }
            for b in &books {
                key.entry(b.clone()).or_insert(10_000 + canon[b] as i64);
            }
        } else {
            match kind {
                1 => {
                    let r = rank_from(&bible_core::plans::chronological_books());
                    for b in &books {
                        key.insert(b.clone(), r.get(b).map(|x| *x as i64).unwrap_or(999));
                    }
                }
                8 => {
                    let count = refs_per_book();
                    for b in &books {
                        key.insert(b.clone(), -count.get(b).copied().unwrap_or(0));
                    }
                }
                9 => {
                    let count = refs_per_book();
                    for b in &books {
                        let per = count.get(b).copied().unwrap_or(0) * 1000 / verses(b).max(1);
                        key.insert(b.clone(), -per);
                    }
                }
                10 => {
                    for b in &books {
                        key.insert(b.clone(), -verses(b));
                    }
                }
                _ => {
                    for b in &books {
                        key.insert(b.clone(), canon[b] as i64);
                    }
                }
            }
        }
        // chapter-level forms override the book order
        let chapter_group: Vec<&'static str> = chs
            .iter()
            .map(|(b, c, _)| {
                if kind == 11 {
                    form(b, *c)
                } else {
                    group.get(b).copied().unwrap_or("Other")
                }
            })
            .collect();
        if kind == 11 {
            group.insert(String::new(), "");
        }
        let mut chap_at: Vec<u16> = (0..chs.len() as u16).collect();
        chap_at.sort_by_key(|&i| {
            let (b, _, _) = &chs[i as usize];
            let primary = if kind == 11 {
                FORMS.iter().position(|f| *f == chapter_group[i as usize]).unwrap_or(99) as i64
            } else {
                0
            };
            (primary, key[b], canon[b], i)
        });
        // where each bucket starts, for labels and dividers
        let n = chs.len().max(1) as f64;
        let mut groups: Vec<(f64, String)> = vec![];
        for (slot, ch) in chap_at.iter().enumerate() {
            let g = chapter_group[*ch as usize];
            if !group.is_empty() && groups.last().map(|x| x.1.as_str()) != Some(g) {
                groups.push((slot as f64 / n, g.to_string()));
            }
        }
        // the side panel explains this order's groups
        while let Some(c) = self.groups_box.first_child() {
            self.groups_box.remove(&c);
        }
        if !groups.is_empty() {
            self.groups_box
                .append(&help_heading(&format!("Groups in “{}”", ORDERS[kind as usize])));
            let mut seen = std::collections::HashSet::new();
            for (_, g) in &groups {
                if seen.insert(g.clone()) {
                    self.groups_box
                        .append(&help_line(g, bucket_help(g).unwrap_or("")));
                }
            }
        }
        *self.groups.borrow_mut() = groups;
        let mut slot_of = vec![0u16; chs.len()];
        for (slot, ch) in chap_at.iter().enumerate() {
            slot_of[*ch as usize] = slot as u16;
        }
        drop(chs);
        *self.chap_at.borrow_mut() = chap_at;
        *self.geo.borrow_mut() = Arc::new(Geo {
            verses: self.chs.borrow().iter().map(|c| c.2).collect(),
            slot_of: slot_of.clone(),
        });
        *self.slot_of.borrow_mut() = slot_of;
        *self.cache.borrow_mut() = None;
        self.view.set(View::HOME);
        // the 3D views follow the chosen order too
        self.feed_space();
        self.describe();
        self.area.queue_draw();
    }

    /// Position along the drawing, 0..1, of a chapter (and verse).
    fn u(&self, ch: u16, verse: Option<u16>) -> f64 {
        self.geo.borrow().u(ch, verse)
    }

    fn base(h: f64) -> f64 {
        h - BAR_MAX - 48.0
    }

    fn circle_geom(w: f64, h: f64) -> (f64, f64, f64) {
        (w / 2.0, h / 2.0, (w.min(h) / 2.0 - 24.0).max(10.0))
    }

    fn matrix_geom(w: f64, h: f64) -> (f64, f64, f64) {
        let l = (w - 2.0 * MARGIN).min(h - 32.0).max(10.0);
        ((w - l) / 2.0, 12.0, l)
    }

    /// World point (unzoomed) of a position along the Bible.
    fn world(mode: Mode, u: f64, w: f64, h: f64) -> (f64, f64) {
        match mode {
            Mode::Rainbow => (MARGIN + u * (w - 2.0 * MARGIN), Self::base(h)),
            Mode::Circle => {
                let (cx, cy, r) = Self::circle_geom(w, h);
                let a = -std::f64::consts::FRAC_PI_2 + u * std::f64::consts::TAU;
                (cx + r * a.cos(), cy + r * a.sin())
            }
            Mode::Matrix => {
                let (x0, _, l) = Self::matrix_geom(w, h);
                (x0 + u * l, 0.0)
            }
        }
    }

    fn clamp_view(&self, mut v: View) -> View {
        let (w, h) = (self.area.width() as f64, self.area.height() as f64);
        v.s = v.s.clamp(1.0, MAX_ZOOM);
        v.ox = v.ox.clamp(w * (1.0 - v.s), 0.0);
        if self.mode.get() == Mode::Rainbow {
            // zoom about the baseline: arcs grow upwards
            v.oy = Self::base(h) * (1.0 - v.s);
        } else {
            v.oy = v.oy.clamp(h * (1.0 - v.s), 0.0);
        }
        v
    }

    fn set_view(&self, v: View) {
        let v = self.clamp_view(v);
        if v == self.view.get() {
            return;
        }
        self.view.set(v);
        self.area.queue_draw();
        // redraw sharply once the zooming settles
        if let Some(id) = self.rerender.borrow_mut().take() {
            id.remove();
        }
        let w = self.me.borrow().clone();
        let id = glib::timeout_add_local_once(std::time::Duration::from_millis(140), move || {
            if let Some(x) = w.upgrade() {
                x.rerender.borrow_mut().take();
                x.render_cache();
                x.area.queue_draw();
            }
        });
        *self.rerender.borrow_mut() = Some(id);
    }

    /// Back to the whole Bible with nothing selected.
    fn reset(&self) {
        if self.is_3d.get() {
            self.space.reset();
            return;
        }
        self.selected.set(None);
        self.describe();
        self.set_view(View::HOME);
        self.area.queue_draw();
    }

    fn zoom_at(&self, f: f64, px: f64, py: f64) {
        let v = self.view.get();
        let s = (v.s * f).clamp(1.0, MAX_ZOOM);
        let k = s / v.s;
        self.set_view(View {
            s,
            ox: px - (px - v.ox) * k,
            oy: py - (py - v.oy) * k,
        });
    }

    /// Chapter (and verse, when it is wide enough to aim at) under a point.
    fn hit(&self, px: f64, py: f64) -> Option<(u16, Option<u16>)> {
        let n = self.chs.borrow().len();
        if n == 0 {
            return None;
        }
        let (w, h) = (self.area.width() as f64, self.area.height() as f64);
        let v = self.view.get();
        let (wx, wy) = ((px - v.ox) / v.s, (py - v.oy) / v.s);
        let u = match self.mode.get() {
            Mode::Rainbow => (wx - MARGIN) / (w - 2.0 * MARGIN),
            Mode::Circle => {
                let (cx, cy, _) = Self::circle_geom(w, h);
                let a = (wy - cy).atan2(wx - cx) + std::f64::consts::FRAC_PI_2;
                a.rem_euclid(std::f64::consts::TAU) / std::f64::consts::TAU
            }
            Mode::Matrix => {
                let (x0, _, l) = Self::matrix_geom(w, h);
                (wx - x0) / l
            }
        };
        if !(0.0..1.0).contains(&u) {
            return None;
        }
        let pos = u * n as f64;
        let slot = pos.floor() as usize;
        let i = self.chap_at.borrow().get(slot).copied().unwrap_or(slot as u16) as usize;
        let vs = self.chs.borrow()[i].2;
        // pixels per verse on screen
        let span = match self.mode.get() {
            Mode::Rainbow => (w - 2.0 * MARGIN) / n as f64 * v.s,
            Mode::Matrix => Self::matrix_geom(w, h).2 / n as f64 * v.s,
            Mode::Circle => Self::circle_geom(w, h).2 * std::f64::consts::TAU / n as f64 * v.s,
        } / vs.max(1) as f64;
        let verse = (span >= 3.0 && vs > 0)
            .then(|| ((pos.fract() * vs as f64).floor() as u16 + 1).min(vs));
        Some((i as u16, verse))
    }

    /// Verse links touching a chapter, or one verse of it.
    fn refs_of(&self, ch: u16, verse: Option<u16>) -> Vec<VLink> {
        self.vlinks
            .borrow()
            .iter()
            .filter(|l| {
                (l[0] == ch && verse.is_none_or(|v| l[1] == v))
                    || (l[2] == ch && verse.is_none_or(|v| l[3] == v))
            })
            .copied()
            .collect()
    }

    /// Give the cube and globe this Bible's chapters and links.
    fn feed_space(&self) {
        if self.is_3d.get() {
            let n = self.chs.borrow().len();
            let order: Vec<f64> = (0..n as u16).map(|c| self.u(c, None)).collect();
            self.space
                .set_data(self.chs.borrow().clone(), self.links.borrow().as_ref().clone(), order);
        }
    }

    fn describe(&self) {
        if self.is_3d.get() {
            self.read.set_visible(false);
            self.info.set_text(
                "Drag to turn, scroll to zoom · ⤢ resets · colours show how far apart the passages are",
            );
            return;
        }
        let total = self.vlinks.borrow().len();
        let source = self.source.borrow().clone();
        self.read.set_visible(self.selected.get().is_some());
        if let Some((i, v)) = self.selected.get() {
            if let Some((b, c, _)) = self.chs.borrow().get(i as usize) {
                let r = Ref::new(b, *c, v.unwrap_or(0));
                self.read.set_label(&format!("Read {}", r));
            }
        }
        let Some((i, verse)) = self.selected.get() else {
            // "from WEB itself", or plainly that the built-in set stands in
            let short = if let Some(own) = source.strip_prefix("Cross-references from ") {
                format!("from {own}")
            } else if source.contains("OpenBible.info") {
                let bible = source.split(' ').next().unwrap_or("");
                format!("from OpenBible.info, built in ({bible} has none of its own)")
            } else {
                format!("from {}", source.split(" (").next().unwrap_or(&source))
            };
            let n = total.to_string();
            let mut grouped = String::new();
            for (k, ch) in n.chars().enumerate() {
                if k > 0 && (n.len() - k).is_multiple_of(3) {
                    grouped.push(',');
                }
                grouped.push(ch);
            }
            self.info.set_text(&format!(
                "{grouped} cross-references {short} · click a chapter to see its links"
            ));
            self.info.set_tooltip_text(Some(&source));
            return;
        };
        let chs = self.chs.borrow();
        let Some((b, c, _)) = chs.get(i as usize) else {
            return;
        };
        let refs = self.refs_of(i, verse);
        let name = |ch: u16, v: Option<u16>| {
            chs.get(ch as usize)
                .map(|(b, c, _)| match v {
                    Some(v) => format!("{} {c}:{v}", Ref::new(b, 1, 0).book_name()),
                    None => format!("{} {c}", Ref::new(b, 1, 0).book_name()),
                })
                .unwrap_or_default()
        };
        let here = name(i, verse);
        if let Some(v) = verse {
            // every passage that meets at this verse
            let mut seen = std::collections::BTreeSet::new();
            for l in &refs {
                let other = if l[0] == i && l[1] == v {
                    (l[2], l[3])
                } else {
                    (l[0], l[1])
                };
                seen.insert(other);
            }
            let list: Vec<String> = seen
                .iter()
                .take(14)
                .map(|(c, v)| name(*c, Some(*v)))
                .collect();
            self.info.set_text(&format!(
                "{here} meets {} other verses: {}{}",
                seen.len(),
                list.join(", "),
                if seen.len() > 14 { ", …" } else { "" }
            ));
            return;
        }
        let mut count: HashMap<u16, u32> = HashMap::new();
        for l in &refs {
            let o = if l[0] == i { l[2] } else { l[0] };
            if o != i {
                *count.entry(o).or_default() += 1;
            }
        }
        let mut others: Vec<(u16, u32)> = count.into_iter().collect();
        others.sort_by_key(|o| std::cmp::Reverse(o.1));
        let top: Vec<String> = others.iter().take(6).map(|(o, _)| name(*o, None)).collect();
        let _ = (b, c);
        self.info.set_text(&format!(
            "{here} has {} cross-references to {} chapters, most of all {}",
            refs.len(),
            others.len(),
            top.join(", ")
        ));
    }

    fn hue(t: f64) -> (f64, f64, f64) {
        // red for near neighbours through to violet for the far ends of the Bible
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

    // ---- drawing --------------------------------------------------------

    /// One rainbow arc between two positions, in screen coordinates.
    /// Returns false when it lies wholly off screen.
    fn arc(cr: &cairo::Context, ua: f64, ub: f64, w: f64, h: f64, v: View) -> bool {
        let base = Self::base(h);
        let (xa, _) = Self::world(Mode::Rainbow, ua.min(ub), w, h);
        let (xb, _) = Self::world(Mode::Rainbow, ua.max(ub), w, h);
        let (x0, x1) = (xa * v.s + v.ox, xb * v.s + v.ox);
        if x1 < -1.0 || x0 > w + 1.0 {
            return false;
        }
        let r = ((x1 - x0) / 2.0).max(0.4);
        let cx = x0 + r;
        // flatten so an arc across the whole Bible fits the height
        let sy = ((base - 8.0) / (w / 2.0)).clamp(0.2, 1.0);
        if x0 < 0.0 && x1 > w {
            let edge = (cx - 0.0).abs().max((w - cx).abs());
            let low = sy * (r * r - edge * edge).max(0.0).sqrt();
            if low > base + 2.0 {
                return false;
            }
        }
        cr.save().ok();
        cr.translate(cx, base);
        cr.scale(1.0, sy);
        cr.arc(0.0, 0.0, r, std::f64::consts::PI, 0.0);
        cr.restore().ok();
        true
    }

    fn chord(cr: &cairo::Context, ua: f64, ub: f64, w: f64, h: f64) {
        let (cx, cy, _) = Self::circle_geom(w, h);
        let (ax, ay) = Self::world(Mode::Circle, ua, w, h);
        let (bx, by) = Self::world(Mode::Circle, ub, w, h);
        // bow towards the centre, more for distant passages
        let pull = 0.25 + 0.6 * (1.0 - ((ua - ub).abs() * 2.0 - 1.0).abs());
        let (c1x, c1y) = (ax + (cx - ax) * pull, ay + (cy - ay) * pull);
        let (c2x, c2y) = (bx + (cx - bx) * pull, by + (cy - by) * pull);
        cr.move_to(ax, ay);
        cr.curve_to(c1x, c1y, c2x, c2y, bx, by);
    }

    /// Draw every cross-reference into pixels. Runs on a worker thread.
    #[allow(clippy::too_many_arguments)]
    fn paint(
        geo: &Geo,
        links: &[(u16, u16, u16)],
        vlinks: &[VLink],
        mode: Mode,
        v: View,
        wd: i32,
        ht: i32,
        dark: bool,
    ) -> Option<(i32, Vec<u8>)> {
        let mut surf = cairo::ImageSurface::create(cairo::Format::ARgb32, wd, ht).ok()?;
        let c2 = cairo::Context::new(&surf).ok()?;
        let (w, h) = (wd as f64, ht as f64);
        match mode {
            Mode::Rainbow => {
                c2.set_line_width(0.6);
                if v.s < 4.0 {
                    for (a, b, cnt) in links.iter() {
                        let (ua, ub) = (geo.u(*a, None), geo.u(*b, None));
                        if Self::arc(&c2, ua, ub, w, h, v) {
                            let (r, g, bl) = Self::hue((ua - ub).abs());
                            c2.set_source_rgba(r, g, bl, (0.10 + 0.05 * (*cnt as f64).ln_1p()).min(0.5));
                            let _ = c2.stroke();
                        }
                    }
                } else {
                    // zoomed in: every verse-to-verse link, so the fine
                    // structure shows
                    c2.set_line_width(0.5);
                    let alpha = (0.05 + v.s / 400.0).min(0.35);
                    for l in vlinks.iter() {
                        let (ua, ub) = (geo.u(l[0], Some(l[1])), geo.u(l[2], Some(l[3])));
                        if (ua - ub).abs() < 1e-9 {
                            continue;
                        }
                        if Self::arc(&c2, ua, ub, w, h, v) {
                            let (r, g, bl) = Self::hue((ua - ub).abs());
                            c2.set_source_rgba(r, g, bl, alpha);
                            let _ = c2.stroke();
                        }
                    }
                }
            }
            Mode::Circle => {
                c2.translate(v.ox, v.oy);
                c2.scale(v.s, v.s);
                c2.set_line_width(0.5 / v.s);
                for (a, b, cnt) in links.iter() {
                    let (ua, ub) = (geo.u(*a, None), geo.u(*b, None));
                    Self::chord(&c2, ua, ub, w, h);
                    let (r, g, bl) = Self::hue((ua - ub).abs());
                    c2.set_source_rgba(r, g, bl, (0.06 + 0.04 * (*cnt as f64).ln_1p()).min(0.4));
                    let _ = c2.stroke();
                }
            }
            Mode::Matrix => {
                let (x0, y0, l) = Self::matrix_geom(w, h);
                let fg = if dark { 1.0 } else { 0.0 };
                c2.set_source_rgba(fg, fg, fg, 0.04);
                c2.rectangle(x0 * v.s + v.ox, y0 * v.s + v.oy, l * v.s, l * v.s);
                let _ = c2.fill();
                let dot = (v.s * l / 4000.0).clamp(1.0, 4.0);
                for k in vlinks.iter() {
                    let (ua, ub) = (geo.u(k[0], Some(k[1])), geo.u(k[2], Some(k[3])));
                    let px = (x0 + ua * l) * v.s + v.ox;
                    let py = (y0 + ub * l) * v.s + v.oy;
                    if px < -dot || py < -dot || px > w || py > h {
                        continue;
                    }
                    let (r, g, bl) = Self::hue((ua - ub).abs());
                    c2.set_source_rgba(r, g, bl, 0.55);
                    c2.rectangle(px, py, dot, dot);
                    let _ = c2.fill();
                }
            }
        }
        drop(c2);
        surf.flush();
        let stride = surf.stride();
        let data = surf.data().ok()?.to_vec();
        Some((stride, data))
    }

    /// Whether the cached drawing matches what should be on screen.
    fn cache_current(&self) -> bool {
        matches!(self.cache.borrow().as_ref(), Some(c)
            if c.w == self.area.width() && c.h == self.area.height()
                && c.mode == self.mode.get() && c.order == self.order.get()
                && c.view == self.view.get())
    }

    /// Make a fresh drawing in the background, keeping the app responsive;
    /// the old one is stretched and shown meanwhile, with a spinner.
    fn render_cache(&self) {
        let (wd, ht) = (self.area.width(), self.area.height());
        if wd < 2 || ht < 2 || self.vlinks.borrow().is_empty() {
            return;
        }
        if self.inflight.get() {
            self.again.set(true);
            return;
        }
        self.inflight.set(true);
        self.busy.set_visible(true);
        let (mode, order, v) = (self.mode.get(), self.order.get(), self.view.get());
        let dark = adw::StyleManager::default().is_dark();
        let geo = self.geo.borrow().clone();
        let links = self.links.borrow().clone();
        let vlinks = self.vlinks.borrow().clone();
        let w = self.me.borrow().clone();
        glib::spawn_future_local(async move {
            let res = gio::spawn_blocking(move || {
                Self::paint(&geo, &links, &vlinks, mode, v, wd, ht, dark)
            })
            .await;
            let Some(x) = w.upgrade() else { return };
            x.inflight.set(false);
            if let Ok(Some((stride, data))) = res {
                if let Ok(surf) = cairo::ImageSurface::create_for_data(
                    data,
                    cairo::Format::ARgb32,
                    wd,
                    ht,
                    stride,
                ) {
                    *x.cache.borrow_mut() = Some(Cached {
                        w: wd,
                        h: ht,
                        mode,
                        order,
                        view: v,
                        surf,
                    });
                }
            }
            if x.again.replace(false) || !x.cache_current() {
                x.render_cache();
            } else {
                x.busy.set_visible(false);
            }
            x.area.queue_draw();
        });
    }



    fn draw(&self, cr: &cairo::Context, wd: i32, ht: i32) {
        let (w, h) = (wd as f64, ht as f64);
        let dark = adw::StyleManager::default().is_dark();
        let fg = if dark { 1.0 } else { 0.0 };
        let mode = self.mode.get();
        self.labels.borrow_mut().clear();
        if let Some((u, z)) = self.pending.take() {
            let (px, py) = Self::world(mode, u, w, h);
            let py = if mode == Mode::Matrix {
                let (_, y0, l) = Self::matrix_geom(w, h);
                y0 + u * l
            } else {
                py
            };
            let v = self.clamp_view(View { s: z, ox: w / 2.0 - px * z, oy: h / 2.0 - py * z });
            self.view.set(v);
        } else {
            // keep the view inside the drawing when the window is resized
            self.view.set(self.clamp_view(self.view.get()));
        }
        let v = self.view.get();
        let fresh = !matches!(self.cache.borrow().as_ref(),
            Some(c) if c.w == wd && c.h == ht && c.mode == mode && c.order == self.order.get());
        if fresh && !self.inflight.get() {
            // size, view type or order changed: draw anew in the background
            let w = self.me.borrow().clone();
            glib::idle_add_local_once(move || {
                if let Some(x) = w.upgrade() {
                    x.render_cache();
                }
            });
        }
        let same_kind = matches!(self.cache.borrow().as_ref(),
            Some(c) if c.mode == mode && c.order == self.order.get() && c.w == wd && c.h == ht);
        if let Some(c) = self.cache.borrow().as_ref().filter(|_| same_kind) {
            // stretch the last drawing while a zoom is in flight
            let k = v.s / c.view.s;
            cr.save().ok();
            cr.translate(v.ox - c.view.ox * k, v.oy - c.view.oy * k);
            cr.scale(k, k);
            let _ = cr.set_source_surface(&c.surf, 0.0, 0.0);
            let _ = cr.paint_with_alpha(if self.selected.get().is_some() { 0.22 } else { 1.0 });
            cr.restore().ok();
        }
        // the chosen chapter's or verse's links on top, meeting at the verse
        if let Some((i, verse)) = self.selected.get() {
            let refs = self.refs_of(i, verse);
            cr.set_source_rgba(0.21, 0.52, 0.89, if refs.len() > 400 { 0.45 } else { 0.85 });
            cr.set_line_width(if verse.is_some() { 1.6 } else { 1.0 });
            for l in &refs {
                let (ua, ub) = (self.u(l[0], Some(l[1])), self.u(l[2], Some(l[3])));
                match mode {
                    Mode::Rainbow => {
                        if Self::arc(cr, ua, ub, w, h, v) {
                            let _ = cr.stroke();
                        }
                    }
                    Mode::Circle => {
                        cr.save().ok();
                        cr.translate(v.ox, v.oy);
                        cr.scale(v.s, v.s);
                        Self::chord(cr, ua, ub, w, h);
                        cr.restore().ok();
                        let _ = cr.stroke();
                    }
                    Mode::Matrix => {
                        let (x0, y0, l2) = Self::matrix_geom(w, h);
                        let px = (x0 + ua * l2) * v.s + v.ox;
                        let py = (y0 + ub * l2) * v.s + v.oy;
                        cr.rectangle(px - 1.5, py - 1.5, 3.0, 3.0);
                        let _ = cr.fill();
                    }
                }
            }
            // a dot where they all meet
            if let (Some(_), Mode::Rainbow) = (verse, mode) {
                let (x, y) = Self::world(Mode::Rainbow, self.u(i, verse), w, h);
                cr.arc(x * v.s + v.ox, y, 4.0, 0.0, std::f64::consts::TAU);
                let _ = cr.fill();
            }
        }
        self.draw_legend(cr, w, fg);
        match mode {
            Mode::Rainbow => self.draw_bars(cr, w, h, v, fg),
            Mode::Circle => self.draw_ring(cr, w, h, v, fg),
            Mode::Matrix => self.draw_axes(cr, w, h, v, fg),
        }
    }

    /// Labels for this zoom: buckets when zoomed out (if the order has
    /// them), else books.
    fn label_starts(&self, s: f64) -> Vec<(f64, String)> {
        let g = self.groups.borrow();
        if !g.is_empty() && s < 3.0 {
            return g.clone();
        }
        self.book_starts()
    }

    /// Whether a label would cover one already drawn.
    fn overlaps(&self, x: f64, y: f64, w: f64, h: f64) -> bool {
        self.labels
            .borrow()
            .iter()
            .any(|(lx, ly, lw, lh, _)| x < lx + lw + 4.0 && lx - 4.0 < x + w && y < ly + lh && *ly < y + h)
    }

    /// Remember a drawn label, for hovering.
    fn label_at(&self, x: f64, y: f64, pl: &pango::Layout, text: &str) {
        let (tw, th) = pl.pixel_size();
        self.labels
            .borrow_mut()
            .push((x, y, tw as f64, th as f64, text.to_string()));
    }

    /// The explanation for a label under the pointer: a bucket's note, or
    /// the opening of a book's introduction.
    fn label_tip(&self, px: f64, py: f64) -> Option<String> {
        let labels = self.labels.borrow();
        let (_, _, _, _, name) = labels
            .iter()
            .find(|(x, y, w, h, _)| px >= *x - 2.0 && px <= x + w + 2.0 && py >= *y - 2.0 && py <= y + h + 2.0)?;
        if let Some(h) = bucket_help(name) {
            return Some(format!("{name}\n{h}"));
        }
        let chs = self.chs.borrow();
        let osis = chs
            .iter()
            .find(|(b, _, _)| Ref::new(b, 1, 0).book_name() == *name)
            .map(|c| c.0.clone())?;
        let intro = bible_core::intros::intro(&osis)?;
        let mut first = String::new();
        for sentence in intro.text.split_inclusive(". ") {
            first.push_str(sentence);
            if first.chars().count() > 160 {
                break;
            }
        }
        let mut out = format!("{name}\n{}", first.trim());
        if !intro.traditional.is_empty() {
            out.push_str(&format!("\n{}", intro.traditional));
        }
        Some(out)
    }

    /// Dividers between buckets.
    fn dividers(&self) -> Vec<f64> {
        self.groups.borrow().iter().skip(1).map(|g| g.0).collect()
    }

    /// What the colours mean: a gradient from near to far, top right.
    fn draw_legend(&self, cr: &cairo::Context, w: f64, fg: f64) {
        let pl = self.area.create_pango_layout(None);
        pl.set_font_description(Some(&pango::FontDescription::from_string("Sans 8")));
        let (bw, bh) = (250.0, 8.0);
        let x = w - MARGIN - bw;
        let y = 26.0;
        // background so it reads over the arcs
        let bg = if fg > 0.5 { 0.15 } else { 0.98 };
        cr.set_source_rgba(bg, bg, bg, 0.85);
        rounded(cr, x - 10.0, y - 20.0, bw + 20.0, 58.0 + if self.selected.get().is_some() { 16.0 } else { 0.0 }, 8.0);
        let _ = cr.fill();
        cr.set_source_rgba(fg, fg, fg, 0.8);
        pl.set_text("Colour = distance between the two passages");
        cr.move_to(x, y - 15.0);
        pangocairo::functions::show_layout(cr, &pl);
        let steps = 60;
        for k in 0..steps {
            let t = k as f64 / steps as f64;
            let (r, g, b) = Self::hue(t);
            cr.set_source_rgb(r, g, b);
            cr.rectangle(x + t * bw, y, bw / steps as f64 + 0.5, bh);
            let _ = cr.fill();
        }
        cr.set_source_rgba(fg, fg, fg, 0.65);
        for (t, text) in [(0.0, "Nearby"), (0.5, "Half the Bible"), (1.0, "Opposite ends")] {
            pl.set_text(text);
            let (tw, _) = pl.pixel_size();
            let tx = (x + t * bw - tw as f64 * t).max(x);
            cr.move_to(tx, y + bh + 3.0);
            pangocairo::functions::show_layout(cr, &pl);
        }
        if self.selected.get().is_some() {
            cr.set_source_rgb(0.21, 0.52, 0.89);
            cr.rectangle(x, y + bh + 22.0, 14.0, 4.0);
            let _ = cr.fill();
            cr.set_source_rgba(fg, fg, fg, 0.8);
            pl.set_text("Links of what you picked");
            cr.move_to(x + 20.0, y + bh + 17.0);
            pangocairo::functions::show_layout(cr, &pl);
        }
    }

    /// Book names that fit at this zoom, with where each starts (0..1).
    fn book_starts(&self) -> Vec<(f64, String)> {
        let chs = self.chs.borrow();
        let n = self.n();
        let mut out: Vec<(f64, String)> = vec![];
        let mut last = String::new();
        for (slot, ch) in self.chap_at.borrow().iter().enumerate() {
            let b = &chs[*ch as usize].0;
            if *b != last {
                last = b.clone();
                out.push((slot as f64 / n, Ref::new(b, 1, 0).book_name()));
            }
        }
        out
    }

    fn draw_bars(&self, cr: &cairo::Context, w: f64, h: f64, v: View, fg: f64) {
        // a bar per chapter, its height the number of verses; books alternate shade
        let base = Self::base(h);
        let chs = self.chs.borrow();
        let n = self.n();
        let bw = (w - 2.0 * MARGIN) / n * v.s;
        let max_v = chs.iter().map(|c| c.2).max().unwrap_or(1) as f64;
        let mut book_i = 0;
        let mut last = String::new();
        let sel = self.selected.get();
        let order = self.chap_at.borrow();
        for (slot, &ch) in order.iter().enumerate() {
            let i = ch as usize;
            let (b, _, vs) = &chs[i];
            if *b != last {
                book_i += 1;
                last = b.clone();
            }
            let x = (MARGIN + slot as f64 / n * (w - 2.0 * MARGIN)) * v.s + v.ox;
            if x + bw < 0.0 || x > w {
                continue;
            }
            let hgt = (*vs as f64 / max_v) * BAR_MAX;
            let shade = if book_i % 2 == 0 { 0.55 } else { 0.35 };
            cr.set_source_rgba(fg, fg, fg, shade);
            if sel.map(|s| s.0) == Some(i as u16) {
                cr.set_source_rgb(0.21, 0.52, 0.89);
            }
            cr.rectangle(x, base + 2.0, bw.max(0.6) - if bw > 6.0 { 1.0 } else { 0.0 }, hgt);
            let _ = cr.fill();
            if bw >= 28.0 {
                let pl = self.area.create_pango_layout(Some(&chs[i].1.to_string()));
                pl.set_font_description(Some(&pango::FontDescription::from_string("Sans 7")));
                cr.set_source_rgba(fg, fg, fg, 0.55);
                cr.move_to(x + 2.0, base + BAR_MAX + 4.0 - 14.0);
                pangocairo::functions::show_layout(cr, &pl);
            }
            // verse ticks once there is room
            let per = bw / (*vs).max(1) as f64;
            if per >= 3.0 {
                cr.set_source_rgba(fg, fg, fg, 0.5);
                for k in 1..*vs {
                    let tx = x + k as f64 * per;
                    cr.rectangle(tx, base + 2.0, 0.6, 5.0);
                }
                let _ = cr.fill();
                if let Some((_, Some(sv))) = sel.filter(|s| s.0 == i as u16) {
                    cr.set_source_rgb(0.21, 0.52, 0.89);
                    cr.rectangle(x + (sv - 1) as f64 * per, base + 2.0, per.max(1.0), hgt + 4.0);
                    let _ = cr.fill();
                }
            }
        }
        // labels: books when zoomed in enough, else the two testaments
        let pl = self.area.create_pango_layout(None);
        pl.set_font_description(Some(&pango::FontDescription::from_string("Sans 8")));
        cr.set_source_rgba(fg, fg, fg, 0.6);
        let mut next_free = [f64::MIN; 3];
        let starts = self.label_starts(v.s);
        // bucket dividers
        cr.set_source_rgba(fg, fg, fg, 0.35);
        for u in self.dividers() {
            let x = (MARGIN + u * (w - 2.0 * MARGIN)) * v.s + v.ox;
            if (0.0..=w).contains(&x) {
                cr.rectangle(x - 0.5, base + 2.0, 1.0, h - base - 6.0);
            }
        }
        let _ = cr.fill();
        cr.set_source_rgba(fg, fg, fg, 0.6);
        let show_all = v.s >= 2.5 || self.order.get() != 0;
        let sx = |u: f64| (MARGIN + u * (w - 2.0 * MARGIN)) * v.s + v.ox;
        // the book running off the left edge keeps its name pinned there
        let pinned = starts.iter().rposition(|(u, _)| sx(*u) <= MARGIN);
        for (k, (u, name)) in starts.iter().enumerate() {
            if !show_all && !matches!(name.as_str(), "Genesis" | "Matthew" | "Psalms" | "Isaiah" | "Acts" | "Romans" | "Revelation") {
                continue;
            }
            let x = sx(*u);
            if x < MARGIN && Some(k) != pinned {
                continue;
            }
            pl.set_text(name);
            let (tw, _) = pl.pixel_size();
            if x > w - MARGIN - 12.0 {
                continue;
            }
            let x = x.max(MARGIN).min(w - MARGIN - tw as f64);
            // two staggered rows so small groups keep their names
            let Some(row) = (0..3).find(|r| x >= next_free[*r]) else {
                continue;
            };
            if x > w {
                continue;
            }
            let y = h - 40.0 + row as f64 * 12.0;
            cr.move_to(x, y);
            pangocairo::functions::show_layout(cr, &pl);
            self.label_at(x, y, &pl, name);
            next_free[row] = x + tw as f64 + 8.0;
        }
        if v.s > 1.01 {
            pl.set_text(&format!("{:.0}×", v.s));
            let (tw, _) = pl.pixel_size();
            cr.move_to(w - MARGIN - tw as f64, 92.0);
            pangocairo::functions::show_layout(cr, &pl);
        }
    }

    fn draw_ring(&self, cr: &cairo::Context, w: f64, h: f64, v: View, fg: f64) {
        let (cx, cy, r) = Self::circle_geom(w, h);
        let pl = self.area.create_pango_layout(None);
        pl.set_font_description(Some(&pango::FontDescription::from_string("Sans 8")));
        let starts = self.label_starts(v.s);
        for (k, (u, name)) in starts.iter().enumerate() {
            let end = starts.get(k + 1).map(|s| s.0).unwrap_or(1.0);
            let a0 = -std::f64::consts::FRAC_PI_2 + u * std::f64::consts::TAU;
            let a1 = -std::f64::consts::FRAC_PI_2 + end * std::f64::consts::TAU;
            cr.set_source_rgba(fg, fg, fg, if k % 2 == 0 { 0.55 } else { 0.3 });
            cr.set_line_width(6.0);
            cr.new_path();
            cr.arc(cx * v.s + v.ox, cy * v.s + v.oy, r * v.s + 6.0, a0, a1);
            let _ = cr.stroke();
            // name the bigger books, or all of them zoomed in
            {
                let am = (a0 + a1) / 2.0;
                let (lx, ly) = (
                    cx * v.s + v.ox + (r * v.s + 14.0) * am.cos(),
                    cy * v.s + v.oy + (r * v.s + 14.0) * am.sin(),
                );
                pl.set_text(name);
                let (tw, th) = pl.pixel_size();
                cr.set_source_rgba(fg, fg, fg, 0.65);
                let off = if am.cos() < 0.0 { tw as f64 } else { 0.0 };
                if self.overlaps(lx - off, ly - th as f64 / 2.0, tw as f64, th as f64) {
                    continue;
                }
                cr.move_to(lx - off, ly - th as f64 / 2.0);
                pangocairo::functions::show_layout(cr, &pl);
                self.label_at(lx - off, ly - th as f64 / 2.0, &pl, name);
            }
        }
    }

    fn draw_axes(&self, cr: &cairo::Context, w: f64, h: f64, v: View, fg: f64) {
        let (x0, y0, l) = Self::matrix_geom(w, h);
        let pl = self.area.create_pango_layout(None);
        pl.set_font_description(Some(&pango::FontDescription::from_string("Sans 8")));
        cr.set_source_rgba(fg, fg, fg, 0.6);
        // bucket grid: the blocks where one group of books echoes another
        cr.set_source_rgba(fg, fg, fg, 0.25);
        cr.set_line_width(1.0);
        for u in self.dividers() {
            let x = (x0 + u * l) * v.s + v.ox;
            let y = (y0 + u * l) * v.s + v.oy;
            cr.move_to(x, y0 * v.s + v.oy);
            cr.line_to(x, (y0 + l) * v.s + v.oy);
            cr.move_to(x0 * v.s + v.ox, y);
            cr.line_to((x0 + l) * v.s + v.ox, y);
        }
        let _ = cr.stroke();
        cr.set_source_rgba(fg, fg, fg, 0.6);
        let mut next_x = f64::MIN;
        let mut next_y = f64::MIN;
        for (u, name) in self.label_starts(v.s) {
            let keep = v.s >= 2.5
                || self.order.get() != 0
                || matches!(name.as_str(), "Genesis" | "Psalms" | "Isaiah" | "Matthew" | "Romans" | "Revelation");
            if !keep {
                continue;
            }
            pl.set_text(&name);
            let (tw, th) = pl.pixel_size();
            let x = (x0 + u * l) * v.s + v.ox;
            if x >= next_x && x > 0.0 && x < w {
                cr.move_to(x + 2.0, h - 18.0);
                pangocairo::functions::show_layout(cr, &pl);
                self.label_at(x + 2.0, h - 18.0, &pl, &name);
                next_x = x + tw as f64 + 6.0;
            }
            let y = (y0 + u * l) * v.s + v.oy;
            if y >= next_y && y > 0.0 && y < h - 30.0 {
                let lx = x0.max(2.0) - tw as f64 - 6.0;
                cr.move_to(lx, y);
                pangocairo::functions::show_layout(cr, &pl);
                self.label_at(lx, y, &pl, &name);
                next_y = y + th as f64 + 2.0;
            }
        }
    }
}

impl Section for CrossRefs {
    fn id(&self) -> &'static str {
        "crossrefs"
    }
    fn title(&self) -> &'static str {
        "Cross-references"
    }
    fn icon(&self) -> &'static str {
        "emblem-shared-symbolic"
    }
    fn widget(&self) -> gtk::Widget {
        self.root.clone().upcast()
    }
    fn in_sidebar(&self) -> bool {
        false
    }
    fn shown(&self) {
        // the first visit opens the explanations once
        {
            let mut c = self.ctx.config.borrow_mut();
            if !c.dismissed_tips.iter().any(|t| t == "xref-help") {
                c.dismissed_tips.push("xref-help".into());
                c.save();
                self.help.set_active(true);
            }
        }
        self.open();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn every_grouping_holds_each_book_once() {
        let canon: HashSet<&str> = bible_core::plans::books().into_iter().collect();
        for (name, b) in [
            ("age", AGE),
            ("writers", WRITERS),
            ("language", LANGUAGE),
            ("culture", CULTURE),
            ("genre", GENRE),
            ("tanakh", TANAKH),
        ] {
            let mut seen = HashSet::new();
            for (_, list) in b {
                for book in *list {
                    assert!(seen.insert(*book), "{name}: {book} twice");
                    assert!(canon.contains(book), "{name}: {book} is not a book");
                }
            }
            assert_eq!(seen.len(), 66, "{name}: {} books", seen.len());
        }
    }

    #[test]
    fn every_group_is_explained() {
        for b in [AGE, WRITERS, LANGUAGE, CULTURE, GENRE, TANAKH] {
            for (g, _) in b {
                assert!(bucket_help(g).is_some(), "no explanation for {g}");
            }
        }
        for f in FORMS {
            assert!(bucket_help(f).is_some(), "no explanation for {f}");
        }
        assert_eq!(ORDERS.len(), ORDER_HELP.len());
    }

    #[test]
    fn literary_forms() {
        for (b, c, want) in [
            ("Ps", 23, "Songs and poems"),
            ("Luke", 15, "Parables"),
            ("John", 17, "Prayers"),
            ("Matt", 5, "Sermons and speeches"),
            ("Lev", 11, "Law"),
            ("1Chr", 3, "Genealogies"),
            ("Rev", 4, "Visions"),
            ("Prov", 3, "Wisdom sayings"),
            ("Isa", 40, "Prophecy"),
            ("Rom", 8, "Letters"),
            ("Gen", 22, "Narrative"),
            ("Exod", 15, "Songs and poems"),
        ] {
            assert_eq!(form(b, c), want, "{b} {c}");
            assert!(FORMS.contains(&form(b, c)));
        }
    }

    #[test]
    fn positions_follow_the_order() {
        let g = Geo {
            verses: vec![10, 20, 30],
            slot_of: vec![2, 0, 1],
        };
        // chapter 1 sits first, chapter 0 last
        assert!(g.u(1, None) < g.u(2, None) && g.u(2, None) < g.u(0, None));
        // verses stay inside their chapter
        let (a, b) = (g.u(1, Some(1)), g.u(1, Some(20)));
        assert!(a < b && a >= 0.0 && b <= 1.0 / 3.0 + 1e-9);
    }

    #[test]
    fn chapter_totals_are_symmetric() {
        let v: Vec<VLink> = vec![[0, 1, 2, 1], [2, 1, 0, 3], [1, 1, 1, 2]];
        let c = by_chapter(&v);
        assert_eq!(c, vec![(0, 2, 2)], "same-chapter links are left out");
    }
}
