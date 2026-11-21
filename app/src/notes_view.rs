//! Notes: every Markdown file in your notes folder, editable in place.

use crate::ctx::{sidebar_button, Ctx, Section};
use adw::prelude::*;
use bible_core::v11n;
use gtk::{gio, glib};
use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;

pub struct NotesView {
    ctx: Rc<Ctx>,
    root: adw::ToolbarView,
    list: gtk::ListBox,
    files: RefCell<Vec<PathBuf>>,
    view: gtk::TextView,
    title: adw::WindowTitle,
    open_btn: gtk::Button,
    stack: gtk::Stack,
    current: RefCell<Option<PathBuf>>,
    loading: Cell<bool>,
    gen: Cell<u64>,
}

impl NotesView {
    pub fn new(ctx: &Rc<Ctx>) -> Rc<NotesView> {
        let list = gtk::ListBox::new();
        list.add_css_class("navigation-sidebar");
        let list_scroll = gtk::ScrolledWindow::builder()
            .child(&list)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .build();
        let folder_btn = gtk::Button::from_icon_name("folder-open-symbolic");
        folder_btn.set_tooltip_text(Some("Open the notes folder"));
        let side_header = adw::HeaderBar::new();
        side_header.set_show_end_title_buttons(false);
        side_header.pack_start(&sidebar_button(ctx));
        side_header.set_title_widget(Some(&adw::WindowTitle::new("Notes", "")));
        side_header.pack_end(&folder_btn);
        let side = adw::ToolbarView::new();
        side.add_top_bar(&side_header);
        side.set_content(Some(&list_scroll));
        side.set_width_request(280);

        let view = gtk::TextView::new();
        view.set_monospace(true);
        view.set_wrap_mode(gtk::WrapMode::WordChar);
        view.set_top_margin(18);
        view.set_bottom_margin(18);
        view.set_left_margin(24);
        view.set_right_margin(24);
        view.add_css_class("md-editor");
        let view_scroll = gtk::ScrolledWindow::builder()
            .child(&view)
            .vexpand(true)
            .hexpand(true)
            .build();
        let empty = adw::StatusPage::builder()
            .icon_name("document-edit-symbolic")
            .title("No notes yet")
            .description("Select a verse in the Bible and write in the Notes tab. Each chapter becomes a Markdown file here, ready for Obsidian too.")
            .build();
        let stack = gtk::Stack::new();
        stack.add_named(&view_scroll, Some("file"));
        stack.add_named(&empty, Some("empty"));
        let title = adw::WindowTitle::new("Notes", "");
        let open_btn = gtk::Button::with_label("Open in Bible");
        let main_header = adw::HeaderBar::new();
        main_header.set_title_widget(Some(&title));
        main_header.pack_end(&open_btn);
        let main = adw::ToolbarView::new();
        main.add_top_bar(&main_header);
        main.set_content(Some(&stack));
        main.set_hexpand(true);

        let paned = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        paned.append(&side);
        paned.append(&gtk::Separator::new(gtk::Orientation::Vertical));
        paned.append(&main);
        let root = adw::ToolbarView::new();
        root.set_content(Some(&paned));

        let n = Rc::new(NotesView {
            ctx: ctx.clone(),
            root,
            list,
            files: RefCell::new(vec![]),
            view,
            title,
            open_btn,
            stack,
            current: RefCell::new(None),
            loading: Cell::new(false),
            gen: Cell::new(0),
        });
        let w = Rc::downgrade(&n);
        n.list.connect_row_selected(move |_, row| {
            if let (Some(n), Some(row)) = (w.upgrade(), row) {
                let p = n.files.borrow().get(row.index() as usize).cloned();
                if let Some(p) = p {
                    n.open_file(&p);
                }
            }
        });
        let c = ctx.clone();
        folder_btn.connect_clicked(move |_| {
            let dir = c.config.borrow().notes_dir.clone();
            let _ = std::fs::create_dir_all(&dir);
            let uri = gio::File::for_path(&dir).uri();
            let _ = gio::AppInfo::launch_default_for_uri(&uri, None::<&gio::AppLaunchContext>);
        });
        let w = Rc::downgrade(&n);
        n.open_btn.connect_clicked(move |_| {
            if let Some(n) = w.upgrade() {
                if let Some(r) = n.current_ref() {
                    n.ctx.open_ref(&r);
                }
            }
        });
        let w = Rc::downgrade(&n);
        n.view.buffer().connect_changed(move |_| {
            if let Some(n) = w.upgrade() {
                n.schedule_save();
            }
        });
        n
    }

