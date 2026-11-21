//! Books, devotionals and dictionaries: a plain list first, then a reader that
//! works like the Bible reader (title and section pickers, previous and next).

use crate::ctx::{sidebar_button, zoom_button, Ctx, Section};
use crate::render::long_markup;
use adw::prelude::*;
use bible_core::Ref;
use bible_sword::{Kind, Module};
use gtk::glib;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

#[derive(Clone)]
enum Pos {
    Book(usize),
    Key(String),
}

pub struct Books {
    ctx: Rc<Ctx>,
    root: gtk::Stack,
    list: adw::PreferencesPage,
    groups: RefCell<Vec<adw::PreferencesGroup>>,
    // reader
    title: adw::WindowTitle,
    section_btn: gtk::MenuButton,
    toc: gtk::ListBox,
    toc_search: gtk::SearchEntry,
    text: gtk::Label,
    heading: gtk::Label,
    scroller: gtk::ScrolledWindow,
    info: gtk::Label,
    current: RefCell<Option<Arc<Module>>>,
    pos: RefCell<Option<Pos>>,
    toc_items: RefCell<Vec<Pos>>,
}

fn month_name(m: u32) -> &'static str {
    [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ]
    .get(m as usize - 1)
    .copied()
    .unwrap_or("")
}

/// "11.16" -> "November 16"
fn date_label(key: &str) -> String {
    match key.split_once('.') {
        Some((m, d)) => match (m.parse::<u32>(), d.parse::<u32>()) {
            (Ok(m), Ok(d)) if (1..=12).contains(&m) => format!("{} {d}", month_name(m)),
            _ => key.to_string(),
        },
        None => key.to_string(),
    }
}

