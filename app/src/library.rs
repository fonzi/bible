//! The Library: modules you have, and modules you can download from SWORD sources.

use crate::ctx::{sidebar_button, Ctx, Section};
use adw::prelude::*;
use bible_sword::install::{self, RemoteModule};
use bible_sword::{Kind, Module};
use gtk::{gio, glib};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;

pub struct LibraryView {
    ctx: Rc<Ctx>,
    root: adw::ToolbarView,
    installed: adw::PreferencesPage,
    browse_box: gtk::Box,
    browse_list: gtk::Box,
    browse_stack: gtk::Stack,
    tabs: adw::ViewStack,
    browse_status: adw::StatusPage,
    search: gtk::SearchEntry,
    lang_dd: gtk::DropDown,
    langs: RefCell<Vec<String>>,
    /// None: Start here; Some(kind_order): one kind of work
    category: std::cell::Cell<Option<u8>>,
    cat_list: gtk::ListBox,
    catalogue: RefCell<Vec<RemoteModule>>,
    legal: gtk::Revealer,
    legal_seen: std::cell::Cell<bool>,
    errors: RefCell<Vec<String>>,
}

/// Widely used free works, in the order we show them. Only those a source offers appear.
const POPULAR: &[(&str, &str)] = &[
    ("KJV", "King James Version"),
    ("BSB", "Berean Standard Bible"),
    ("engWEB2015eb", "World English Bible"),
    ("WEB", "World English Bible"),
    ("ASV", "American Standard Version"),
    ("NETfree", "NET Bible"),
    ("LEB", "Lexham English Bible"),
    ("YLT", "Young's Literal Translation"),
    ("Geneva1599", "Geneva Bible"),
    ("DRC", "Douay-Rheims"),
    ("KJVA", "KJV with Apocrypha"),
    ("SBLGNT", "SBL Greek New Testament"),
    ("TR", "Textus Receptus"),
    ("OSHB", "Open Scriptures Hebrew Bible"),
    ("WLC", "Westminster Leningrad Codex"),
    ("Vulgate", "Latin Vulgate"),
    ("SpaRV1909", "Reina-Valera 1909"),
    ("GerLut1545", "Luther 1545"),
    ("FreSegond1910", "Louis Segond"),
    ("MHCC", "Matthew Henry Concise"),
    ("MHC", "Matthew Henry Complete"),
    ("TSK", "Treasury of Scripture Knowledge"),
    ("Barnes", "Barnes' Notes"),
    ("Clarke", "Adam Clarke"),
    ("JFB", "Jamieson-Fausset-Brown"),
    ("CalvinCommentaries", "Calvin's Commentaries"),
    ("RWP", "Robertson's Word Pictures"),
    ("StrongsGreek", "Strong's Greek"),
    ("StrongsHebrew", "Strong's Hebrew"),
    ("Institutes", "Calvin's Institutes"),
    ("Pilgrim", "Pilgrim's Progress"),
    ("Daily", "Daily Light"),
    ("SME", "Spurgeon's Morning and Evening"),
];

/// "Strong’s word study · Footnotes · …" for a Bible's subtitle.
fn feature_line(conf: &bible_sword::conf::Conf) -> String {
    conf.features()
        .iter()
        .map(|f| f.label())
        .collect::<Vec<_>>()
        .join(" · ")
}

/// The richest Bible among candidates; ties go to the more popular one.
fn best_bible<'a, T>(
    items: impl Iterator<Item = &'a T>,
    conf: impl Fn(&T) -> &bible_sword::conf::Conf,
    name: impl Fn(&T) -> String,
) -> Option<&'a T>
where
    T: 'a,
{
    let rank = |n: &str| {
        POPULAR
            .iter()
            .position(|(p, _)| p.eq_ignore_ascii_case(n))
            .unwrap_or(usize::MAX)
    };
    items
        .filter(|m| Kind::from_conf(conf(m)) == Kind::Bible && conf(m).feature_score() > 0)
        .max_by(|a, b| {
            conf(a)
                .feature_score()
                .cmp(&conf(b).feature_score())
                .then(rank(&name(b)).cmp(&rank(&name(a))))
        })
}

const RECOMMEND_NOTE: &str = "Turns on the most features: tap a word for its Greek or Hebrew and Strong’s entry, grammar, footnotes and cross-references, red letters and headings. Maps, the timeline, intros from commentaries and the interlinear work with every Bible.";

/// How literally a Bible translates, as Wikipedia's article "Dynamic and
/// formal equivalence" classifies it (matched on the translation's name).
/// None for translations that article doesn't list.
fn approach(description: &str) -> Option<&'static str> {
    let d = description.to_lowercase();
    let any = |names: &[&str]| names.iter().any(|n| d.contains(n));
    // the more specific names first
    if any(&["living bible", "the message", "the voice", "passion translation", "street bible"]) {
        return Some("Paraphrase");
    }
    if any(&["reader's version", "revised english bible", "new english bible", "new jerusalem", "jerusalem bible",
             "good news", "contemporary english", "new living", "god's word", "god’s word", "easy-to-read",
             "new english translation", "knox", "amplified", "complete jewish", "new life version",
             "christian community bible", "today's new international", "today’s new international"]) {
        return Some("Thought for thought");
    }
    if any(&["new world translation", "new international version", "christian standard", "common english bible",
             "new american bible", "modern language bible", "evangelical heritage", "new catholic"]) {
        return Some("Balanced");
    }
    if any(&["king james", "young's literal", "young’s literal", "douay", "revised version", "american standard",
             "revised standard", "world english", "english standard", "lexham", "literal standard", "concordant",
             "green's literal", "modern english version", "tree of life", "recovery version", "third millennium",
             "orthodox study"]) {
        return Some("Word for word");
    }
    None
}

/// The category a catalogue entry is listed under in Browse: the kinds of
/// work, then CrossWire's maps and pictures, and other traditions last.
fn browse_key(conf: &bible_sword::conf::Conf) -> u8 {
    let cat = conf.get_or("Category", "").to_ascii_lowercase();
    if cat.contains("cult") || cat.contains("unorthodox") || cat.contains("questionable") {
        7
    } else if cat.contains("map") || cat.contains("image") {
        6
    } else {
        kind_order(Kind::from_conf(conf))
    }
}

fn browse_label(key: u8, conf: &bible_sword::conf::Conf) -> &'static str {
    match key {
        6 => "Maps and pictures",
        7 => "Other traditions",
        _ => Kind::from_conf(conf).label(),
    }
}

