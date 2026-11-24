//! The church fathers: the Ante-Nicene and Nicene and Post-Nicene Fathers
//! (Schaff and Roberts–Donaldson, public domain), downloaded one volume at a
//! time from the Christian Classics Ethereal Library when asked, then read
//! offline.

use crate::ctx::{sidebar_button, Ctx, Section};
use adw::prelude::*;
use gtk::{gio, glib};
use serde::{Deserialize, Serialize};
use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::{Rc, Weak};

/// (id, series, volume title). The id is CCEL's.
pub const VOLUMES: [(&str, &str, &str); 37] = [
    ("anf01", "Ante-Nicene Fathers", "I. The Apostolic Fathers, Justin Martyr, Irenaeus"),
    ("anf02", "Ante-Nicene Fathers", "II. Hermas, Tatian, Athenagoras, Theophilus, Clement of Alexandria"),
    ("anf03", "Ante-Nicene Fathers", "III. Tertullian"),
    ("anf04", "Ante-Nicene Fathers", "IV. Tertullian, Minucius Felix, Commodian, Origen"),
    ("anf05", "Ante-Nicene Fathers", "V. Hippolytus, Cyprian, Caius, Novatian"),
    ("anf06", "Ante-Nicene Fathers", "VI. Gregory Thaumaturgus, Dionysius, Julius Africanus, Methodius, Arnobius"),
    ("anf07", "Ante-Nicene Fathers", "VII. Lactantius, Victorinus, the Apostolic Constitutions"),
    ("anf08", "Ante-Nicene Fathers", "VIII. The Twelve Patriarchs, the Clementina, Apocrypha, Syriac documents"),
    ("anf09", "Ante-Nicene Fathers", "IX. The Gospel of Peter, the Diatessaron, Origen on John and Matthew"),
    ("npnf101", "Nicene and Post-Nicene Fathers, Series I", "I. Augustine: Confessions, Letters"),
    ("npnf102", "Nicene and Post-Nicene Fathers, Series I", "II. Augustine: City of God, Christian Doctrine"),
    ("npnf103", "Nicene and Post-Nicene Fathers, Series I", "III. Augustine: On the Trinity, doctrinal and moral treatises"),
    ("npnf104", "Nicene and Post-Nicene Fathers, Series I", "IV. Augustine: Against the Manichaeans and Donatists"),
    ("npnf105", "Nicene and Post-Nicene Fathers, Series I", "V. Augustine: Against the Pelagians"),
    ("npnf106", "Nicene and Post-Nicene Fathers, Series I", "VI. Augustine: Sermon on the Mount, Harmony of the Gospels"),
    ("npnf107", "Nicene and Post-Nicene Fathers, Series I", "VII. Augustine: Homilies on John and First John, Soliloquies"),
    ("npnf108", "Nicene and Post-Nicene Fathers, Series I", "VIII. Augustine: Expositions on the Psalms"),
    ("npnf109", "Nicene and Post-Nicene Fathers, Series I", "IX. Chrysostom: On the Priesthood, Homilies on the Statues"),
    ("npnf110", "Nicene and Post-Nicene Fathers, Series I", "X. Chrysostom: Homilies on Matthew"),
    ("npnf111", "Nicene and Post-Nicene Fathers, Series I", "XI. Chrysostom: Homilies on Acts and Romans"),
    ("npnf112", "Nicene and Post-Nicene Fathers, Series I", "XII. Chrysostom: Homilies on Corinthians"),
    ("npnf113", "Nicene and Post-Nicene Fathers, Series I", "XIII. Chrysostom: Homilies on Galatians to Philemon"),
    ("npnf114", "Nicene and Post-Nicene Fathers, Series I", "XIV. Chrysostom: Homilies on John and Hebrews"),
    ("npnf201", "Nicene and Post-Nicene Fathers, Series II", "I. Eusebius: Church History, Life of Constantine"),
    ("npnf202", "Nicene and Post-Nicene Fathers, Series II", "II. Socrates and Sozomen: Church Histories"),
    ("npnf203", "Nicene and Post-Nicene Fathers, Series II", "III. Theodoret, Jerome, Gennadius, Rufinus"),
    ("npnf204", "Nicene and Post-Nicene Fathers, Series II", "IV. Athanasius: Select Works and Letters"),
    ("npnf205", "Nicene and Post-Nicene Fathers, Series II", "V. Gregory of Nyssa"),
    ("npnf206", "Nicene and Post-Nicene Fathers, Series II", "VI. Jerome: Letters and Select Works"),
    ("npnf207", "Nicene and Post-Nicene Fathers, Series II", "VII. Cyril of Jerusalem, Gregory Nazianzen"),
    ("npnf208", "Nicene and Post-Nicene Fathers, Series II", "VIII. Basil: Letters and Select Works"),
    ("npnf209", "Nicene and Post-Nicene Fathers, Series II", "IX. Hilary of Poitiers, John of Damascus"),
    ("npnf210", "Nicene and Post-Nicene Fathers, Series II", "X. Ambrose: Select Works and Letters"),
    ("npnf211", "Nicene and Post-Nicene Fathers, Series II", "XI. Sulpitius Severus, Vincent of Lérins, John Cassian"),
    ("npnf212", "Nicene and Post-Nicene Fathers, Series II", "XII. Leo the Great, Gregory the Great"),
    ("npnf213", "Nicene and Post-Nicene Fathers, Series II", "XIII. Gregory the Great, Ephraim Syrus, Aphrahat"),
    ("npnf214", "Nicene and Post-Nicene Fathers, Series II", "XIV. The Seven Ecumenical Councils"),
];

