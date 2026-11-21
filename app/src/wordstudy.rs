//! Word study: one Greek or Hebrew word through the whole Bible: what it
//! means, how it is translated, where it clusters, and every verse.

use crate::ctx::{page, Ctx, Section};
use adw::prelude::*;
use bible_core::refs::parse_osis;
use bible_core::Ref;
use bible_step::{Interlinear, Occurrence, Pack};
use gtk::{gio, glib};
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

const SHOW: usize = 200;

pub struct WordStudy {
    ctx: Rc<Ctx>,
    root: adw::ToolbarView,
    title: adw::WindowTitle,
    body: gtk::Box,
    scroll: gtk::ScrolledWindow,
    gen: Cell<u64>,
    me: RefCell<std::rc::Weak<WordStudy>>,
}

fn heading(text: &str) -> gtk::Label {
    let l = gtk::Label::new(Some(text));
    l.set_xalign(0.0);
    l.add_css_class("heading");
    l.set_margin_top(8);
    l
}

/// A row with a label, a bar and a count.
fn bar_row(label: &str, n: usize, max: usize) -> gtk::Box {
    let r = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let l = gtk::Label::new(Some(label));
    l.set_xalign(0.0);
    l.set_width_chars(16);
    l.set_max_width_chars(22);
    l.set_ellipsize(gtk::pango::EllipsizeMode::End);
    let bar = gtk::LevelBar::for_interval(0.0, max.max(1) as f64);
    bar.set_value(n as f64);
    bar.set_hexpand(true);
    bar.set_valign(gtk::Align::Center);
    bar.add_css_class("study-bar");
    let c = gtk::Label::new(Some(&n.to_string()));
    c.add_css_class("numeric");
    c.add_css_class("dim-label");
    c.set_width_chars(4);
    c.set_xalign(1.0);
    r.append(&l);
    r.append(&bar);
    r.append(&c);
    r
}

impl WordStudy {
    pub fn new(ctx: &Rc<Ctx>) -> Rc<WordStudy> {
        let body = gtk::Box::new(gtk::Orientation::Vertical, 10);
        let clamp = adw::Clamp::builder().maximum_size(820).child(&body).build();
        clamp.set_margin_top(24);
        clamp.set_margin_bottom(32);
        clamp.set_margin_start(18);
        clamp.set_margin_end(18);
        let scroll = gtk::ScrolledWindow::builder()
            .child(&clamp)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build();
        let (root, hb) = page(ctx, "Word study", "", &scroll);
        let title = adw::WindowTitle::new("Word study", "");
        hb.set_title_widget(Some(&title));
        let back = gtk::Button::from_icon_name("go-previous-symbolic");
        back.set_tooltip_text(Some("Back to the Bible"));
        let c = ctx.clone();
        back.connect_clicked(move |_| c.go("bible"));
        hb.pack_start(&back);
        let w = Rc::new(WordStudy {
            ctx: ctx.clone(),
            root,
            title,
            body,
            scroll,
            gen: Cell::new(0),
            me: Default::default(),
        });
        *w.me.borrow_mut() = Rc::downgrade(&w);
        let ww = Rc::downgrade(&w);
        ctx.set_open_word_study(move |strong| {
            if let Some(w) = ww.upgrade() {
                w.load(strong);
            }
        });
        if let Ok(s) = std::env::var("BIBLE_APP_WORD") {
            w.load(&s);
        }
        w
    }

    fn clear(&self) {
        while let Some(c) = self.body.first_child() {
            self.body.remove(&c);
        }
        self.scroll.vadjustment().set_value(0.0);
    }

    pub fn load(&self, strong: &str) {
        let strong = bible_step::plain_strong(strong);
        let pack = if strong.starts_with('H') {
            Pack::HebrewOt
        } else {
            Pack::GreekNt
        };
        self.clear();
        self.title.set_title("Word study");
        self.title.set_subtitle(&strong);
        if !bible_step::is_installed(pack) {
            let s = adw::StatusPage::builder()
                .icon_name("bibleapp-words-symbolic")
                .title("Interlinear data needed")
                .description(format!(
                    "Word study uses the {}. Download it from the Library (about 10 MB).",
                    pack.title().to_lowercase()
                ))
                .build();
            let b = gtk::Button::with_label("Open Library");
            b.add_css_class("pill");
            b.add_css_class("suggested-action");
            b.set_halign(gtk::Align::Center);
            let c = self.ctx.clone();
            b.connect_clicked(move |_| c.go("library"));
            s.set_child(Some(&b));
            self.body.append(&s);
            return;
        }
        let spinner = crate::loader::new(72);
        spinner.set_margin_top(48);
        self.body.append(&spinner);
        let gen = self.gen.get() + 1;
        self.gen.set(gen);
        let w = self.me.borrow().clone();
        let s2 = strong.clone();
        glib::spawn_future_local(async move {
            let res = gio::spawn_blocking(move || {
                let il = Interlinear::get(pack)?;
                // the lexicon keys are zero-padded ("G0025")
                let padded = format!("{}{:04}", &s2[..1], s2[1..].parse::<u32>().unwrap_or(0));
                let lex = il.lookup(&s2).or_else(|| il.lookup(&padded)).cloned();
                Some((lex, il.occurrences(&s2)))
            })
            .await;
            let Some(w) = w.upgrade() else { return };
            if w.gen.get() != gen {
                return;
            }
            w.clear();
            match res {
                Ok(Some((lex, occ))) => w.show(&strong, lex, occ),
                _ => w
                    .body
                    .append(&heading("Couldn’t read the interlinear data.")),
            }
        });
    }

