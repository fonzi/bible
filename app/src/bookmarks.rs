//! Bookmarks: verses saved with tags, filterable and exportable.

use crate::config::Bookmark;
use crate::ctx::{page, Ctx, Section};
use adw::prelude::*;
use bible_core::refs::parse_osis;
use gtk::glib;
use std::cell::RefCell;
use std::rc::Rc;

pub fn is_bookmarked(ctx: &Ctx, osis: &str) -> bool {
    ctx.config.borrow().bookmarks.iter().any(|b| b.osis == osis)
}

/// Add or remove a bookmark; returns whether it is now bookmarked.
pub fn toggle(ctx: &Ctx, osis: &str) -> bool {
    let now = {
        let mut c = ctx.config.borrow_mut();
        if c.bookmarks.iter().any(|b| b.osis == osis) {
            c.bookmarks.retain(|b| b.osis != osis);
            false
        } else {
            c.bookmarks.push(Bookmark {
                osis: osis.to_string(),
                tags: vec![],
                added: chrono::Local::now().format("%Y-%m-%d").to_string(),
            });
            true
        }
    };
    ctx.save_config();
    now
}

fn parse_tags(s: &str) -> Vec<String> {
    let mut t: Vec<String> = s
        .split([',', ' '])
        .map(|t| t.trim().trim_start_matches('#').to_lowercase())
        .filter(|t| !t.is_empty())
        .collect();
    t.dedup();
    t
}

/// The verse text from the reading Bible, plain.
fn verse_text(ctx: &Ctx, osis: &str) -> String {
    let (Some(m), Some(r)) = (ctx.bible(), parse_osis(osis)) else {
        return String::new();
    };
    m.verse_plain(&r.book, r.chapter, r.verse)
        .unwrap_or_default()
}

/// Bookmarks as Markdown, grouped by tag, with Obsidian links to the chapter notes.
pub fn to_markdown(ctx: &Ctx, marks: &[Bookmark]) -> String {
    let version = ctx.bible().map(|m| m.abbreviation()).unwrap_or_default();
    let mut out = format!(
        "# Bookmarks\n\nExported {} · {version}\n\n",
        chrono::Local::now().format("%B %-d, %Y")
    );
    let mut tags: Vec<String> = marks.iter().flat_map(|b| b.tags.clone()).collect();
    tags.sort();
    tags.dedup();
    let mut sections: Vec<(String, Vec<&Bookmark>)> = tags
        .iter()
        .map(|t| {
            (
                format!("#{t}"),
                marks.iter().filter(|b| b.tags.contains(t)).collect(),
            )
        })
        .collect();
    let untagged: Vec<&Bookmark> = marks.iter().filter(|b| b.tags.is_empty()).collect();
    if !untagged.is_empty() {
        sections.push(("Untagged".into(), untagged));
    }
    for (title, list) in sections {
        out.push_str(&format!("## {title}\n\n"));
        for b in list {
            let Some(r) = parse_osis(&b.osis) else {
                continue;
            };
            let text = verse_text(ctx, &b.osis);
            out.push_str(&format!("- **{r}** {text}\n"));
        }
        out.push('\n');
    }
    out
}

pub struct BookmarksView {
    ctx: Rc<Ctx>,
    root: adw::ToolbarView,
    filters: gtk::FlowBox,
    list: gtk::ListBox,
    empty: adw::StatusPage,
    stack: gtk::Stack,
    filter: RefCell<Option<String>>,
    me: RefCell<std::rc::Weak<BookmarksView>>,
}