    /// Work out which chapter a file under Bible/ belongs to from its name ("John 03.md").
    fn current_ref(&self) -> Option<bible_core::Ref> {
        let p = self.current.borrow().clone()?;
        let stem = p.file_stem()?.to_string_lossy().to_string();
        let (name, ch) = stem.rsplit_once(' ')?;
        let ch: u16 = ch.parse().ok()?;
        let osis = bible_core::refs::canonical_book(name)?;
        v11n::system("KJVA").book(&osis)?;
        Some(bible_core::Ref::new(&osis, ch, 0))
    }

    fn open_file(&self, p: &PathBuf) {
        self.loading.set(true);
        let text = std::fs::read_to_string(p).unwrap_or_default();
        self.view.buffer().set_text(&text);
        *self.current.borrow_mut() = Some(p.clone());
        let root = self.ctx.config.borrow().notes_dir.clone();
        let rel = p
            .strip_prefix(&root)
            .map(|x| x.display().to_string())
            .unwrap_or_else(|_| p.display().to_string());
        self.title.set_title(
            &p.file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_default(),
        );
        self.title.set_subtitle(&rel);
        self.open_btn.set_visible(self.current_ref().is_some());
        self.stack.set_visible_child_name("file");
        self.loading.set(false);
    }

    fn schedule_save(self: &Rc<Self>) {
        if self.loading.get() {
            return;
        }
        let gen = self.gen.get() + 1;
        self.gen.set(gen);
        let w = Rc::downgrade(self);
        glib::timeout_add_local_once(std::time::Duration::from_millis(800), move || {
            let Some(n) = w.upgrade() else { return };
            if n.gen.get() != gen {
                return;
            }
            let cur = n.current.borrow().clone();
            if let Some(p) = cur {
                let b = n.view.buffer();
                let t = b.text(&b.start_iter(), &b.end_iter(), false);
                if let Err(e) = std::fs::write(&p, t.as_str()) {
                    n.ctx.toast(&format!("Couldn’t save: {e}"));
                }
            }
        });
    }
}

impl Section for NotesView {
    fn id(&self) -> &'static str {
        "notes"
    }
    fn title(&self) -> &'static str {
        "Notes and highlights"
    }
    fn icon(&self) -> &'static str {
        "document-edit-symbolic"
    }
    fn widget(&self) -> gtk::Widget {
        self.root.clone().upcast()
    }
    fn shown(&self) {
        self.refresh();
    }
    fn refresh(&self) {
        let files = self.ctx.notes().all_files();
        while let Some(c) = self.list.first_child() {
            self.list.remove(&c);
        }
        let root = self.ctx.config.borrow().notes_dir.clone();
        for f in &files {
            let name = f
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            let folder = f
                .parent()
                .and_then(|p| p.strip_prefix(&root).ok())
                .map(|p| p.display().to_string())
                .unwrap_or_default();
            let b = gtk::Box::new(gtk::Orientation::Vertical, 2);
            let t = gtk::Label::new(Some(&name));
            t.set_xalign(0.0);
            let s = gtk::Label::new(Some(&folder));
            s.set_xalign(0.0);
            s.add_css_class("caption");
            s.add_css_class("dim-label");
            b.append(&t);
            b.append(&s);
            b.set_margin_top(4);
            b.set_margin_bottom(4);
            self.list.append(&b);
        }
        let had = self.current.borrow().clone();
        *self.files.borrow_mut() = files.clone();
        if files.is_empty() {
            self.stack.set_visible_child_name("empty");
            self.open_btn.set_visible(false);
            self.title.set_title("Notes");
            self.title.set_subtitle("");
        } else {
            let idx = had
                .and_then(|h| files.iter().position(|f| *f == h))
                .unwrap_or(0);
            self.list
                .select_row(self.list.row_at_index(idx as i32).as_ref());
        }
    }
}