fn kind_order(k: Kind) -> u8 {
    match k {
        Kind::Bible => 0,
        Kind::Commentary => 1,
        Kind::Book => 2,
        Kind::Devotional => 3,
        Kind::Lexicon => 4,
        Kind::Other => 5,
    }
}

/// Every ISO 639 language and ISO 3166 region name, keyed by code (regions
/// as "R" + code), so the catalogue never shows a bare code.
fn iso_names() -> &'static std::collections::HashMap<&'static str, &'static str> {
    static T: std::sync::OnceLock<std::collections::HashMap<&str, &str>> = std::sync::OnceLock::new();
    T.get_or_init(|| {
        include_str!("lang_names.tsv")
            .lines()
            .filter(|l| !l.starts_with('#'))
            .filter_map(|l| l.split_once('\t'))
            .collect()
    })
}

/// "en" → "English", "zh-Hant" → "中文 (Traditional)", "pt-BR" →
/// "Português (Brazil)". The big languages in their own words.
fn lang_name(code: &str) -> String {
    let mut parts = code.split(['-', '_']);
    let base = parts.next().unwrap_or(code).to_ascii_lowercase();
    let own = match base.as_str() {
        "en" => Some("English"),
        "de" => Some("Deutsch"),
        "es" => Some("Español"),
        "fr" => Some("Français"),
        "nl" => Some("Nederlands"),
        "pt" => Some("Português"),
        "it" => Some("Italiano"),
        "ru" => Some("Русский"),
        "zh" => Some("中文"),
        "ko" => Some("한국어"),
        "ja" => Some("日本語"),
        "ar" => Some("العربية"),
        "sw" => Some("Kiswahili"),
        "he" => Some("Hebrew"),
        "el" => Some("Greek"),
        _ => None,
    };
    let names = iso_names();
    let mut name = match own.or_else(|| names.get(base.as_str()).copied()) {
        Some(n) => n.to_string(),
        None => format!("Other ({code})"),
    };
    let extra: Vec<String> = parts
        .filter_map(|t| match t {
            "Hans" => Some("Simplified".to_string()),
            "Hant" => Some("Traditional".to_string()),
            "Latn" => Some("Latin script".to_string()),
            "Cyrl" => Some("Cyrillic".to_string()),
            "Arab" => Some("Arabic script".to_string()),
            "Deva" => Some("Devanagari".to_string()),
            "Grek" => Some("Greek script".to_string()),
            "Hebr" => Some("Hebrew script".to_string()),
            "Ethi" => Some("Ethiopic".to_string()),
            "Syrc" => Some("Syriac script".to_string()),
            t if t.len() == 2 => names
                .get(format!("R{}", t.to_ascii_uppercase()).as_str())
                .map(|r| r.to_string()),
            _ => None,
        })
        .collect();
    if !extra.is_empty() {
        name = format!("{name} ({})", extra.join(", "));
    }
    name
}

impl LibraryView {
    /// The church fathers, one volume at a time from CCEL.
    fn render_fathers(&self) {
        let g = adw::PreferencesGroup::builder()
            .title("Church fathers")
            .description("The Ante-Nicene and Nicene and Post-Nicene Fathers (Roberts, Donaldson and Schaff, public domain), from the Christian Classics Ethereal Library. Each volume is about 3–10 MB and reads offline in the Reading room.")
            .build();
        g.set_widget_name("libgroup");
        let mut series: Vec<(&str, adw::ExpanderRow)> = Vec::new();
        for (id, ser, title) in crate::fathers::VOLUMES {
            let exp = match series.iter().find(|s| s.0 == ser) {
                Some(s) => s.1.clone(),
                None => {
                    let e = adw::ExpanderRow::builder().title(ser).build();
                    g.add(&e);
                    series.push((ser, e.clone()));
                    e
                }
            };
            let row = self.volume_row(id, title);
            exp.add_row(&row);
        }
        for (ser, e) in &series {
            let n = crate::fathers::VOLUMES
                .iter()
                .filter(|v| v.1 == *ser && crate::fathers::is_installed(v.0))
                .count();
            let total = crate::fathers::VOLUMES.iter().filter(|v| v.1 == *ser).count();
            e.set_subtitle(&format!("{n} of {total} volumes downloaded"));
        }
        self.installed.add(&g);
    }

    /// The legal notice shows until it has been read once ("Understood").
    fn legal_due(&self) -> bool {
        let c = self.ctx.config.borrow();
        c.legal_notice && !self.legal_seen.get() && !c.dismissed_tips.iter().any(|t| t == "legal")
    }