pub fn title_of(id: &str) -> &'static str {
    VOLUMES.iter().find(|v| v.0 == id).map(|v| v.2).unwrap_or("")
}

pub fn url(id: &str) -> String {
    format!("https://ccel.org/ccel/s/schaff/{id}.xml")
}

pub fn dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("bible-app/fathers")
}

fn xml_path(id: &str) -> PathBuf {
    dir().join(format!("{id}.xml"))
}
/// Bump the number when parsing changes, so volumes are read again.
const PARSE_VERSION: u32 = 2;

fn cache_path(id: &str) -> PathBuf {
    dir().join(format!("{id}.v{PARSE_VERSION}.json"))
}

pub fn is_installed(id: &str) -> bool {
    xml_path(id).exists()
}

pub fn installed() -> Vec<&'static str> {
    VOLUMES.iter().map(|v| v.0).filter(|id| is_installed(id)).collect()
}

pub fn remove(id: &str) {
    let _ = std::fs::remove_file(xml_path(id));
    // parsed copies of any version
    if let Ok(rd) = std::fs::read_dir(dir()) {
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if n.starts_with(&format!("{id}.")) && n.ends_with(".json") {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
}

/// Download one volume. Fails with a reason a person can act on.
pub fn download(id: &str, progress: &mut dyn FnMut(u64, u64)) -> std::io::Result<()> {
    std::fs::create_dir_all(dir())?;
    let tmp = dir().join(format!("{id}.part"));
    bible_sword::install::download_file(&url(id), &tmp, progress)?;
    let head = std::fs::read(&tmp).map(|b| b.iter().take(4096).copied().collect::<Vec<u8>>())?;
    let head = String::from_utf8_lossy(&head);
    if !head.contains("<ThML") && !head.contains("<div1") {
        let _ = std::fs::remove_file(&tmp);
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "CCEL sent something other than the book (the site may be busy); try again later",
        ));
    }
    std::fs::rename(&tmp, xml_path(id))?;
    let _ = std::fs::remove_file(cache_path(id));
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Part {
    pub title: String,
    /// 1 for a book's main divisions, deeper for chapters inside them
    pub depth: u8,
    pub paras: Vec<String>,
}

/// Remove self-closing tags (<pb n="1"/>, <br/>) that an HTML parser would
/// otherwise leave open around everything after them.
fn strip_empty_tags(xml: &str) -> String {
    let mut out = String::with_capacity(xml.len());
    let mut rest = xml;
    while let Some(i) = rest.find('<') {
        out.push_str(&rest[..i]);
        let tail = &rest[i..];
        match tail.find('>') {
            Some(j) if tail[..j].ends_with('/') => {
                rest = &tail[j + 1..];
            }
            Some(j) => {
                out.push_str(&tail[..=j]);
                rest = &tail[j + 1..];
            }
            None => {
                out.push_str(tail);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

fn clean(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Take out footnotes before parsing: they hold paragraphs of their own, and
/// an HTML parser ends the surrounding paragraph at the first one.
fn strip_notes(xml: &str) -> String {
    let mut out = String::with_capacity(xml.len());
    let mut rest = xml;
    while let Some(i) = rest.find("<note") {
        // only the <note> element, not e.g. <notes>
        let after = rest[i + 5..].chars().next();
        if !matches!(after, Some(' ') | Some('>') | Some('\n') | Some('\t')) {
            out.push_str(&rest[..i + 5]);
            rest = &rest[i + 5..];
            continue;
        }
        out.push_str(&rest[..i]);
        match rest[i..].find("</note>") {
            Some(j) => rest = &rest[i + j + "</note>".len()..],
            None => {
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

/// Read a ThML volume into its parts, in order. Footnotes are left out.
pub fn parse(xml: &str) -> Vec<Part> {
    let body = match xml.find("<ThML.body") {
        Some(i) => &xml[i..],
        None => xml,
    };
    let doc = scraper::Html::parse_fragment(&strip_empty_tags(&strip_notes(body)));
    let mut parts: Vec<Part> = Vec::new();
    fn text_of(n: ego_tree::NodeRef<scraper::Node>, out: &mut String) {
        for c in n.children() {
            match c.value() {
                scraper::Node::Text(t) => out.push_str(t),
                scraper::Node::Element(e) if e.name() == "note" => {}
                scraper::Node::Element(_) => text_of(c, out),
                _ => {}
            }
        }
    }
    fn walk(n: ego_tree::NodeRef<scraper::Node>, parts: &mut Vec<Part>) {
        for c in n.children() {
            let scraper::Node::Element(e) = c.value() else { continue };
            let name = e.name();
            if let Some(d) = name.strip_prefix("div").and_then(|d| d.parse::<u8>().ok()) {
                let title = clean(e.attr("title").unwrap_or(""));
                parts.push(Part {
                    title: if title.is_empty() { "Untitled".into() } else { title },
                    depth: d,
                    paras: vec![],
                });
                walk(c, parts);
            } else if name == "p" || name.len() == 2 && name.starts_with('h') {
                let mut t = String::new();
                text_of(c, &mut t);
                let t = clean(&t);
                if !t.is_empty() {
                    if parts.is_empty() {
                        parts.push(Part { title: "Front matter".into(), depth: 1, paras: vec![] });
                    }
                    let last = parts.last_mut().unwrap();
                    // a heading that repeats the part's title adds nothing
                    if !(name.starts_with('h') && t.eq_ignore_ascii_case(&last.title)) {
                        last.paras.push(t);
                    }
                }
            } else if name != "note" {
                walk(c, parts);
            }
        }
    }
    walk(doc.tree.root(), &mut parts);
    // empty divisions that only hold others stay, as headings in the contents
    parts
}

/// A volume's parts, parsed once and then kept beside the download.
pub fn load(id: &str) -> Option<Vec<Part>> {
    if let Some(p) = std::fs::read(cache_path(id))
        .ok()
        .and_then(|b| serde_json::from_slice::<Vec<Part>>(&b).ok())
    {
        return Some(p);
    }
    let xml = std::fs::read(xml_path(id)).ok()?;
    let parts = parse(&String::from_utf8_lossy(&xml));
    if let Ok(b) = serde_json::to_vec(&parts) {
        let _ = std::fs::write(cache_path(id), b);
    }
    Some(parts)
}

thread_local! {
    static VIEW: RefCell<Weak<FathersView>> = RefCell::new(Weak::new());
}

/// Open a downloaded volume in the reader.
pub fn open(ctx: &Ctx, id: &str) {
    if let Some(v) = VIEW.with(|v| v.borrow().upgrade()) {
        v.open(id);
    }
    ctx.go("fathers");
}

pub struct FathersView {
    ctx: Rc<Ctx>,
    root: adw::ToolbarView,
    title: adw::WindowTitle,
    toc: gtk::ListBox,
    toc_pop: gtk::Popover,
    body: gtk::Box,
    scroll: gtk::ScrolledWindow,
    prev: gtk::Button,
    next: gtk::Button,
    stack: gtk::Stack,
    id: RefCell<String>,
    parts: RefCell<Vec<Part>>,
    at: Cell<usize>,
    me: RefCell<Weak<FathersView>>,
}

impl FathersView {
    pub fn new(ctx: &Rc<Ctx>) -> Rc<FathersView> {
        let root = adw::ToolbarView::new();
        let hb = adw::HeaderBar::new();
        hb.pack_start(&sidebar_button(ctx));
        let back = gtk::Button::from_icon_name("go-previous-symbolic");
        back.set_tooltip_text(Some("Back to the Reading room"));
        let c = ctx.clone();
        back.connect_clicked(move |_| c.go("books"));
        hb.pack_start(&back);
        let title = adw::WindowTitle::new("", "");
        hb.set_title_widget(Some(&title));
        let prev = gtk::Button::from_icon_name("go-up-symbolic");
        prev.set_tooltip_text(Some("Previous section"));
        let next = gtk::Button::from_icon_name("go-down-symbolic");
        next.set_tooltip_text(Some("Next section"));
        let toc = gtk::ListBox::new();
        toc.add_css_class("navigation-sidebar");
        let toc_scroll = gtk::ScrolledWindow::builder()
            .child(&toc)
            .min_content_width(440)
            .max_content_height(720)
            .propagate_natural_height(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build();
        let toc_pop = gtk::Popover::new();
        toc_pop.set_child(Some(&toc_scroll));
        toc.set_selection_mode(gtk::SelectionMode::Single);
        let toc_btn = gtk::MenuButton::builder()
            .child(
                &adw::ButtonContent::builder()
                    .icon_name("view-list-symbolic")
                    .label("Contents")
                    .build(),
            )
            .popover(&toc_pop)
            .tooltip_text("Contents of this volume")
            .build();
        hb.pack_end(&next);
        hb.pack_end(&prev);
        hb.pack_end(&toc_btn);
        root.add_top_bar(&hb);

        let body = gtk::Box::new(gtk::Orientation::Vertical, 14);
        body.set_margin_top(28);
        body.set_margin_bottom(48);
        body.set_margin_start(24);
        body.set_margin_end(24);
        let clamp = adw::Clamp::builder().maximum_size(720).child(&body).build();
        let scroll = gtk::ScrolledWindow::builder()
            .child(&clamp)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .build();
        let loading = gtk::Box::new(gtk::Orientation::Vertical, 14);
        loading.set_valign(gtk::Align::Center);
        loading.set_halign(gtk::Align::Center);
        loading.append(&crate::loader::new(96));
        let ll = gtk::Label::new(Some("Opening the volume…"));
        ll.add_css_class("title-4");
        loading.append(&ll);
        let stack = gtk::Stack::new();
        stack.add_named(&scroll, Some("text"));
        stack.add_named(&loading, Some("loading"));
        root.set_content(Some(&stack));

        let v = Rc::new(FathersView {
            ctx: ctx.clone(),
            root,
            title,
            toc: toc.clone(),
            toc_pop,
            body,
            scroll,
            prev: prev.clone(),
            next: next.clone(),
            stack,
            id: RefCell::new(String::new()),
            parts: RefCell::new(vec![]),
            at: Cell::new(0),
            me: RefCell::new(Weak::new()),
        });
        *v.me.borrow_mut() = Rc::downgrade(&v);
        VIEW.with(|w| *w.borrow_mut() = Rc::downgrade(&v));
        // open the contents at the section being read
        let w = Rc::downgrade(&v);
        v.toc_pop.connect_show(move |_| {
            if let Some(v) = w.upgrade() {
                if let Some(row) = v.toc.row_at_index(v.at.get() as i32) {
                    v.toc.select_row(Some(&row));
                    let r = row.clone();
                    glib::idle_add_local_once(move || {
                        r.grab_focus();
                    });
                }
            }
        });
        let w = Rc::downgrade(&v);
        toc.connect_row_activated(move |_, row| {
            if let Some(v) = w.upgrade() {
                v.toc_pop.popdown();
                v.show(row.index() as usize);
            }
        });
        for (b, step) in [(&prev, -1i64), (&next, 1)] {
            let w = Rc::downgrade(&v);
            b.connect_clicked(move |_| {
                if let Some(v) = w.upgrade() {
                    let i = v.at.get() as i64 + step;
                    if i >= 0 && (i as usize) < v.parts.borrow().len() {
                        v.show(i as usize);
                    }
                }
            });
        }
        v
    }

    fn open(self: &Rc<Self>, id: &str) {
        if *self.id.borrow() == id && !self.parts.borrow().is_empty() {
            return;
        }
        *self.id.borrow_mut() = id.to_string();
        self.title.set_title(title_of(id));
        self.title.set_subtitle("");
        self.stack.set_visible_child_name("loading");
        let id2 = id.to_string();
        let w = Rc::downgrade(self);
        glib::spawn_future_local(async move {
            let parts = gio::spawn_blocking(move || load(&id2)).await.ok().flatten();
            let Some(v) = w.upgrade() else { return };
            let parts = parts.unwrap_or_default();
            while let Some(c) = v.toc.first_child() {
                v.toc.remove(&c);
            }
            for p in &parts {
                let l = gtk::Label::new(Some(&p.title));
                l.set_xalign(0.0);
                l.set_ellipsize(gtk::pango::EllipsizeMode::End);
                l.set_max_width_chars(48);
                l.set_tooltip_text(Some(&p.title));
                l.set_margin_start(((p.depth.max(1) - 1) as i32) * 14);
                if p.depth <= 1 {
                    l.add_css_class("heading");
                }
                v.toc.append(&l);
            }
            *v.parts.borrow_mut() = parts;
            let place = v
                .ctx
                .config
                .borrow()
                .fathers_place
                .get(&*v.id.borrow())
                .copied()
                .unwrap_or(0);
            v.stack.set_visible_child_name("text");
            v.show(place);
        });
    }

    fn show(&self, i: usize) {
        let parts = self.parts.borrow();
        while let Some(c) = self.body.first_child() {
            self.body.remove(&c);
        }
        let Some(p) = parts.get(i) else {
            let l = gtk::Label::new(Some("This volume couldn’t be read. Remove it in Downloads and download it again."));
            l.set_wrap(true);
            self.body.append(&l);
            return;
        };
        self.at.set(i);
        self.title.set_subtitle(&p.title);
        let h = gtk::Label::new(Some(&p.title));
        h.set_wrap(true);
        h.set_xalign(0.0);
        h.add_css_class("title-1");
        h.add_css_class("serif");
        // focus rests on the heading, so no paragraph starts out selected
        h.set_focusable(true);
        self.body.append(&h);
        {
            let h = h.clone();
            glib::idle_add_local_once(move || {
                h.grab_focus();
            });
        }
        if p.paras.is_empty() {
            // a division that only holds others: list them to open
            let kids: Vec<(usize, &Part)> = parts
                .iter()
                .enumerate()
                .skip(i + 1)
                .take_while(|(_, q)| q.depth > p.depth)
                .filter(|(_, q)| q.depth == p.depth + 1)
                .collect();
            if !kids.is_empty() {
                let list = gtk::ListBox::new();
                list.add_css_class("boxed-list");
                list.set_selection_mode(gtk::SelectionMode::None);
                list.set_margin_top(8);
                let mut targets = Vec::new();
                for (j, q) in &kids {
                    let row = adw::ActionRow::builder()
                        .title(glib::markup_escape_text(&q.title).as_str())
                        .activatable(true)
                        .build();
                    row.set_title_lines(2);
                    row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
                    list.append(&row);
                    targets.push(*j);
                }
                let w = self.me.borrow().clone();
                list.connect_row_activated(move |_, row| {
                    if let (Some(v), Some(j)) = (w.upgrade(), targets.get(row.index() as usize)) {
                        let j = *j;
                        glib::idle_add_local_once(move || v.show(j));
                    }
                });
                self.body.append(&list);
            }
        }
        for t in &p.paras {
            let l = gtk::Label::new(Some(t));
            l.set_wrap(true);
            l.set_xalign(0.0);
            l.set_selectable(true);
            l.add_css_class("scripture");
            l.add_css_class("book-text");
            self.body.append(&l);
        }
        self.prev.set_sensitive(i > 0);
        self.next.set_sensitive(i + 1 < parts.len());
        self.scroll.vadjustment().set_value(0.0);
        let id = self.id.borrow().clone();
        let mut cfg = self.ctx.config.borrow_mut();
        cfg.fathers_place.insert(id, i);
        cfg.save();
    }
}

impl Section for FathersView {
    fn id(&self) -> &'static str {
        "fathers"
    }
    fn title(&self) -> &'static str {
        "Church fathers"
    }
    fn icon(&self) -> &'static str {
        "accessories-text-editor-symbolic"
    }
    fn widget(&self) -> gtk::Widget {
        self.root.clone().upcast()
    }
    fn in_sidebar(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<?xml version="1.0"?>
<ThML><ThML.head><generalInfo><description>x</description></generalInfo></ThML.head>
<ThML.body>
<div1 title="The Epistle of Ignatius to the Ephesians" id="v.ii">
<pb n="49"/>
<h2>The Epistle of Ignatius to the Ephesians</h2>
<div2 title="Chapter I.—Praise of the Ephesians." id="v.ii.i">
<p class="normal">I have become acquainted with your name, much-beloved in God,<note n="1">Or, beloved name.</note> which ye have acquired by the habit of righteousness.</p>
<pb n="50"/>
<p>Being the followers of God, and stirring up yourselves by the <scripRef passage="Heb 10:24">blood of God</scripRef>, ye have perfectly accomplished the work.</p>
</div2>
<div2 title="Chapter II.—Congratulations and entreaties." id="v.ii.ii"><p>As to my fellow-servant Burrhus.</p></div2>
</div1>
</ThML.body></ThML>"#;

    #[test]
    fn reads_parts_in_order_without_footnotes() {
        let p = parse(SAMPLE);
        let titles: Vec<&str> = p.iter().map(|x| x.title.as_str()).collect();
        assert_eq!(
            titles,
            [
                "The Epistle of Ignatius to the Ephesians",
                "Chapter I.—Praise of the Ephesians.",
                "Chapter II.—Congratulations and entreaties."
            ]
        );
        assert_eq!(p[0].depth, 1);
        assert_eq!(p[1].depth, 2);
        // the repeated heading is dropped, the footnote left out
        assert!(p[0].paras.is_empty());
        assert_eq!(p[1].paras.len(), 2);
        assert!(p[1].paras[0].starts_with("I have become acquainted"));
        assert!(!p[1].paras[0].contains("Or, beloved name"));
        assert!(p[1].paras[1].contains("blood of God"));
        assert_eq!(p[2].paras, vec!["As to my fellow-servant Burrhus."]);
    }

    #[test]
    fn footnotes_with_paragraphs_do_not_split_text() {
        let xml = r#"<ThML.body><div2 title="Ch"><p>Through envy, those women, the Danaids<note n="18"><p>Some suppose these to have been the names of two martyrs.</p></note> and Dircae, suffered.</p></div2></ThML.body>"#;
        let p = parse(xml);
        assert_eq!(p[0].paras, vec!["Through envy, those women, the Danaids and Dircae, suffered."]);
    }

    #[test]
    fn page_breaks_do_not_swallow_text() {
        let s = strip_empty_tags(r#"<p>a</p><pb n="2"/><p>b<br/>c</p>"#);
        assert_eq!(s, "<p>a</p><p>bc</p>");
    }

    #[test]
    fn catalogue_is_complete_and_unique() {
        let mut ids: Vec<&str> = VOLUMES.iter().map(|v| v.0).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 37);
        assert!(url("anf01").ends_with("/schaff/anf01.xml"));
    }
}
