//! Shared application state and navigation between sections.
//!
//! Every sidebar section implements [`Section`]; adding a feature means writing one
//! more section and registering it in `window.rs`.

use crate::config::Config;
use adw::prelude::*;
use bible_core::notes::NotesStore;
use bible_core::Ref;
use bible_sword::Library;
use std::cell::{OnceCell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

pub trait Section {
    /// Stable id used for navigation ("today", "bible", ...)
    fn id(&self) -> &'static str;
    fn title(&self) -> &'static str;
    fn icon(&self) -> &'static str;
    fn widget(&self) -> gtk::Widget;
    /// Called when the section becomes visible.
    fn shown(&self) {}
    /// Called after modules were installed or removed, or settings changed.
    fn refresh(&self) {}
    /// Whether the section is listed in the sidebar's main group.
    fn in_sidebar(&self) -> bool {
        true
    }
}

pub struct Ctx {
    pub config: RefCell<Config>,
    pub library: RefCell<Arc<Library>>,
    pub window: OnceCell<adw::ApplicationWindow>,
    pub stack: OnceCell<gtk::Stack>,
    pub split: OnceCell<adw::OverlaySplitView>,
    pub sidebar: OnceCell<gtk::ListBox>,
    pub sections: RefCell<Vec<Rc<dyn Section>>>,
    pub toasts: OnceCell<adw::ToastOverlay>,
    pub update_banner: OnceCell<adw::Banner>,
    open_ref_cb: RefCell<Option<Box<dyn Fn(&Ref)>>>,
    open_book_cb: RefCell<Option<Box<dyn Fn(&str, Option<&str>)>>>,
    open_search_cb: RefCell<Option<Box<dyn Fn(&str)>>>,
    open_time_cb: RefCell<Option<Box<dyn Fn(i32, i32, &str)>>>,
    play_audio_cb: RefCell<Option<Box<dyn Fn(&str, u16)>>>,
    word_study_cb: RefCell<Option<Box<dyn Fn(&str)>>>,
    zoom_listeners: RefCell<Vec<Box<dyn Fn(f64)>>>,
}

impl Ctx {
    pub fn new() -> Rc<Ctx> {
        Rc::new(Ctx {
            config: RefCell::new(Config::load()),
            library: RefCell::new(Arc::new(Library::scan())),
            window: OnceCell::new(),
            stack: OnceCell::new(),
            split: OnceCell::new(),
            sidebar: OnceCell::new(),
            sections: RefCell::new(Vec::new()),
            toasts: OnceCell::new(),
            update_banner: OnceCell::new(),
            open_ref_cb: RefCell::new(None),
            open_book_cb: RefCell::new(None),
            open_search_cb: RefCell::new(None),
            open_time_cb: RefCell::new(None),
            play_audio_cb: RefCell::new(None),
            word_study_cb: RefCell::new(None),
            zoom_listeners: RefCell::new(Vec::new()),
        })
    }

    /// The Bible chosen for reading (or the first readable one).
    pub fn bible(&self) -> Option<Arc<bible_sword::Module>> {
        let name = self.config.borrow().bible.clone();
        let lib = self.lib();
        lib.get(&name)
            .filter(|m| m.kind == bible_sword::Kind::Bible && m.is_readable())
            .or_else(|| {
                // nothing chosen yet: one in the reader's language, else English
                let lang = crate::i18n::pick(&self.config.borrow().language);
                let all: Vec<_> = lib
                    .of_kind(bible_sword::Kind::Bible)
                    .into_iter()
                    .filter(|m| m.is_readable())
                    .collect();
                let first_in = |l: &str| all.iter().find(|m| m.lang() == l).cloned();
                first_in(&lang)
                    .or_else(|| first_in("en"))
                    .or_else(|| all.first().cloned())
            })
    }

    pub fn lib(&self) -> Arc<Library> {
        self.library.borrow().clone()
    }

    pub fn notes(&self) -> NotesStore {
        let c = self.config.borrow();
        let mut s = NotesStore::new(c.notes_dir.clone());
        s.obsidian_links = c.obsidian_links;
        s
    }

    pub fn save_config(&self) {
        self.config.borrow().save();
    }

    pub fn toast(&self, msg: &str) {
        if let Some(t) = self.toasts.get() {
            t.add_toast(adw::Toast::new(msg));
        }
    }

    /// A toast with a button, e.g. "Exported" [Open].
    pub fn toast_action(&self, msg: &str, button: &str, f: impl Fn() + 'static) {
        if let Some(t) = self.toasts.get() {
            let toast = adw::Toast::new(msg);
            toast.set_button_label(Some(button));
            toast.connect_button_clicked(move |_| f());
            t.add_toast(toast);
        }
    }

    /// Open a file or folder in the user's default app.
    pub fn launch(&self, path: &std::path::Path) {
        let uri = gtk::gio::File::for_path(path).uri();
        let _ =
            gtk::gio::AppInfo::launch_default_for_uri(&uri, None::<&gtk::gio::AppLaunchContext>);
    }

    /// Switch to a section by id, keeping the sidebar selection in step.
    pub fn go(&self, id: &str) {
        if let Some(stack) = self.stack.get() {
            stack.set_visible_child_name(id);
            if let Some(w) = stack.visible_child() {
                label_icon_buttons(&w);
            }
        }
        if let Some(lb) = self.sidebar.get() {
            let sections = self.sections.borrow();
            let listed: Vec<_> = sections.iter().filter(|s| s.in_sidebar()).collect();
            match listed.iter().position(|s| s.id() == id) {
                Some(i) => lb.select_row(lb.row_at_index(i as i32).as_ref()),
                None => lb.unselect_all(),
            }
        }
        if let Some(split) = self.split.get() {
            if split.is_collapsed() {
                split.set_show_sidebar(false);
            }
        }
        let s = self
            .sections
            .borrow()
            .iter()
            .find(|s| s.id() == id)
            .cloned();
        if let Some(s) = s {
            s.shown();
        }
    }

    pub fn toggle_sidebar(&self) {
        if let Some(split) = self.split.get() {
            split.set_show_sidebar(!split.shows_sidebar());
        }
    }

    pub fn set_open_ref(&self, f: impl Fn(&Ref) + 'static) {
        *self.open_ref_cb.borrow_mut() = Some(Box::new(f));
    }

    /// Open a passage in the Bible reader.
    pub fn open_ref(&self, r: &Ref) {
        if let Some(f) = self.open_ref_cb.borrow().as_ref() {
            f(r);
        }
        self.go("bible");
    }

    pub fn set_open_book(&self, f: impl Fn(&str, Option<&str>) + 'static) {
        *self.open_book_cb.borrow_mut() = Some(Box::new(f));
    }

    /// Open a book, devotional or dictionary module, optionally at a section or key.
    pub fn open_book(&self, module: &str, key: Option<&str>) {
        if let Some(f) = self.open_book_cb.borrow().as_ref() {
            f(module, key);
        }
    }

    pub fn set_open_word_study(&self, f: impl Fn(&str) + 'static) {
        *self.word_study_cb.borrow_mut() = Some(Box::new(f));
    }

    /// Study a Greek or Hebrew word ("G25") through the whole Bible.
    pub fn open_word_study(&self, strong: &str) {
        self.go("wordstudy");
        if let Some(f) = self.word_study_cb.borrow().as_ref() {
            f(strong);
        }
    }

    pub fn set_play_audio(&self, f: impl Fn(&str, u16) + 'static) {
        *self.play_audio_cb.borrow_mut() = Some(Box::new(f));
    }

    /// Open a chapter in the reader and play its downloaded audio.
    pub fn play_audio(&self, book: &str, chapter: u16) {
        self.go("bible");
        if let Some(f) = self.play_audio_cb.borrow().as_ref() {
            f(book, chapter);
        }
    }

    pub fn set_open_time(&self, f: impl Fn(i32, i32, &str) + 'static) {
        *self.open_time_cb.borrow_mut() = Some(Box::new(f));
    }

    /// Show the timeline at a span of years, marked with what is being read.
    pub fn open_time(&self, start: i32, end: i32, label: &str) {
        self.go("timeline");
        if let Some(f) = self.open_time_cb.borrow().as_ref() {
            f(start, end, label);
        }
    }

    pub fn set_open_search(&self, f: impl Fn(&str) + 'static) {
        *self.open_search_cb.borrow_mut() = Some(Box::new(f));
    }

    /// Run a search (a phrase or a Strong's number) in the Search section.
    pub fn open_search(&self, q: &str) {
        self.go("search");
        if let Some(f) = self.open_search_cb.borrow().as_ref() {
            f(q);
        }
    }

    pub const DEFAULT_TEXT: f64 = 15.0;

    /// Make reading text larger (+1), smaller (-1) or reset it (0).
    pub fn zoom(&self, step: i32) {
        let size = {
            let mut c = self.config.borrow_mut();
            c.text_size = if step == 0 {
                Ctx::DEFAULT_TEXT
            } else {
                (c.text_size + step as f64).clamp(10.0, 36.0)
            };
            c.text_size
        };
        self.save_config();
        crate::window::load_css(self);
        for f in self.zoom_listeners.borrow().iter() {
            f(size);
        }
    }

    pub fn on_zoom(&self, f: impl Fn(f64) + 'static) {
        self.zoom_listeners.borrow_mut().push(Box::new(f));
    }

    /// Show or hide the main sidebar, returning whether it was shown.
    pub fn set_sidebar(&self, show: bool) -> bool {
        match self.split.get() {
            Some(s) => {
                let was = s.shows_sidebar();
                s.set_show_sidebar(show);
                was
            }
            None => false,
        }
    }

    /// Re-scan installed modules and tell every section.
    pub fn rescan(&self) {
        *self.library.borrow_mut() = Arc::new(Library::scan());
        self.refresh_all();
    }

    /// Refresh every section but one (the one making the change).
    pub fn refresh_except(&self, id: &str) {
        let sections: Vec<_> = self.sections.borrow().clone();
        for s in sections.iter().filter(|s| s.id() != id) {
            s.refresh();
        }
    }

    pub fn refresh_all(&self) {
        let sections: Vec<_> = self.sections.borrow().clone();
        for s in sections {
            s.refresh();
        }
    }
}

/// A header bar button that shows or hides the main sidebar.
pub fn sidebar_button(ctx: &Rc<Ctx>) -> gtk::Button {
    let b = gtk::Button::from_icon_name("sidebar-show-symbolic");
    b.set_tooltip_text(Some("Show or hide the sidebar"));
    let c = ctx.clone();
    b.connect_clicked(move |_| c.toggle_sidebar());
    b
}

/// A header-bar button with − 100% + to change the reading text size.
pub fn zoom_button(ctx: &Rc<Ctx>) -> gtk::MenuButton {
    let mb = gtk::MenuButton::new();
    mb.set_icon_name("zoom-in-symbolic");
    mb.set_tooltip_text(Some("Text size (Ctrl + and Ctrl −)"));
    let out_b = gtk::Button::from_icon_name("zoom-out-symbolic");
    out_b.set_tooltip_text(Some("Smaller text"));
    let in_b = gtk::Button::from_icon_name("zoom-in-symbolic");
    in_b.set_tooltip_text(Some("Larger text"));
    let pct = |s: f64| format!("{}%", (s / Ctx::DEFAULT_TEXT * 100.0).round() as i32);
    let reset = gtk::Button::with_label(&pct(ctx.config.borrow().text_size));
    reset.set_tooltip_text(Some("Reset to normal size"));
    reset.set_hexpand(true);
    for b in [&out_b, &reset, &in_b] {
        b.add_css_class("flat");
    }
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    row.add_css_class("zoom-row");
    row.append(&out_b);
    row.append(&reset);
    row.append(&in_b);
    let hint = gtk::Label::new(Some("Ctrl + bigger · Ctrl − smaller · Ctrl 0 reset"));
    hint.add_css_class("caption");
    hint.add_css_class("dim-label");
    let body = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let title = gtk::Label::new(Some("Text size"));
    title.add_css_class("heading");
    body.append(&title);
    body.append(&row);
    body.append(&hint);
    body.set_size_request(240, -1);
    let pop = gtk::Popover::new();
    pop.set_child(Some(&body));
    mb.set_popover(Some(&pop));
    let c = ctx.clone();
    out_b.connect_clicked(move |_| c.zoom(-1));
    let c = ctx.clone();
    in_b.connect_clicked(move |_| c.zoom(1));
    let c = ctx.clone();
    reset.connect_clicked(move |_| c.zoom(0));
    let r = reset.downgrade();
    ctx.on_zoom(move |s| {
        if let Some(r) = r.upgrade() {
            r.set_label(&pct(s));
        }
    });
    mb
}

/// Standard full-window section page: header bar on top, content below.
pub fn page(
    ctx: &Rc<Ctx>,
    title: &str,
    subtitle: &str,
    content: &impl IsA<gtk::Widget>,
) -> (adw::ToolbarView, adw::HeaderBar) {
    let tv = adw::ToolbarView::new();
    let hb = adw::HeaderBar::new();
    hb.pack_start(&sidebar_button(ctx));
    hb.set_title_widget(Some(&adw::WindowTitle::new(&crate::i18n::t(title), subtitle)));
    tv.add_top_bar(&hb);
    tv.set_content(Some(content));
    (tv, hb)
}

/// Screen readers name a button by its label; an icon-only button has none,
/// so give each one its tooltip as its accessible name.
pub fn label_icon_buttons(w: &gtk::Widget) {

    let is_button = w.is::<gtk::Button>() || w.is::<gtk::MenuButton>();
    if is_button {
        let has_text = {
            let mut found = false;
            let mut stack = vec![w.clone()];
            while let Some(x) = stack.pop() {
                if let Some(l) = x.downcast_ref::<gtk::Label>() {
                    if !l.text().trim().is_empty() {
                        found = true;
                        break;
                    }
                }
                let mut c = x.first_child();
                while let Some(ch) = c {
                    c = ch.next_sibling();
                    stack.push(ch);
                }
            }
            found
        };
        if !has_text {
            match w.tooltip_text() {
                Some(t) => w.update_property(&[gtk::accessible::Property::Label(&t)]),
                // BIBLE_APP_A11Y_REPORT=1 lists buttons that still have no name
                None if std::env::var_os("BIBLE_APP_A11Y_REPORT").is_some() => {
                    let icon = w
                        .downcast_ref::<gtk::Button>()
                        .and_then(|b| b.icon_name())
                        .or_else(|| w.downcast_ref::<gtk::MenuButton>().and_then(|b| b.icon_name()))
                        .map(|s| s.to_string())
                        .unwrap_or_default();
                    let mut chain = Vec::new();
                    let mut p = w.parent();
                    while let Some(x) = p {
                        chain.push(x.type_().name().to_string());
                        p = x.parent();
                        if chain.len() > 3 {
                            break;
                        }
                    }
                    eprintln!("A11Y unnamed button: {} icon={icon} in {}", w.type_().name(), chain.join("<"));
                }
                None => {}
            }
        }
    }
    // the toolkit names the insides of its own controls
    let own = ["GtkWindowControls", "AdwBackButton", "GtkSpinButton", "GtkMenuButton", "AdwBanner"];
    if own.contains(&w.type_().name()) {
        return;
    }
    let mut c = w.first_child();
    while let Some(ch) = c {
        c = ch.next_sibling();
        label_icon_buttons(&ch);
    }
}