    pub fn new(ctx: &Rc<Ctx>) -> Rc<LibraryView> {
        let stack = adw::ViewStack::new();
        let installed = adw::PreferencesPage::new();
        stack.add_titled_with_icon(
            &installed,
            Some("installed"),
            "Installed",
            "emblem-ok-symbolic",
        );

        // browse page
        let search = gtk::SearchEntry::new();
        search.set_placeholder_text(Some("Search Bibles, commentaries, books…"));
        search.set_hexpand(true);
        let lang_dd = gtk::DropDown::from_strings(&["All languages"]);
        lang_dd.set_enable_search(true);
        // every row the same width, so the list doesn't change size (and
        // shake) as longer names scroll into view
        let lf = gtk::SignalListItemFactory::new();
        lf.connect_setup(|_, item| {
            let l = gtk::Label::new(None);
            l.set_xalign(0.0);
            l.set_width_chars(26);
            l.set_max_width_chars(26);
            l.set_ellipsize(gtk::pango::EllipsizeMode::End);
            item.downcast_ref::<gtk::ListItem>().unwrap().set_child(Some(&l));
        });
        lf.connect_bind(|_, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();
            let text = item.item().and_downcast::<gtk::StringObject>().map(|o| o.string()).unwrap_or_default();
            item.child().and_downcast::<gtk::Label>().unwrap().set_text(&text);
        });
        lang_dd.set_list_factory(Some(&lf));
        lang_dd.set_expression(Some(gtk::PropertyExpression::new(
            gtk::StringObject::static_type(),
            None::<&gtk::Expression>,
            "string",
        )));
        let refresh = gtk::Button::from_icon_name("view-refresh-symbolic");
        refresh.set_tooltip_text(Some("Refresh catalogue"));
        let bar = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        bar.append(&search);
        bar.append(&lang_dd);
        bar.append(&refresh);
        let browse_list = gtk::Box::new(gtk::Orientation::Vertical, 18);
        let browse_status = adw::StatusPage::builder()
            .icon_name("folder-download-symbolic")
            .title("Browse the library")
            .description("Fetch the catalogue from your sources to see Bibles, commentaries and books you can download.")
            .build();
        let fetch = gtk::Button::with_label("Load catalogue");
        fetch.add_css_class("pill");
        fetch.add_css_class("suggested-action");
        fetch.set_halign(gtk::Align::Center);
        browse_status.set_child(Some(&fetch));
        let spinner_page = gtk::Box::new(gtk::Orientation::Vertical, 12);
        spinner_page.set_valign(gtk::Align::Center);
        let spinner = crate::loader::new(72);
        let sp_label = gtk::Label::new(Some("Loading catalogue…"));
        sp_label.add_css_class("dim-label");
        spinner_page.append(&spinner);
        spinner_page.append(&sp_label);
        let browse_stack = gtk::Stack::new();
        browse_stack.set_transition_type(gtk::StackTransitionType::Crossfade);
        browse_stack.add_named(&browse_status, Some("status"));
        browse_stack.add_named(&spinner_page, Some("loading"));
        let list_scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .build();
        let list_clamp = adw::Clamp::builder()
            .maximum_size(860)
            .child(&browse_list)
            .build();
        list_clamp.set_margin_top(12);
        list_clamp.set_margin_bottom(24);
        list_clamp.set_margin_start(12);
        list_clamp.set_margin_end(12);
        list_scroll.set_child(Some(&list_clamp));
        // categories down the left, so the long catalogue comes a kind at a time
        let cat_list = gtk::ListBox::new();
        cat_list.add_css_class("navigation-sidebar");
        cat_list.set_selection_mode(gtk::SelectionMode::Single);
        let cat_scroll = gtk::ScrolledWindow::builder()
            .child(&cat_list)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .width_request(250)
            .build();
        let list_page = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        list_page.append(&cat_scroll);
        list_page.append(&gtk::Separator::new(gtk::Orientation::Vertical));
        list_scroll.set_hexpand(true);
        list_page.append(&list_scroll);
        browse_stack.add_named(&list_page, Some("list"));
        let browse_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let bar_clamp = adw::Clamp::builder().maximum_size(860).child(&bar).build();
        bar_clamp.set_margin_top(12);
        bar_clamp.set_margin_start(12);
        bar_clamp.set_margin_end(12);
        bar_clamp.set_margin_bottom(12);
        browse_box.append(&bar_clamp);
        browse_box.append(&browse_stack);
        stack.add_titled_with_icon(
            &browse_box,
            Some("browse"),
            "Browse",
            "folder-download-symbolic",
        );

        // legal notice
        let legal_label = gtk::Label::new(None);
        legal_label.set_markup("<b>Before you download:</b> in some countries owning or reading religious texts is restricted or illegal. Please take precautions. Everything here works offline once installed. You can turn this notice off in Preferences.");
        legal_label.set_wrap(true);
        legal_label.set_xalign(0.0);
        legal_label.set_hexpand(true);
        let legal_ok = gtk::Button::with_label("Understood");
        legal_ok.set_valign(gtk::Align::Center);
        let legal_row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        legal_row.add_css_class("warning-bar");
        legal_row.append(&legal_label);
        legal_row.append(&legal_ok);
        let legal = gtk::Revealer::builder()
            .child(&legal_row)
            .transition_type(gtk::RevealerTransitionType::SlideDown)
            .build();

        let header = adw::HeaderBar::new();
        header.pack_start(&sidebar_button(ctx));
        let sw = adw::ViewSwitcher::new();
        sw.set_stack(Some(&stack));
        sw.set_policy(adw::ViewSwitcherPolicy::Wide);
        header.set_title_widget(Some(&sw));
        let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
        content.append(&legal);
        content.append(&stack);
        stack.set_vexpand(true);
        let root = adw::ToolbarView::new();
        root.add_top_bar(&header);
        root.set_content(Some(&content));

