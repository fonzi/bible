//! Search the text of an installed Bible, by phrase or Strong's number.

use crate::ctx::{page, Ctx, Section};
use adw::prelude::*;
use bible_core::Ref;
use bible_sword::Kind;
use gtk::{gio, glib};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub struct Search {
    ctx: Rc<Ctx>,
    root: adw::ToolbarView,
    entry: gtk::SearchEntry,
    tips: gtk::Box,
    dd: gtk::DropDown,
    names: RefCell<Vec<String>>,
    results: gtk::ListBox,
    status: gtk::Label,
    bar: gtk::ProgressBar,
    gen: Cell<u64>,
}

impl Search {
    pub fn new(ctx: &Rc<Ctx>) -> Rc<Search> {
        let entry = gtk::SearchEntry::new();
        entry.set_placeholder_text(Some("Words, a phrase, or a Strong’s number like G25"));
        entry.set_hexpand(true);
        let dd = gtk::DropDown::from_strings(&[]);
        let top = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        top.append(&entry);
        top.append(&dd);
        let status = gtk::Label::new(Some(
            "Type and press Enter to search. A reference like “John 3:16” opens it directly.",
        ));
        status.add_css_class("dim-label");
        status.set_xalign(0.0);
        status.set_wrap(true);
        // ideas to start with, shown until there are results
        let tips = gtk::Box::new(gtk::Orientation::Vertical, 8);
        let tips_label = gtk::Label::new(Some("Try one of these"));
        tips_label.add_css_class("heading");
        tips_label.set_xalign(0.0);
        tips_label.set_margin_top(12);
        tips.append(&tips_label);
        let flow = gtk::FlowBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .max_children_per_line(4)
            .column_spacing(8)
            .row_spacing(8)
            .build();
        tips.append(&flow);
        let bar = gtk::ProgressBar::new();
        bar.set_visible(false);
        let results = gtk::ListBox::new();
        results.add_css_class("boxed-list");
        results.set_selection_mode(gtk::SelectionMode::None);
        results.set_visible(false);
        let body = gtk::Box::new(gtk::Orientation::Vertical, 12);
        body.append(&top);
        body.append(&bar);
        body.append(&status);
        body.append(&tips);
        body.append(&results);
        let clamp = adw::Clamp::builder().maximum_size(860).child(&body).build();
        clamp.set_margin_top(18);
        clamp.set_margin_bottom(24);
        clamp.set_margin_start(18);
        clamp.set_margin_end(18);
        let scroll = gtk::ScrolledWindow::builder()
            .child(&clamp)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build();
        let (root, _) = page(ctx, "Search", "", &scroll);
        let s = Rc::new(Search {
            ctx: ctx.clone(),
            root,
            entry,
            dd,
            names: RefCell::new(vec![]),
            results,
            status,
            tips: tips.clone(),
            bar,
            gen: Cell::new(0),
        });
        for (label, q, what) in [
            ("John 3:16", "John 3:16", "Opens the verse"),
            ("Psalm 23", "Psalm 23", "Opens the chapter"),
            ("“love one another”", "love one another", "A phrase"),
            ("grace", "grace", "A word"),
            ("G26 · agapē", "G26", "Greek word for love, by Strong’s number"),
            ("H2617 · ḥesed", "H2617", "Hebrew for steadfast love"),
            ("G4102 · pistis", "G4102", "Greek word for faith"),
            ("Romans 8", "Romans 8", "Opens the chapter"),
        ] {
            let b = gtk::Button::with_label(label);
            b.add_css_class("pill");
            b.set_tooltip_text(Some(what));
            let w = Rc::downgrade(&s);
            b.connect_clicked(move |_| {
                if let Some(s) = w.upgrade() {
                    s.entry.set_text(q);
                    s.run();
                }
            });
            flow.insert(&b, -1);
        }
        let w = Rc::downgrade(&s);
        s.entry.connect_activate(move |_| {
            if let Some(s) = w.upgrade() {
                s.run();
            }
        });
        let ctx2 = ctx.clone();
        s.results.connect_row_activated(move |_, row| {
            if let Some(r) = bible_core::refs::parse_osis(&row.widget_name()) {
                ctx2.open_ref(&r);
            }
        });
        let w = Rc::downgrade(&s);
        ctx.set_open_search(move |q| {
            if let Some(s) = w.upgrade() {
                s.entry.set_text(q);
                s.run();
            }
        });
        s.refresh();
        s
    }