impl BookmarksView {
    pub fn new(ctx: &Rc<Ctx>) -> Rc<BookmarksView> {
        let filters = gtk::FlowBox::new();
        filters.set_selection_mode(gtk::SelectionMode::None);
        filters.set_max_children_per_line(12);
        filters.set_column_spacing(6);
        filters.set_row_spacing(6);
        let list = gtk::ListBox::new();
        list.add_css_class("boxed-list");
        list.set_selection_mode(gtk::SelectionMode::None);
        let body = gtk::Box::new(gtk::Orientation::Vertical, 14);
        body.append(&filters);
        body.append(&list);
        let clamp = adw::Clamp::builder().maximum_size(820).child(&body).build();
        clamp.set_margin_top(24);
        clamp.set_margin_bottom(32);
        clamp.set_margin_start(18);
        clamp.set_margin_end(18);
        let scroll = gtk::ScrolledWindow::builder()
            .child(&clamp)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build();
        let empty = adw::StatusPage::builder()
            .icon_name("user-bookmarks-symbolic")
            .title("No bookmarks yet")
            .description("Click a verse in the Bible and choose Bookmark. Add tags here to group them, like #promises or #prayer.")
            .build();
        let stack = gtk::Stack::new();
        stack.add_named(&scroll, Some("list"));
        stack.add_named(&empty, Some("empty"));
        let (root, hb) = page(ctx, "Bookmarks", "", &stack);
        let export = gtk::Button::with_label("Export");
        export.set_tooltip_text(Some(
            "Save the bookmarks shown as a Markdown file in your notes folder",
        ));
        hb.pack_end(&export);
        let v = Rc::new(BookmarksView {
            ctx: ctx.clone(),
            root,
            filters,
            list,
            empty,
            stack,
            filter: RefCell::new(None),
            me: Default::default(),
        });
        *v.me.borrow_mut() = Rc::downgrade(&v);
        let w = Rc::downgrade(&v);
        export.connect_clicked(move |_| {
            if let Some(v) = w.upgrade() {
                v.export();
            }
        });
        v.refresh();
        v
    }

    fn shown_marks(&self) -> Vec<Bookmark> {
        let f = self.filter.borrow().clone();
        let mut marks = self.ctx.config.borrow().bookmarks.clone();
        if let Some(f) = f {
            marks.retain(|b| b.tags.contains(&f));
        }
        // Bible order
        let sys = bible_core::v11n::kjv();
        marks.sort_by_key(|b| {
            parse_osis(&b.osis)
                .map(|r| (sys.book_index(&r.book).unwrap_or(999), r.chapter, r.verse))
                .unwrap_or((999, 0, 0))
        });
        marks
    }