        let v = Rc::new(LibraryView {
            ctx: ctx.clone(),
            root,
            installed,
            browse_box,
            browse_list,
            browse_stack,
            tabs: stack.clone(),
            browse_status,
            search,
            lang_dd,
            langs: RefCell::new(vec![]),
            category: std::cell::Cell::new(None),
            cat_list: cat_list.clone(),
            catalogue: RefCell::new(vec![]),
            legal,
            legal_seen: std::cell::Cell::new(false),
            errors: RefCell::new(vec![]),
        });
        let w = Rc::downgrade(&v);
        fetch.connect_clicked(move |_| {
            if let Some(v) = w.upgrade() {
                v.load_catalogue();
            }
        });
        let w = Rc::downgrade(&v);
        refresh.connect_clicked(move |_| {
            if let Some(v) = w.upgrade() {
                v.load_catalogue();
            }
        });
        let w = Rc::downgrade(&v);
        v.search.connect_search_changed(move |_| {
            if let Some(v) = w.upgrade() {
                v.render_browse();
            }
        });
        let w = Rc::downgrade(&v);
        v.lang_dd.connect_selected_notify(move |_| {
            if let Some(v) = w.upgrade() {
                v.render_browse();
            }
        });
        let w = Rc::downgrade(&v);
        v.cat_list.connect_row_activated(move |_, row| {
            let Some(v) = w.upgrade() else { return };
            let name = row.widget_name();
            let cat = name.strip_prefix('k').and_then(|n| n.parse::<u8>().ok());
            if v.category.get() == cat {
                return;
            }
            v.category.set(cat);
            // rebuilt after this click is handled, never inside it
            let w = Rc::downgrade(&v);
            glib::idle_add_local_once(move || {
                if let Some(v) = w.upgrade() {
                    v.render_browse();
                }
            });
        });
        let w = Rc::downgrade(&v);
        legal_ok.connect_clicked(move |_| {
            if let Some(v) = w.upgrade() {
                // read once is enough: it stays hidden, and Preferences can bring it back
                v.legal.set_reveal_child(false);
                v.legal_seen.set(true);
                let mut c = v.ctx.config.borrow_mut();
                if !c.dismissed_tips.iter().any(|t| t == "legal") {
                    c.dismissed_tips.push("legal".into());
                }
                c.save();
            }
        });
        v.refresh();
        v
    }

    fn render_installed(&self) {
        // PreferencesPage has no clear(); remove groups one by one
        let mut groups = Vec::new();
        let mut child = self.installed.first_child();
        while let Some(c) = child {
            // page -> scrolled window -> viewport -> clamp -> box -> groups
            groups.push(c.clone());
            child = c.next_sibling();
        }
        let lib = self.ctx.lib();
        let mut by_kind: BTreeMap<u8, Vec<Arc<Module>>> = BTreeMap::new();
        for m in &lib.modules {
            by_kind
                .entry(kind_order(m.kind))
                .or_default()
                .push(m.clone());
        }
        // remove previously added groups we track via widget name
        for g in self.tracked_groups() {
            self.installed.remove(&g);
        }
        // interlinear data packs
        let g = adw::PreferencesGroup::builder()
            .title("Interlinear · STEP Bible")
            .description("Greek and Hebrew word by word, with glosses, grammar and a brief lexicon. Data from STEP Bible (Tyndale House, Cambridge), CC BY 4.0.")
            .build();
        g.set_widget_name("libgroup");
        for pack in bible_step::Pack::all() {
            let installed = bible_step::is_installed(pack);
            let row = adw::ActionRow::builder()
                .title(pack.title())
                .subtitle(if installed {
                    "Installed · works offline"
                } else {
                    pack.description()
                })
                .build();
            row.set_subtitle_lines(3);
            let bar = gtk::ProgressBar::new();
            bar.set_visible(false);
            bar.set_valign(gtk::Align::Center);
            bar.set_size_request(120, -1);
            row.add_suffix(&bar);
            let btn = gtk::Button::with_label(if installed { "Remove" } else { "Download" });
            btn.set_valign(gtk::Align::Center);
            if installed {
                btn.add_css_class("flat");
            }
            let ctx = self.ctx.clone();
            btn.connect_clicked(move |b| {
                if bible_step::is_installed(pack) {
                    if b.label().as_deref() == Some("Remove") {
                        b.set_label("Confirm remove");
                        b.add_css_class("destructive-action");
                        b.remove_css_class("flat");
                        return;
                    }
                    let _ = bible_step::remove(pack);
                    ctx.toast(&format!("Removed the {}", pack.title().to_lowercase()));
                    ctx.refresh_all();
                    return;
                }
                b.set_sensitive(false);
                b.set_label("Downloading");
                bar.set_visible(true);
                bar.set_fraction(0.02);
                let (tx, rx) = std::sync::mpsc::channel::<(usize, usize)>();
                let barp = bar.clone();
                let tick =
                    glib::timeout_add_local(std::time::Duration::from_millis(120), move || {
                        while let Ok((d, t)) = rx.try_recv() {
                            barp.set_fraction((d as f64 / t.max(1) as f64).max(0.02));
                        }
                        glib::ControlFlow::Continue
                    });
                let ctx = ctx.clone();
                let b = b.clone();
                glib::spawn_future_local(async move {
                    let res = gio::spawn_blocking(move || {
                        bible_step::download(pack, &mut |d, t| {
                            let _ = tx.send((d, t));
                        })
                        .map_err(|e| e.to_string())
                    })
                    .await;
                    tick.remove();
                    match res {
                        Ok(Ok(())) => {
                            ctx.toast(&format!("Downloaded the {}", pack.title().to_lowercase()));
                            ctx.refresh_all();
                        }
                        Ok(Err(e)) => {
                            b.set_sensitive(true);
                            b.set_label("Try again");
                            ctx.toast(&format!("Download failed: {e}"));
                        }
                        Err(_) => {
                            b.set_sensitive(true);
                            b.set_label("Try again");
                        }
                    }
                });
            });
            row.add_suffix(&btn);
            g.add(&row);
        }
        self.installed.add(&g);
        self.render_fathers();
        self.render_audio();

        if lib.modules.is_empty() {
            let g = adw::PreferencesGroup::new();
            g.set_widget_name("libgroup");
            let row = adw::ActionRow::builder()
                .title("Nothing installed yet")
                .subtitle("Open Browse to download Bibles, commentaries and books.")
                .build();
            g.add(&row);
            self.installed.add(&g);
            return;
        }
        let best_installed =
            best_bible(lib.modules.iter(), |m| &m.conf, |m| m.name.clone()).map(|m| m.name.clone());
        for (_, mods) in by_kind {
            let g = adw::PreferencesGroup::new();
            g.set_widget_name("libgroup");
            g.set_title(mods[0].kind.label());
            for m in mods {
                let mut sub = vec![m.abbreviation(), lang_name(&m.lang()), m.license()];
                if !m.user_installed {
                    sub.push("installed by your system".into());
                }
                if m.is_locked() {
                    sub.push("locked".into());
                } else if !m.is_readable() && matches!(m.kind, Kind::Bible | Kind::Commentary) {
                    sub.push("format not supported yet".into());
                }
                let mut subtitle = sub.join(" · ");
                if m.kind == Kind::Bible {
                    let f = feature_line(&m.conf);
                    if !f.is_empty() {
                        subtitle = format!("{subtitle}\n{f}");
                    }
                }
                let row = adw::ActionRow::builder()
                    .title(glib::markup_escape_text(&m.description()).as_str())
                    .subtitle(glib::markup_escape_text(&subtitle).as_str())
                    .subtitle_lines(3)
                    .build();
                if best_installed.as_deref() == Some(m.name.as_str()) {
                    let badge = gtk::Label::new(Some("Recommended"));
                    badge.add_css_class("recommended-badge");
                    badge.set_valign(gtk::Align::Center);
                    badge.set_tooltip_text(Some(RECOMMEND_NOTE));
                    row.add_suffix(&badge);
                }
                if m.user_installed {
                    let rm = gtk::Button::with_label("Remove");
                    rm.set_valign(gtk::Align::Center);
                    rm.add_css_class("flat");
                    let ctx = self.ctx.clone();
                    let mm = m.clone();
                    rm.connect_clicked(move |b| {
                        // two-step confirm instead of a dialog
                        if b.label().as_deref() == Some("Remove") {
                            b.set_label("Confirm remove");
                            b.add_css_class("destructive-action");
                            b.remove_css_class("flat");
                            return;
                        }
                        match install::remove(&mm) {
                            Ok(_) => {
                                ctx.toast(&format!("Removed {}", mm.description()));
                                ctx.rescan();
                            }
                            Err(e) => ctx.toast(&e.to_string()),
                        }
                    });
                    row.add_suffix(&rm);
                }
                g.add(&row);
            }
            self.installed.add(&g);
        }
    }

    /// Downloaded chapter audio, by book, each chapter playable and removable.
    fn render_audio(&self) {
        use bible_core::audio;
        for src in audio::SOURCES {
            let g = adw::PreferencesGroup::new();
            g.set_widget_name("libgroup");
            g.set_title(&format!("Audio · {}", src.title));
            let items = audio::downloaded(src);
            let total: u64 = items.iter().map(|d| d.bytes).sum();
            g.set_description(Some(&if items.is_empty() {
                format!("Read by {}. {}", src.narrator, src.licence)
            } else {
                format!(
                    "Read by {} · {} chapter{} · {} · {}",
                    src.narrator,
                    items.len(),
                    if items.len() == 1 { "" } else { "s" },
                    audio::size(total),
                    src.licence
                )
            }));
            if items.is_empty() {
                let row = adw::ActionRow::builder()
                    .title("No audio downloaded yet")
                    .subtitle("Press the headphones button while reading to download a chapter or a whole book.")
                    .build();
                row.add_prefix(&gtk::Image::from_icon_name("audio-headphones-symbolic"));
                g.add(&row);
                self.installed.add(&g);
                continue;
            }
            let mut books: Vec<(String, Vec<audio::Downloaded>)> = vec![];
            for d in items {
                match books.last_mut() {
                    Some((b, v)) if *b == d.book => v.push(d),
                    _ => books.push((d.book.clone(), vec![d])),
                }
            }
            for (book, chapters) in books {
                let name = bible_core::Ref::new(&book, 1, 0).book_name();
                let bytes: u64 = chapters.iter().map(|d| d.bytes).sum();
                let exp = adw::ExpanderRow::builder()
                    .title(glib::markup_escape_text(&name).as_str())
                    .subtitle(format!(
                        "{} chapter{} · {}",
                        chapters.len(),
                        if chapters.len() == 1 { "" } else { "s" },
                        audio::size(bytes)
                    ))
                    .build();
                let rm_book = gtk::Button::with_label("Remove");
                rm_book.add_css_class("flat");
                rm_book.set_valign(gtk::Align::Center);
                let ctx = self.ctx.clone();
                let b2 = book.clone();
                rm_book.connect_clicked(move |b| {
                    if b.label().as_deref() == Some("Remove") {
                        b.set_label("Remove all");
                        b.add_css_class("destructive-action");
                        return;
                    }
                    let _ = audio::remove(src, &b2, None);
                    ctx.refresh_all();
                });
                exp.add_suffix(&rm_book);
                for d in chapters {
                    let row = adw::ActionRow::builder()
                        .title(format!("{name} {}", d.chapter))
                        .subtitle(audio::size(d.bytes))
                        .build();
                    let play = gtk::Button::from_icon_name("media-playback-start-symbolic");
                    play.add_css_class("flat");
                    play.add_css_class("circular");
                    play.set_valign(gtk::Align::Center);
                    play.set_tooltip_text(Some("Play in the reader"));
                    let ctx = self.ctx.clone();
                    let (b, c) = (d.book.clone(), d.chapter);
                    play.connect_clicked(move |_| ctx.play_audio(&b, c));
                    row.add_prefix(&play);
                    let rm = gtk::Button::from_icon_name("user-trash-symbolic");
                    rm.add_css_class("flat");
                    rm.set_valign(gtk::Align::Center);
                    rm.set_tooltip_text(Some("Remove this chapter's audio"));
                    let ctx = self.ctx.clone();
                    let (b, c) = (d.book.clone(), d.chapter);
                    rm.connect_clicked(move |_| {
                        let _ = audio::remove(src, &b, Some(c));
                        ctx.refresh_all();
                    });
                    row.add_suffix(&rm);
                    exp.add_row(&row);
                }
                g.add(&exp);
            }
            self.installed.add(&g);
        }
    }

    fn tracked_groups(&self) -> Vec<adw::PreferencesGroup> {
        let mut out = Vec::new();
        fn walk(w: &gtk::Widget, out: &mut Vec<adw::PreferencesGroup>) {
            if let Ok(g) = w.clone().downcast::<adw::PreferencesGroup>() {
                if g.widget_name() == "libgroup" {
                    out.push(g);
                    return;
                }
            }
            let mut c = w.first_child();
            while let Some(ch) = c {
                walk(&ch, out);
                c = ch.next_sibling();
            }
        }
        walk(self.installed.upcast_ref(), &mut out);
        out
    }

    fn load_catalogue(self: &Rc<Self>) {
        self.browse_stack.set_visible_child_name("loading");
        let sources = self.ctx.config.borrow().sources();
        let w = Rc::downgrade(self);
        glib::spawn_future_local(async move {
            let res = gio::spawn_blocking(move || {
                let mut all = Vec::new();
                let mut errs = Vec::new();
                for s in sources {
                    match install::fetch_catalogue(&s) {
                        Ok(mut v) => all.append(&mut v),
                        Err(e) => errs.push(format!("{}: {e}", s.name)),
                    }
                }
                (all, errs)
            })
            .await;
            let Some(v) = w.upgrade() else { return };
            let (all, errs) = res.unwrap_or_default();
            if all.is_empty() {
                v.browse_status.set_title("Couldn’t load the catalogue");
                v.browse_status.set_description(Some(&format!(
                    "Check your connection or your sources in Preferences.\n{}",
                    errs.join("\n")
                )));
                v.browse_stack.set_visible_child_name("status");
                return;
            }
            let mut count: BTreeMap<String, usize> = BTreeMap::new();
            for m in all.iter() {
                *count.entry(m.lang()).or_default() += 1;
            }
            let mut named: Vec<(String, String)> =
                count.keys().map(|l| (lang_name(l), l.clone())).collect();
            named.sort_by_key(|(n, _)| n.to_lowercase());
            let mut labels = vec![format!("All languages ({})", all.len())];
            labels.extend(named.iter().map(|(n, l)| format!("{n} ({})", count[l])));
            let langs: Vec<String> = named.into_iter().map(|(_, l)| l).collect();
            let lr: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
            v.lang_dd.set_model(Some(&gtk::StringList::new(&lr)));
            // default to English if available
            if let Some(i) = langs.iter().position(|l| l == "en") {
                v.lang_dd.set_selected(i as u32 + 1);
            }
            *v.langs.borrow_mut() = langs;
            *v.catalogue.borrow_mut() = all;
            *v.errors.borrow_mut() = errs;
            v.render_browse();
            v.browse_stack.set_visible_child_name("list");
        });
    }

    fn module_row(&self, m: &RemoteModule) -> adw::ActionRow {
        let lib = self.ctx.lib();
        let have = lib.get(m.name());
        let sub = format!(
            "{} · {} · {} · {}",
            m.name(),
            lang_name(&m.lang()),
            m.source.name,
            m.conf.get("DistributionLicense").unwrap_or("see details")
        );

        let row = adw::ActionRow::builder()
            .title(glib::markup_escape_text(&m.description()).as_str())
            .subtitle(glib::markup_escape_text(&sub).as_str())
            .subtitle_lines(3)
            .build();
        // a few short tags for what a Bible can do (at most three)
        if Kind::from_conf(&m.conf) == Kind::Bible {
            use bible_sword::conf::Feature as F;
            let have = m.conf.features();
            let mut tags: Vec<(&str, &str)> = approach(&m.description())
                .map(|a| (a, "How literally it translates, as classified in Wikipedia’s article “Dynamic and formal equivalence”"))
                .into_iter()
                .collect();
            let room = 3 - tags.len();
            tags.extend::<Vec<(&str, &str)>>([
                (F::CrossRefs, "Cross-references", "Comes with its own cross-references: the rainbow and the other cross-reference views use them"),
                (F::Strongs, "Word study", "Tap a word for its Greek or Hebrew and Strong’s entry"),
                (F::Footnotes, "Footnotes", "Translators’ notes on the text"),
                (F::RedLetters, "Red letters", "The words of Jesus in red"),
                (F::Headings, "Headings", "Section headings in the text"),
            ]
            .into_iter()
            .filter(|(f, _, _)| have.contains(f))
            .map(|(_, t, tip)| (t, tip))
            .take(room)
            .collect());
            let tag_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
            tag_box.set_valign(gtk::Align::Center);
            for (t, tip) in tags {
                let tag = gtk::Label::new(Some(t));
                tag.add_css_class("xref-tag");
                tag.add_css_class("caption");
                tag.set_tooltip_text(Some(tip));
                tag_box.append(&tag);
            }
            if tag_box.first_child().is_some() {
                row.add_suffix(&tag_box);
            }
        }
        let bar = gtk::ProgressBar::new();
        bar.set_visible(false);
        bar.set_valign(gtk::Align::Center);
        bar.set_size_request(120, -1);
        row.add_suffix(&bar);
        let btn = match &have {
            Some(h) if h.conf.get("Version") != m.conf.get("Version") && h.user_installed => {
                let b = gtk::Button::with_label("Update");
                b.add_css_class("suggested-action");
                b
            }
            Some(_) => {
                let b = gtk::Button::with_label("Installed");
                b.set_sensitive(false);
                b.add_css_class("flat");
                b
            }
            None => gtk::Button::with_label("Download"),
        };
        btn.set_valign(gtk::Align::Center);
        let ctx = self.ctx.clone();
        let mm = m.clone();
        let barc = bar.clone();
        btn.connect_clicked(move |b| {
            b.set_sensitive(false);
            b.set_label("Downloading");
            barc.set_visible(true);
            barc.set_fraction(0.0);
            let (tx, rx) = std::sync::mpsc::channel::<(usize, usize)>();
            let barp = barc.clone();
            let tick = glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                while let Ok((d, t)) = rx.try_recv() {
                    barp.set_fraction(if t == 0 { 0.0 } else { d as f64 / t as f64 });
                }
                glib::ControlFlow::Continue
            });
            let mm = mm.clone();
            let ctx = ctx.clone();
            let b = b.clone();
            let barc = barc.clone();
            glib::spawn_future_local(async move {
                let res = gio::spawn_blocking(move || {
                    install::install(&mm, |d, t| {
                        let _ = tx.send((d, t));
                    })
                    .map(|_| mm.description())
                    .map_err(|e| e.to_string())
                })
                .await;
                tick.remove();
                barc.set_visible(false);
                match res {
                    Ok(Ok(name)) => {
                        b.set_label("Installed");
                        ctx.toast(&format!("Installed {name}"));
                        ctx.rescan();
                    }
                    Ok(Err(e)) => {
                        b.set_label("Try again");
                        b.set_sensitive(true);
                        ctx.toast(&format!("Download failed: {e}"));
                    }
                    Err(_) => {
                        b.set_label("Try again");
                        b.set_sensitive(true);
                    }
                }
            });
        });
        row.add_suffix(&btn);
        row
    }

    /// Scriptures of groups outside historic creedal Christianity, kept apart
    /// at the bottom of the list with a plain note.
    /// A downloadable book (a church fathers volume, the Quran) with its
    /// Download / Remove button, updated in place.
    fn volume_row(&self, id: &'static str, title: &str) -> adw::ActionRow {
            let installed = crate::fathers::is_installed(id);
            let row = adw::ActionRow::builder()
                .title(glib::markup_escape_text(title).as_str())
                .subtitle(if installed { "Downloaded · in the Reading room" } else { "Not downloaded" })
                .build();
            row.set_title_lines(2);
            let spin = crate::loader::new(24);
            spin.set_visible(false);
            spin.set_valign(gtk::Align::Center);
            row.add_suffix(&spin);
            let btn = gtk::Button::with_label(if installed { "Remove" } else { "Download" });
            btn.set_valign(gtk::Align::Center);
            if installed {
                btn.add_css_class("flat");
            }
            let ctx = self.ctx.clone();
            let sub = row.clone();
            btn.connect_clicked(move |b| {
                if crate::fathers::is_installed(id) {
                    if b.label().as_deref() == Some("Remove") {
                        b.set_label("Confirm remove");
                        b.add_css_class("destructive-action");
                        b.remove_css_class("flat");
                        return;
                    }
                    crate::fathers::remove(id);
                    ctx.toast("Removed the volume");
                    // update this row in place, so the list stays open
                    b.set_label("Download");
                    b.remove_css_class("destructive-action");
                    b.remove_css_class("flat");
                    sub.set_subtitle("Not downloaded");
                    ctx.refresh_one("books");
                    return;
                }
                b.set_sensitive(false);
                b.set_label("Downloading");
                spin.set_visible(true);
                let ctx = ctx.clone();
                let b = b.clone();
                let spin = spin.clone();
                let sub = sub.clone();
                glib::spawn_future_local(async move {
                    let res = gio::spawn_blocking(move || {
                        crate::fathers::download(id, &mut |_, _| {}).map_err(|e| e.to_string())
                    })
                    .await;
                    spin.set_visible(false);
                    match res {
                        Ok(Ok(())) => {
                            ctx.toast("Downloaded. It’s in the Reading room");
                            b.set_sensitive(true);
                            b.set_label("Remove");
                            b.add_css_class("flat");
                            sub.set_subtitle("Downloaded · in the Reading room");
                            ctx.refresh_one("books");
                        }
                        Ok(Err(e)) => {
                            b.set_sensitive(true);
                            b.set_label("Try again");
                            ctx.toast(&format!("Download failed: {e}"));
                        }
                        Err(_) => {
                            b.set_sensitive(true);
                            b.set_label("Try again");
                        }
                    }
                });
            });
            row.add_suffix(&btn);
            row
    }

    /// The Quran, last on Start here: for study and comparison.
    fn quran_group(&self) -> adw::PreferencesGroup {
        let g = adw::PreferencesGroup::new();
        g.set_title("The Quran");
        g.set_description(Some(&format!(
            "For study and comparison: the Arabic text with an English translation, verse by verse, read in the Reading room. {}",
            crate::fathers::QURAN_CREDIT
        )));
        g.add(&self.volume_row(crate::fathers::QURAN, "The Quran, Arabic and English"));
        g
    }

    /// "Personal copies": texts the person already owns, added from a file.
    /// The app never hosts or shares them.
    fn personal_copies(&self) -> adw::PreferencesGroup {
        let g = adw::PreferencesGroup::new();
        g.set_title("Personal copies");
        g.set_description(Some(
            "Many modern translations are still under copyright, so we can’t offer them for download. We believe Scripture should be free for everyone to read; until it is, you can add a copy you already have, as a SWORD module (.zip). It stays on this computer and is never shared. Please only add texts you have the right to use.",
        ));
        let row = adw::ActionRow::builder()
            .title("Add a SWORD module from a file")
            .subtitle("A .zip with mods.d and modules folders")
            .activatable(true)
            .build();
        row.add_prefix(&gtk::Image::from_icon_name("document-open-symbolic"));
        let btn = gtk::Button::with_label("Choose file…");
        btn.set_valign(gtk::Align::Center);
        row.add_suffix(&btn);
        row.set_activatable_widget(Some(&btn));
        let ctx = self.ctx.clone();
        btn.connect_clicked(move |b| {
            let filter = gtk::FileFilter::new();
            filter.set_name(Some("SWORD module (.zip)"));
            filter.add_pattern("*.zip");
            filter.add_pattern("*.ZIP");
            let filters = gio::ListStore::new::<gtk::FileFilter>();
            filters.append(&filter);
            let dialog = gtk::FileDialog::builder()
                .title("Add a personal copy")
                .accept_label("Add")
                .filters(&filters)
                .build();
            let win = b.root().and_downcast::<gtk::Window>();
            let ctx = ctx.clone();
            dialog.open(win.as_ref(), gio::Cancellable::NONE, move |res| {
                let Ok(file) = res else { return };
                let Some(path) = file.path() else { return };
                match install::import_zip(&path, &bible_sword::module::user_root()) {
                    Ok(names) => {
                        ctx.rescan();
                        ctx.refresh_all();
                        ctx.toast(&format!("Added {}", names.join(", ")));
                    }
                    Err(e) => ctx.toast(&format!("Couldn’t add it: {e}")),
                }
            });
        });
        g.add(&row);
        g
    }

    fn other_traditions(&self, cat: &[RemoteModule]) -> adw::PreferencesGroup {
        let g = adw::PreferencesGroup::new();
        g.set_title("Other traditions");
        g.set_description(Some(
            "Texts used by groups outside historic Christianity as defined by the ecumenical creeds, such as the Nicene Creed. They are listed for study and comparison.",
        ));
        let link_row = |title: &str, sub: &str, url: &'static str, ctx: Rc<Ctx>| {
            let row = adw::ActionRow::builder()
                .title(title)
                .subtitle(sub)
                .subtitle_lines(3)
                .build();
            let b = gtk::Button::from_icon_name("adw-external-link-symbolic");
            b.set_tooltip_text(Some(url));
            b.add_css_class("flat");
            b.set_valign(gtk::Align::Center);
            b.connect_clicked(move |_| {
                let _ = gtk::gio::AppInfo::launch_default_for_uri(
                    url,
                    None::<&gtk::gio::AppLaunchContext>,
                );
            });
            row.add_suffix(&b);
            let _ = ctx;
            row
        };
        g.add(&link_row(
            "New World Translation · Jehovah’s Witnesses",
            "Copyrighted by the Watch Tower Society and not offered by any free source, so it can’t be downloaded here. It can be read online.",
            "https://www.jw.org/en/library/bible/",
            self.ctx.clone(),
        ));
        let mormon: Vec<&RemoteModule> = cat
            .iter()
            .filter(|m| m.description().to_lowercase().contains("mormon"))
            .collect();
        if mormon.is_empty() {
            g.add(&link_row(
                "Book of Mormon · The Church of Jesus Christ of Latter-day Saints",
                "Not offered by your download sources. Latter-day Saints read it alongside the King James Bible. It can be read online.",
                "https://www.churchofjesuschrist.org/study/scriptures/bofm",
                self.ctx.clone(),
            ));
        } else {
            for m in mormon {
                g.add(&self.module_row(m));
            }
        }
        g
    }

    fn render_browse(self: &Rc<Self>) {
        while let Some(c) = self.browse_list.first_child() {
            self.browse_list.remove(&c);
        }
        let q = self.search.text().to_lowercase();
        let sel = self.lang_dd.selected() as usize;
        let lang = if sel == 0 {
            None
        } else {
            self.langs.borrow().get(sel - 1).cloned()
        };
        let cat = self.catalogue.borrow();
        let mut groups: BTreeMap<u8, Vec<&RemoteModule>> = BTreeMap::new();
        let mut seen = std::collections::HashSet::new();
        for m in cat.iter() {
            if let Some(l) = &lang {
                if &m.lang() != l {
                    continue;
                }
            }
            if !q.is_empty()
                && !m.description().to_lowercase().contains(&q)
                && !m.name().to_lowercase().contains(&q)
            {
                continue;
            }
            if !seen.insert(m.name().to_lowercase()) {
                continue;
            }
            groups
                .entry(browse_key(&m.conf))
                .or_default()
                .push(m);
        }
        // the category list, with counts for this language
        if let Some(c) = self.category.get() {
            if !groups.contains_key(&c) {
                self.category.set(None);
            }
        }
        while let Some(c) = self.cat_list.first_child() {
            self.cat_list.remove(&c);
        }
        let cat_row = |id: &str, title: &str, icon: &str, n: Option<usize>| {
            let b = gtk::Box::new(gtk::Orientation::Horizontal, 10);
            b.append(&gtk::Image::from_icon_name(icon));
            let l = gtk::Label::new(Some(title));
            l.set_xalign(0.0);
            l.set_hexpand(true);
            l.set_ellipsize(gtk::pango::EllipsizeMode::End);
            b.append(&l);
            if let Some(n) = n {
                let c = gtk::Label::new(Some(&n.to_string()));
                c.add_css_class("dim-label");
                c.add_css_class("numeric");
                b.append(&c);
            }
            let row = gtk::ListBoxRow::new();
            row.set_widget_name(id);
            row.set_child(Some(&b));
            self.cat_list.append(&row);
            row
        };
        let start = cat_row("start", "Start here", "starred-symbolic", None);
        let mut selected = start;
        for (k, mods) in &groups {
            let icon = match *k {
                0 => "bibleapp-book-symbolic",
                1 => "accessories-text-editor-symbolic",
                2 => "bibleapp-books-symbolic",
                3 => "x-office-calendar-symbolic",
                4 => "bibleapp-words-symbolic",
                6 => "image-x-generic-symbolic",
                7 => "system-users-symbolic",
                _ => "view-more-horizontal-symbolic",
            };
            let r = cat_row(&format!("k{k}"), browse_label(*k, &mods[0].conf), icon, Some(mods.len()));
            if self.category.get() == Some(*k) {
                selected = r;
            }
        }
        self.cat_list.select_row(Some(&selected));
        let category = self.category.get();

        let errs = self.errors.borrow();
        if !errs.is_empty() {
            let l = gtk::Label::new(Some(&format!(
                "Some sources didn’t respond: {}",
                errs.join("; ")
            )));
            l.set_wrap(true);
            l.set_xalign(0.0);
            l.add_css_class("dim-label");
            l.add_css_class("caption");
            self.browse_list.append(&l);
        }
        let mut shown = 0;
        if q.is_empty() && category.is_none() {
            // in "All languages", suggest an English Bible
            let want = lang.clone().unwrap_or_else(|| "en".to_string());
            if let Some(best) = best_bible(
                cat.iter().filter(|m| m.lang() == want),
                |m| &m.conf,
                |m| m.name().to_string(),
            ) {
                let g = adw::PreferencesGroup::new();
                g.set_title("Recommended Bible");
                g.set_description(Some(RECOMMEND_NOTE));
                g.add(&self.module_row(best));
                self.browse_list.append(&g);
                shown += 1;
            }
            let popular: Vec<&RemoteModule> = POPULAR
                .iter()
                .filter_map(|(name, _)| cat.iter().find(|m| m.name().eq_ignore_ascii_case(name)))
                .filter(|m| lang.as_ref().is_none_or(|l| &m.lang() == l))
                .collect();
            // one row per work, even when two module names carry the same text
            let mut dup = std::collections::HashSet::new();
            let popular: Vec<&RemoteModule> = popular
                .into_iter()
                .filter(|m| dup.insert(m.description().to_lowercase()))
                .collect();
            if !popular.is_empty() {
                let g = adw::PreferencesGroup::new();
                g.set_title("Most popular");
                g.set_description(Some("The Bibles and study works most people start with. Many well-known modern translations (NIV, ESV, NKJV, NASB) are copyrighted and aren’t offered for free download."));
                for m in popular {
                    shown += 1;
                    g.add(&self.module_row(m));
                }
                self.browse_list.append(&g);
            }
            if lang.as_deref().is_none_or(|l| l == "en") {
                let g = adw::PreferencesGroup::new();
                g.set_title("Audio Bibles");
                g.set_description(Some("Download audio a chapter or a whole book at a time with the headphones button while reading. Downloaded chapters are listed under Installed."));
                for src in bible_core::audio::SOURCES {
                    let row = adw::ActionRow::builder()
                        .title(
                            glib::markup_escape_text(&format!(
                                "{}, read by {}",
                                src.title, src.narrator
                            ))
                            .as_str(),
                        )
                        .subtitle(glib::markup_escape_text(src.credit).as_str())
                        .subtitle_lines(2)
                        .build();
                    row.add_prefix(&gtk::Image::from_icon_name("audio-headphones-symbolic"));
                    g.add(&row);
                }
                self.browse_list.append(&g);
            }
        }
        for (k, mods) in groups {
            // a search looks through every kind; otherwise only the chosen one
            if q.is_empty() && category != Some(k) {
                continue;
            }
            let g = adw::PreferencesGroup::new();
            g.set_title(browse_label(k, &mods[0].conf));
            for m in mods.into_iter().take(400) {
                shown += 1;
                g.add(&self.module_row(m));
            }
            self.browse_list.append(&g);
        }
        if q.is_empty() && category.is_none() {
            self.browse_list.append(&self.personal_copies());
            shown += 1;
        }
        if q.is_empty() && category.is_none() && lang.as_deref().is_none_or(|l| l == "en") {
            self.browse_list.append(&self.other_traditions(&cat));
        }
        if q.is_empty() && category.is_none() {
            self.browse_list.append(&self.quran_group());
        }
        if shown == 0 {
            let l = gtk::Label::new(Some(if q.is_empty() {
                "Nothing picked out in this language yet. Choose a category on the left."
            } else {
                "Nothing matches your search."
            }));
            l.set_wrap(true);
            l.add_css_class("dim-label");
            l.set_margin_top(24);
            self.browse_list.append(&l);
        }
    }
}