    fn run(self: &Rc<Self>) {
        let q = self.entry.text().to_string();
        self.tips.set_visible(q.trim().is_empty());
        if q.trim().is_empty() {
            return;
        }
        // a typed reference jumps straight there
        if let Some(r) = bible_core::refs::find_refs(&q)
            .into_iter()
            .next()
            .filter(|f| f.start == 0 && f.end >= q.trim().len())
        {
            self.ctx.open_ref(&r.r);
            return;
        }
        let Some(name) = self
            .names
            .borrow()
            .get(self.dd.selected() as usize)
            .cloned()
        else {
            return;
        };
        let Some(m) = self.ctx.lib().get(&name) else {
            return;
        };
        let gen = self.gen.get() + 1;
        self.gen.set(gen);
        self.bar.set_visible(true);
        self.bar.set_fraction(0.0);
        self.status.set_text("Searching…");
        let (tx, rx) = std::sync::mpsc::channel::<f64>();
        let barc = self.bar.clone();
        let tick = glib::timeout_add_local(std::time::Duration::from_millis(80), move || {
            while let Ok(f) = rx.try_recv() {
                barc.set_fraction(f);
            }
            glib::ControlFlow::Continue
        });
        let w = Rc::downgrade(self);
        let abbr = m.abbreviation();
        glib::spawn_future_local(async move {
            let qq = q.clone();
            let hits = gio::spawn_blocking(move || {
                m.search(&qq, 500, |f| {
                    let _ = tx.send(f);
                })
            })
            .await
            .unwrap_or_default();
            tick.remove();
            let Some(s) = w.upgrade() else { return };
            if s.gen.get() != gen {
                return;
            }
            s.bar.set_visible(false);
            s.show(&q, &abbr, hits);
        });
    }

    fn show(&self, q: &str, abbr: &str, hits: Vec<(Ref, String)>) {
        while let Some(c) = self.results.first_child() {
            self.results.remove(&c);
        }
        let n = hits.len();
        self.status.set_text(&match n {
            0 => format!("No verses in the {abbr} contain “{q}”."),
            1 => format!("1 verse in the {abbr}"),
            500 => format!("Showing the first 500 verses in the {abbr}"),
            _ => format!("{n} verses in the {abbr}"),
        });
        let ql = q.to_lowercase();
        for (r, text) in hits {
            let lower = text.to_lowercase();
            let marked = match lower.find(&ql) {
                Some(i) if text.is_char_boundary(i) && text.is_char_boundary(i + ql.len()) => {
                    format!(
                        "{}<span background=\"#f6d32d\" bgalpha=\"45%\">{}</span>{}",
                        glib::markup_escape_text(&text[..i]),
                        glib::markup_escape_text(&text[i..i + ql.len()]),
                        glib::markup_escape_text(&text[i + ql.len()..])
                    )
                }
                _ => glib::markup_escape_text(&text).to_string(),
            };
            let row = adw::ActionRow::builder()
                .title(r.to_string())
                .subtitle(marked)
                .activatable(true)
                .build();
            row.set_subtitle_lines(3);
            row.add_css_class("search-hit");
            let mut osis = format!("{}.{}.{}", r.book, r.chapter, r.verse);
            if r.verse == 0 {
                osis = format!("{}.{}", r.book, r.chapter);
            }
            row.set_widget_name(&osis);
            row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
            self.results.append(&row);
        }
        self.results.set_visible(n > 0);
    }
}

impl Section for Search {
    fn id(&self) -> &'static str {
        "search"
    }
    fn title(&self) -> &'static str {
        "Search"
    }
    fn icon(&self) -> &'static str {
        "system-search-symbolic"
    }
    fn widget(&self) -> gtk::Widget {
        self.root.clone().upcast()
    }
    fn shown(&self) {
        self.entry.grab_focus();
    }
    fn refresh(&self) {
        let lib = self.ctx.lib();
        let bibles: Vec<_> = lib
            .of_kind(Kind::Bible)
            .into_iter()
            .filter(|m| m.is_readable())
            .collect();
        let labels: Vec<String> = bibles.iter().map(|m| m.abbreviation()).collect();
        let lr: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
        self.dd.set_model(Some(&gtk::StringList::new(&lr)));
        let cur = self.ctx.config.borrow().bible.clone();
        let names: Vec<String> = bibles.iter().map(|m| m.name.clone()).collect();
        if let Some(i) = names.iter().position(|n| *n == cur) {
            self.dd.set_selected(i as u32);
        }
        *self.names.borrow_mut() = names;
    }
}