impl Books {
    pub fn new(ctx: &Rc<Ctx>) -> Rc<Books> {
        // list page
        let list = adw::PreferencesPage::new();
        let list_tv = adw::ToolbarView::new();
        let lh = adw::HeaderBar::new();
        lh.pack_start(&sidebar_button(ctx));
        lh.set_title_widget(Some(&adw::WindowTitle::new("Books and devotionals", "")));
        let more = gtk::Button::from_icon_name("list-add-symbolic");
        more.set_tooltip_text(Some("Get more from the Library"));
        let c = ctx.clone();
        more.connect_clicked(move |_| c.go("library"));
        lh.pack_end(&more);
        list_tv.add_top_bar(&lh);
        list_tv.set_content(Some(&list));

        // reader page
        let back = gtk::Button::from_icon_name("go-previous-symbolic");
        back.set_tooltip_text(Some("All books"));
        let prev = gtk::Button::from_icon_name("go-up-symbolic");
        prev.set_tooltip_text(Some("Previous section"));
        let next = gtk::Button::from_icon_name("go-down-symbolic");
        next.set_tooltip_text(Some("Next section"));
        let section_btn = gtk::MenuButton::new();
        section_btn.set_always_show_arrow(true);
        let toc = gtk::ListBox::new();
        toc.add_css_class("navigation-sidebar");
        let toc_search = gtk::SearchEntry::new();
        toc_search.set_placeholder_text(Some("Find a section or word"));
        let toc_scroll = gtk::ScrolledWindow::builder()
            .child(&toc)
            .min_content_height(360)
            .max_content_height(480)
            .propagate_natural_height(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build();
        let pop_box = gtk::Box::new(gtk::Orientation::Vertical, 8);
        pop_box.append(&toc_search);
        pop_box.append(&toc_scroll);
        pop_box.set_size_request(340, -1);
        let pop = gtk::Popover::new();
        pop.set_child(Some(&pop_box));
        section_btn.set_popover(Some(&pop));
        let title = adw::WindowTitle::new("", "");
        let rh = adw::HeaderBar::new();
        rh.pack_start(&sidebar_button(ctx));
        rh.pack_start(&back);
        rh.pack_start(&section_btn);
        rh.pack_start(&prev);
        rh.pack_start(&next);
        rh.set_title_widget(Some(&title));
        rh.pack_end(&zoom_button(ctx));
        let heading = gtk::Label::new(None);
        heading.add_css_class("chapter-heading");
        heading.set_xalign(0.0);
        heading.set_wrap(true);
        let text = gtk::Label::new(None);
        text.set_wrap(true);
        text.set_xalign(0.0);
        text.set_selectable(true);
        text.set_use_markup(true);
        text.add_css_class("scripture");
        text.add_css_class("book-text");
        text.set_focus_on_click(false);
        text.set_can_focus(false);
        let info = gtk::Label::new(None);
        info.set_wrap(true);
        info.set_xalign(0.0);
        info.add_css_class("dim-label");
        info.add_css_class("caption");
        let body = gtk::Box::new(gtk::Orientation::Vertical, 12);
        body.append(&heading);
        body.append(&text);
        body.append(&info);
        let clamp = adw::Clamp::builder().maximum_size(720).child(&body).build();
        clamp.set_margin_top(24);
        clamp.set_margin_bottom(48);
        clamp.set_margin_start(24);
        clamp.set_margin_end(24);
        let scroller = gtk::ScrolledWindow::builder()
            .child(&clamp)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .build();
        let read_tv = adw::ToolbarView::new();
        read_tv.add_top_bar(&rh);
        read_tv.set_content(Some(&scroller));

        let root = gtk::Stack::new();
        root.set_transition_type(gtk::StackTransitionType::SlideLeftRight);
        root.add_named(&list_tv, Some("list"));
        root.add_named(&read_tv, Some("read"));

        let b = Rc::new(Books {
            ctx: ctx.clone(),
            root,
            list,
            groups: RefCell::new(vec![]),
            title,
            section_btn,
            toc,
            toc_search,
            text,
            heading,
            scroller,
            info,
            current: RefCell::new(None),
            pos: RefCell::new(None),
            toc_items: RefCell::new(vec![]),
        });
        let w = Rc::downgrade(&b);
        back.connect_clicked(move |_| {
            if let Some(b) = w.upgrade() {
                b.root.set_visible_child_name("list");
            }
        });
        let w = Rc::downgrade(&b);
        prev.connect_clicked(move |_| {
            if let Some(b) = w.upgrade() {
                b.step(false);
            }
        });
        let w = Rc::downgrade(&b);
        next.connect_clicked(move |_| {
            if let Some(b) = w.upgrade() {
                b.step(true);
            }
        });
        let w = Rc::downgrade(&b);
        b.toc.connect_row_activated(move |_, row| {
            if let Some(b) = w.upgrade() {
                let p = b.toc_items.borrow().get(row.index() as usize).cloned();
                if let Some(p) = p {
                    b.section_btn.popdown();
                    b.show(p);
                }
            }
        });
        let w = Rc::downgrade(&b);
        b.toc_search.connect_search_changed(move |_| {
            if let Some(b) = w.upgrade() {
                b.fill_toc();
            }
        });
        let w = Rc::downgrade(&b);
        b.toc_search.connect_activate(move |_| {
            if let Some(b) = w.upgrade() {
                let first = b.toc_items.borrow().first().cloned();
                if let Some(p) = first {
                    b.section_btn.popdown();
                    b.show(p);
                }
            }
        });
        let c = ctx.clone();
        b.text.connect_activate_link(move |_, uri| {
            if let Some(r) = Ref::from_uri(uri) {
                c.open_ref(&r);
            }
            glib::Propagation::Stop
        });
        let w = Rc::downgrade(&b);
        ctx.set_open_book(move |name, key| {
            if let Some(b) = w.upgrade() {
                b.open(name, key);
            }
        });
        b.refresh();
        b
    }

    fn modules(&self) -> Vec<Arc<Module>> {
        let lib = self.ctx.lib();
        lib.modules
            .iter()
            .filter(|m| {
                matches!(m.kind, Kind::Book | Kind::Devotional | Kind::Lexicon) && m.is_readable()
            })
            .cloned()
            .collect()
    }

    /// Open a module, optionally at a section path or key ("today" for devotionals).
    pub fn open(&self, name: &str, key: Option<&str>) {
        let Some(m) = self.ctx.lib().get(name) else {
            return;
        };
        *self.current.borrow_mut() = Some(m.clone());
        self.title.set_title(&m.description());
        self.title.set_subtitle(match m.kind {
            Kind::Devotional => "Devotional",
            Kind::Lexicon => "Dictionary",
            _ => "Book",
        });
        self.toc_search.set_text("");
        self.fill_toc();
        let pos = if let Some(gb) = m.genbook() {
            let idx = key
                .and_then(|k| gb.find(k))
                .or_else(|| gb.next_with_text(0, true))
                .unwrap_or(0);
            Pos::Book(idx)
        } else if let Some(ld) = m.ld() {
            let today = chrono::Local::now();
            let k = match key {
                Some("today") | None if m.kind == Kind::Devotional => format!(
                    "{:02}.{:02}",
                    chrono::Datelike::month(&today),
                    chrono::Datelike::day(&today)
                ),
                Some(k) => k.to_string(),
                None => ld.keys().next().unwrap_or("").to_string(),
            };
            Pos::Key(k)
        } else {
            return;
        };
        self.show(pos);
        self.root.set_visible_child_name("read");
        self.ctx.go("books");
    }

    fn fill_toc(&self) {
        while let Some(c) = self.toc.first_child() {
            self.toc.remove(&c);
        }
        let mut items = Vec::new();
        let Some(m) = self.current.borrow().clone() else {
            return;
        };
        let q = self.toc_search.text().to_lowercase();
        if let Some(gb) = m.genbook() {
            for (i, n) in gb.nodes.iter().enumerate().skip(1) {
                if !q.is_empty() && !n.name.to_lowercase().contains(&q) {
                    continue;
                }
                let l = gtk::Label::new(Some(&n.name));
                l.set_xalign(0.0);
                l.set_ellipsize(gtk::pango::EllipsizeMode::End);
                l.set_margin_start(((n.depth.saturating_sub(1)) * 16) as i32);
                if n.depth <= 1 {
                    l.add_css_class("heading");
                }
                self.toc.append(&l);
                items.push(Pos::Book(i));
            }
        } else if let Some(ld) = m.ld() {
            let lang = m.strongs_lang();
            // "g3439" / "3439" should find "03439"
            let q = match lang {
                Some(_) => {
                    let digits: String = q
                        .trim_start_matches(['g', 'h'])
                        .chars()
                        .filter(|c| c.is_ascii_digit())
                        .collect();
                    if !digits.is_empty()
                        && q.trim_start_matches(['g', 'h'])
                            .chars()
                            .all(|c| c.is_ascii_digit())
                    {
                        digits.trim_start_matches('0').to_string()
                    } else {
                        q.clone()
                    }
                }
                None => q.clone(),
            };
            for k in ld.keys() {
                let label = match (m.kind == Kind::Devotional, lang) {
                    (true, _) => date_label(k),
                    (false, Some(l)) if k.chars().all(|c| c.is_ascii_digit()) => {
                        format!("{l}{}", k.trim_start_matches('0'))
                    }
                    _ => k.to_string(),
                };
                let q_match = |x: &str| {
                    x.to_lowercase()
                        .trim_start_matches(['g', 'h'])
                        .trim_start_matches('0')
                        .starts_with(&q)
                };
                if !q.is_empty()
                    && !label.to_lowercase().contains(&q)
                    && !k.to_lowercase().contains(&q)
                    && !(lang.is_some() && q_match(k))
                {
                    continue;
                }
                let l = gtk::Label::new(Some(&label));
                l.set_xalign(0.0);
                self.toc.append(&l);
                items.push(Pos::Key(k.to_string()));
                if items.len() >= 400 {
                    break;
                }
            }
        }
        *self.toc_items.borrow_mut() = items;
    }

    fn show(&self, p: Pos) {
        let Some(m) = self.current.borrow().clone() else {
            return;
        };
        let dark = adw::StyleManager::default().is_dark();
        match &p {
            Pos::Book(i) => {
                let Some(gb) = m.genbook() else { return };
                let n = &gb.nodes[*i];
                let crumbs: Vec<String> = {
                    let mut v = Vec::new();
                    let mut cur = n.parent;
                    while let Some(c) = cur {
                        if !gb.nodes[c].name.is_empty() {
                            v.push(gb.nodes[c].name.clone());
                        }
                        cur = gb.nodes[c].parent;
                    }
                    v.reverse();
                    v
                };
                self.section_btn.set_label(&n.name);
                self.heading.set_text(&n.name);
                let body = gb
                    .text(*i)
                    .map(|raw| long_markup(&m.render(&raw), dark))
                    .unwrap_or_else(|| {
                        let kids: Vec<String> = n
                            .children
                            .iter()
                            .map(|c| format!("• {}", glib::markup_escape_text(&gb.nodes[*c].name)))
                            .collect();
                        if kids.is_empty() {
                            "<i>No text in this section.</i>".into()
                        } else {
                            kids.join("\n")
                        }
                    });
                self.text.set_markup(&body);
                self.info.set_text(&if crumbs.is_empty() {
                    m.license()
                } else {
                    format!("{} · {}", crumbs.join(" › "), m.license())
                });
            }
            Pos::Key(k) => {
                let Some(ld) = m.ld() else { return };
                let label = match (m.kind == Kind::Devotional, m.strongs_lang()) {
                    (true, _) => date_label(k),
                    (false, Some(l)) if k.chars().all(|c| c.is_ascii_digit()) => {
                        format!("{l}{}", k.trim_start_matches('0'))
                    }
                    _ => k.clone(),
                };
                self.section_btn.set_label(&label);
                self.heading.set_text(&label);
                match ld.get(k) {
                    Some((_, raw)) => self.text.set_markup(&long_markup(&m.render(&raw), dark)),
                    None => self.text.set_markup("<i>No entry for this key.</i>"),
                }
                self.info.set_text(&m.license());
            }
        }
        *self.pos.borrow_mut() = Some(p);
        // a selectable label selects everything when it gets focus; keep the page clean
        self.text.select_region(0, 0);
        self.scroller.vadjustment().set_value(0.0);
    }

    fn step(&self, forward: bool) {
        let Some(m) = self.current.borrow().clone() else {
            return;
        };
        let Some(p) = self.pos.borrow().clone() else {
            return;
        };
        let next = match p {
            Pos::Book(i) => m
                .genbook()
                .and_then(|gb| gb.next_with_text(i, forward))
                .map(Pos::Book),
            Pos::Key(k) => m.ld().and_then(|ld| {
                let keys: Vec<&str> = ld.keys().collect();
                let i = keys.iter().position(|x| x.eq_ignore_ascii_case(&k))?;
                let j = if forward { i + 1 } else { i.checked_sub(1)? };
                keys.get(j).map(|s| Pos::Key(s.to_string()))
            }),
        };
        if let Some(n) = next {
            self.show(n);
        }
    }
}

impl Section for Books {
    fn id(&self) -> &'static str {
        "books"
    }
    fn title(&self) -> &'static str {
        "Books and devotionals"
    }
    fn icon(&self) -> &'static str {
        "bibleapp-books-symbolic"
    }
    fn widget(&self) -> gtk::Widget {
        self.root.clone().upcast()
    }
    fn refresh(&self) {
        for g in self.groups.borrow_mut().drain(..) {
            self.list.remove(&g);
        }
        let mods = self.modules();
        let mut groups = Vec::new();
        if mods.is_empty() {
            let g = adw::PreferencesGroup::new();
            let row = adw::ActionRow::builder()
                .title("No books yet")
                .subtitle("Calvin’s Institutes, the Westminster Confession, Spurgeon’s Morning and Evening, Strong’s dictionaries and more are in the Library.")
                .activatable(true)
                .build();
            row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
            let c = self.ctx.clone();
            row.connect_activated(move |_| c.go("library"));
            g.add(&row);
            self.list.add(&g);
            groups.push(g);
        }
        for (kind, title) in [
            (Kind::Book, "Books"),
            (Kind::Devotional, "Devotionals"),
            (Kind::Lexicon, "Dictionaries and lexicons"),
        ] {
            let ms: Vec<_> = mods.iter().filter(|m| m.kind == kind).collect();
            if ms.is_empty() {
                continue;
            }
            let g = adw::PreferencesGroup::builder().title(title).build();
            for m in ms {
                let sub = if kind == Kind::Devotional {
                    "Opens to today’s entry".to_string()
                } else {
                    format!("{} · {}", m.abbreviation(), m.license())
                };
                let row = adw::ActionRow::builder()
                    .title(glib::markup_escape_text(&m.description()).as_str())
                    .subtitle(glib::markup_escape_text(&sub).as_str())
                    .activatable(true)
                    .build();
                row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
                let c = self.ctx.clone();
                let name = m.name.clone();
                row.connect_activated(move |_| {
                    c.open_book(
                        &name,
                        if kind == Kind::Devotional {
                            Some("today")
                        } else {
                            None
                        },
                    )
                });
                g.add(&row);
            }
            self.list.add(&g);
            groups.push(g);
        }
        *self.groups.borrow_mut() = groups;
        // the open module may have been removed
        let cur = self.current.borrow().as_ref().map(|m| m.name.clone());
        if let Some(n) = cur {
            if self.ctx.lib().get(&n).is_none() {
                self.root.set_visible_child_name("list");
            }
        }
    }
}