    fn show(&self, strong: &str, lex: Option<bible_step::LexEntry>, occ: Vec<Occurrence>) {
        // the word
        let card = gtk::Box::new(gtk::Orientation::Vertical, 4);
        card.add_css_class("card");
        card.add_css_class("today-card");
        card.add_css_class("word-card");
        if let Some(l) = &lex {
            self.title.set_title(&l.lemma);
            let lemma = gtk::Label::new(Some(&l.lemma));
            lemma.add_css_class("orig");
            lemma.add_css_class("title-1");
            lemma.set_xalign(0.0);
            card.append(&lemma);
            let sub = gtk::Label::new(Some(&format!("{} · {strong} · {}", l.translit, l.gloss)));
            sub.set_xalign(0.0);
            sub.add_css_class("dim-label");
            card.append(&sub);
            let def = gtk::Label::new(None);
            // the pack stores definitions already as Pango markup
            if crate::render::markup_ok(&l.definition) {
                def.set_markup(&l.definition);
            } else {
                def.set_text(&l.definition);
            }
            def.set_wrap(true);
            def.set_xalign(0.0);
            def.set_margin_top(8);
            let c = self.ctx.clone();
            def.connect_activate_link(move |_, uri| {
                if let Some(r) = Ref::from_uri(uri) {
                    c.open_ref(&r);
                }
                glib::Propagation::Stop
            });
            card.append(&def);
        }
        let verses: std::collections::BTreeSet<&str> =
            occ.iter().map(|o| o.osis.as_str()).collect();
        let stat = gtk::Label::new(Some(&format!(
            "Used {} times in {} verses",
            occ.len(),
            verses.len()
        )));
        stat.set_xalign(0.0);
        stat.add_css_class("heading");
        stat.set_margin_top(8);
        card.append(&stat);
        self.body.append(&card);
        if occ.is_empty() {
            return;
        }

        // how it is translated
        let mut glosses: BTreeMap<String, usize> = BTreeMap::new();
        for o in &occ {
            // STEP marks implied words with brackets and questions with ¿
            let g: String = o
                .gloss
                .chars()
                .filter(|c| !matches!(c, '[' | ']' | '<' | '>' | '¿' | '?' | '/' | '{' | '}'))
                .collect();
            let g = g
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .to_lowercase();
            if !g.is_empty() {
                *glosses.entry(g).or_default() += 1;
            }
        }
        let mut glosses: Vec<(String, usize)> = glosses.into_iter().collect();
        glosses.sort_by_key(|g| std::cmp::Reverse(g.1));
        self.body.append(&heading("How it is glossed"));
        let max = glosses.first().map(|g| g.1).unwrap_or(1);
        for (g, n) in glosses.iter().take(10) {
            self.body.append(&bar_row(g, *n, max));
        }
        if glosses.len() > 10 {
            let more = gtk::Label::new(Some(&format!(
                "and {} other renderings",
                glosses.len() - 10
            )));
            more.set_xalign(0.0);
            more.add_css_class("dim-label");
            more.add_css_class("caption");
            self.body.append(&more);
        }

        // where it clusters
        let mut books: Vec<(String, usize)> = vec![];
        for o in &occ {
            let b = o.osis.split('.').next().unwrap_or("").to_string();
            match books.last_mut() {
                Some((x, n)) if *x == b => *n += 1,
                _ => books.push((b, 1)),
            }
        }
        self.body.append(&heading("By book"));
        let max = books.iter().map(|b| b.1).max().unwrap_or(1);
        for (b, n) in &books {
            self.body
                .append(&bar_row(&Ref::new(b, 1, 0).book_name(), *n, max));
        }

        // every verse
        self.body.append(&heading(&format!(
            "Every verse{}",
            if verses.len() > SHOW {
                format!(" (first {SHOW})")
            } else {
                String::new()
            }
        )));
        let list = gtk::ListBox::new();
        list.add_css_class("boxed-list");
        list.set_selection_mode(gtk::SelectionMode::None);
        let bible = self.ctx.bible();
        let mut seen = std::collections::HashSet::new();
        for o in occ
            .iter()
            .filter(|o| seen.insert(o.osis.clone()))
            .take(SHOW)
        {
            let Some(r) = parse_osis(&o.osis) else {
                continue;
            };
            let text = bible
                .as_ref()
                .and_then(|m| m.verse_plain(&r.book, r.chapter, r.verse))
                .unwrap_or_default();
            let row = adw::ActionRow::builder()
                .title(
                    glib::markup_escape_text(&format!("{r} · {} “{}”", o.text, o.gloss)).as_str(),
                )
                .subtitle(glib::markup_escape_text(&text).as_str())
                .subtitle_lines(2)
                .activatable(true)
                .build();
            let c = self.ctx.clone();
            row.connect_activated(move |_| c.open_ref(&r));
            list.append(&row);
        }
        self.body.append(&list);
        let search = gtk::Button::with_label("Search your Bible for this word");
        search.add_css_class("pill");
        search.set_halign(gtk::Align::Start);
        search.set_margin_top(8);
        let c = self.ctx.clone();
        let q = strong.to_string();
        search.connect_clicked(move |_| c.open_search(&q));
        self.body.append(&search);
        let credit = gtk::Label::new(Some(bible_step::CREDIT));
        credit.set_wrap(true);
        credit.set_xalign(0.0);
        credit.add_css_class("caption");
        credit.add_css_class("dim-label");
        self.body.append(&credit);
    }
}

impl Section for WordStudy {
    fn id(&self) -> &'static str {
        "wordstudy"
    }
    fn title(&self) -> &'static str {
        "Word study"
    }
    fn icon(&self) -> &'static str {
        "bibleapp-words-symbolic"
    }
    fn widget(&self) -> gtk::Widget {
        self.root.clone().upcast()
    }
    fn in_sidebar(&self) -> bool {
        false
    }
}