    fn export(&self) {
        let marks = self.shown_marks();
        if marks.is_empty() {
            return;
        }
        let md = to_markdown(&self.ctx, &marks);
        let dir = self.ctx.config.borrow().notes_dir.join("Exports");
        let name = match self.filter.borrow().as_ref() {
            Some(t) => format!("Bookmarks #{t}.md"),
            None => "Bookmarks.md".to_string(),
        };
        let path = dir.join(name);
        match std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&path, md)) {
            Ok(_) => {
                let ctx = self.ctx.clone();
                self.ctx.toast_action(
                    &format!("Saved {}", path.file_name().unwrap().to_string_lossy()),
                    "Open",
                    move || ctx.launch(&path),
                );
            }
            Err(e) => self.ctx.toast(&format!("Couldn’t export: {e}")),
        }
    }

    fn render(&self) {
        while let Some(c) = self.filters.first_child() {
            self.filters.remove(&c);
        }
        while let Some(c) = self.list.first_child() {
            self.list.remove(&c);
        }
        let all = self.ctx.config.borrow().bookmarks.clone();
        self.stack
            .set_visible_child_name(if all.is_empty() { "empty" } else { "list" });
        let _ = &self.empty;
        let mut tags: Vec<String> = all.iter().flat_map(|b| b.tags.clone()).collect();
        tags.sort();
        tags.dedup();
        let current = self.filter.borrow().clone();
        if current.as_ref().is_some_and(|c| !tags.contains(c)) {
            *self.filter.borrow_mut() = None;
        }
        let current = self.filter.borrow().clone();
        self.filters.set_visible(!tags.is_empty());
        for t in std::iter::once(None).chain(tags.iter().map(|t| Some(t.clone()))) {
            let b = gtk::ToggleButton::with_label(
                &t.as_ref()
                    .map(|t| format!("#{t}"))
                    .unwrap_or_else(|| format!("All {}", all.len())),
            );
            b.add_css_class("pill");
            b.add_css_class("tag-chip");
            b.set_active(t == current);
            let w = self.me.borrow().clone();
            b.connect_clicked(move |_| {
                if let Some(v) = w.upgrade() {
                    *v.filter.borrow_mut() = t.clone();
                    let vw = Rc::downgrade(&v);
                    glib::idle_add_local_once(move || {
                        if let Some(v) = vw.upgrade() {
                            v.render();
                        }
                    });
                }
            });
            self.filters.insert(&b, -1);
        }
        for bm in self.shown_marks() {
            let Some(r) = parse_osis(&bm.osis) else {
                continue;
            };
            let text = verse_text(&self.ctx, &bm.osis);
            let row = adw::ActionRow::builder()
                .title(glib::markup_escape_text(&r.to_string()).as_str())
                .subtitle(glib::markup_escape_text(&text).as_str())
                .subtitle_lines(3)
                .activatable(true)
                .build();
            let ctx = self.ctx.clone();
            let rr = r.clone();
            row.connect_activated(move |_| ctx.open_ref(&rr));
            let tags = gtk::Entry::new();
            tags.set_placeholder_text(Some("Add tags"));
            tags.set_text(
                &bm.tags
                    .iter()
                    .map(|t| format!("#{t}"))
                    .collect::<Vec<_>>()
                    .join(" "),
            );
            tags.set_valign(gtk::Align::Center);
            tags.set_width_chars(16);
            tags.set_tooltip_text(Some("Tags, separated by spaces, then Enter"));
            let ctx = self.ctx.clone();
            let osis = bm.osis.clone();
            let w = self.me.borrow().clone();
            let save = move |e: &gtk::Entry| {
                let t = parse_tags(&e.text());
                {
                    let mut c = ctx.config.borrow_mut();
                    if let Some(b) = c.bookmarks.iter_mut().find(|b| b.osis == osis) {
                        if b.tags == t {
                            return;
                        }
                        b.tags = t;
                    }
                }
                ctx.save_config();
                if let Some(v) = w.upgrade() {
                    let vw = Rc::downgrade(&v);
                    glib::idle_add_local_once(move || {
                        if let Some(v) = vw.upgrade() {
                            v.render();
                        }
                    });
                }
            };
            let s2 = save.clone();
            tags.connect_activate(move |e| s2(e));
            let fc = gtk::EventControllerFocus::new();
            let te = tags.clone();
            fc.connect_leave(move |_| save(&te));
            tags.add_controller(fc);
            row.add_suffix(&tags);
            let rm = gtk::Button::from_icon_name("user-trash-symbolic");
            rm.add_css_class("flat");
            rm.set_valign(gtk::Align::Center);
            rm.set_tooltip_text(Some("Remove bookmark"));
            let ctx = self.ctx.clone();
            let osis = bm.osis.clone();
            let w = self.me.borrow().clone();
            rm.connect_clicked(move |_| {
                toggle(&ctx, &osis);
                if let Some(v) = w.upgrade() {
                    v.render();
                }
            });
            row.add_suffix(&rm);
            self.list.append(&row);
        }
    }
}

impl Section for BookmarksView {
    fn id(&self) -> &'static str {
        "bookmarks"
    }
    fn title(&self) -> &'static str {
        "Bookmarks"
    }
    fn icon(&self) -> &'static str {
        "user-bookmarks-symbolic"
    }
    fn widget(&self) -> gtk::Widget {
        self.root.clone().upcast()
    }
    fn shown(&self) {
        self.render();
    }
    fn refresh(&self) {
        self.render();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags() {
        assert_eq!(
            parse_tags("#Faith, love  #hope"),
            vec!["faith", "love", "hope"]
        );
    }
}