impl Section for LibraryView {
    fn id(&self) -> &'static str {
        "library"
    }
    fn title(&self) -> &'static str {
        "Downloads"
    }
    fn icon(&self) -> &'static str {
        "folder-download-symbolic"
    }
    fn widget(&self) -> gtk::Widget {
        self.root.clone().upcast()
    }
    fn shown(&self) {
        self.legal.set_reveal_child(self.legal_due());
        // with no Bible yet, start where the recommended one is
        if self.ctx.lib().of_kind(Kind::Bible).is_empty() {
            self.tabs.set_visible_child_name("browse");
        }
    }
    fn refresh(&self) {
        self.render_installed();
        self.legal.set_reveal_child(self.legal_due());
        let _ = &self.browse_box;
    }
}

#[cfg(test)]
mod approach_tests {
    use super::approach;
    #[test]
    fn translations_by_approach() {
        assert_eq!(approach("King James Version (1769) with Strongs Numbers"), Some("Word for word"));
        assert_eq!(approach("Young's Literal Translation (1898)"), Some("Word for word"));
        assert_eq!(approach("World English Bible"), Some("Word for word"));
        assert_eq!(approach("New International Reader's Version"), Some("Thought for thought"));
        assert_eq!(approach("NET Bible® (New English Translation)"), Some("Thought for thought"));
        assert_eq!(approach("The Message"), Some("Paraphrase"));
        assert_eq!(approach("Bible in Basic English"), None);
    }
}
