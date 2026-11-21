//! The main window: sidebar of sections on the left, the active section on the right.

use crate::ctx::{Ctx, Section};
use crate::{
    about, bookmarks, books, crossrefs, fasting, library, notes_view, plans, prefs, reader, search,
    timeline, today, wordstudy,
};
use adw::prelude::*;
use gtk::gdk;
use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    static CSS: RefCell<Option<gtk::CssProvider>> = const { RefCell::new(None) };
}

/// (Re)load the app stylesheet; text size and highlight colours come from settings.
pub fn load_css(ctx: &Ctx) {
    let cfg = ctx.config.borrow();
    let mut css = String::from(include_str!("style.css"));
    css.push_str(&format!(
        ".scripture {{ font-size: {}pt; }}\n",
        cfg.text_size
    ));
    css.push_str(&format!(
        ".commentary {{ font-size: {}pt; }}\n",
        (cfg.text_size - 3.0).max(10.0)
    ));
    css.push_str(&format!(
        ".chapter-heading {{ font-size: {}pt; }}\n",
        (cfg.text_size * 1.7).round()
    ));
    css.push_str(&format!(
        ".word-card .orig {{ font-size: {}pt; }}\n",
        (cfg.text_size + 2.0).round()
    ));
    if cfg.vintage {
        // classic typography: the type of an early printed Bible, on the
        // normal page (the background and colours stay as they are)
        let rule = "alpha(currentColor, 0.35)";
        let font =
            "\"EB Garamond\", \"Cardo\", \"Linux Libertine O\", \"Goudy Bookletter 1911\", serif";
        css.push_str(&format!(
            ".reader-page .scripture {{ font-family: {font}; font-size: {size}pt; }}\n\
             .reader-page .running-head {{ font-family: {font}; font-style: italic; font-size: {head}pt; border-top: 1px solid {rule}; border-bottom: 1px solid {rule}; padding: 4px 0; margin-bottom: 14px; }}\n\
             .reader-page .vintage-chapter {{ font-family: {font}; font-variant: small-caps; letter-spacing: 3px; font-weight: 600; font-size: {chap}pt; }}\n\
             .reader-page .fleuron {{ font-family: {font}; font-size: {orn}pt; opacity: 0.7; margin-bottom: 8px; }}\n",
            size = cfg.text_size + 1.0,
            head = (cfg.text_size + 2.0).round(),
            chap = (cfg.text_size * 1.5).round(),
            orn = (cfg.text_size * 1.2).round(),
        ));
    }
    css.push_str(&format!(
        ".note-editor, .md-editor {{ font-size: {}pt; }}\n",
        (cfg.text_size - 4.0).max(10.0)
    ));
    for c in &cfg.palette {
        css.push_str(&format!(
            ".swatch-{id} {{ background-color: {hex}; }}\n",
            id = c.id,
            hex = c.hex
        ));
    }
    for (name, hex) in [
        ("purple", "#9141ac"),
        ("white", "#deddda"),
        ("green", "#2ec27e"),
        ("red", "#e01b24"),
        ("black", "#3d3846"),
    ] {
        css.push_str(&format!(".season-{name} {{ background-color: {hex}; }}\n"));
    }
    let Some(display) = gdk::Display::default() else {
        return;
    };
    CSS.with(|cell| {
        let mut slot = cell.borrow_mut();
        if let Some(old) = slot.take() {
            gtk::style_context_remove_provider_for_display(&display, &old);
        }
        let p = gtk::CssProvider::new();
        p.load_from_string(&css);
        gtk::style_context_add_provider_for_display(
            &display,
            &p,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        *slot = Some(p);
    });
}

fn sidebar_row(s: &dyn Section) -> gtk::ListBoxRow {
    let b = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    b.append(&gtk::Image::from_icon_name(s.icon()));
    let l = gtk::Label::new(Some(&crate::i18n::t(s.title())));
    l.set_xalign(0.0);
    b.append(&l);
    let row = gtk::ListBoxRow::new();
    row.set_child(Some(&b));
    row
}

pub fn build(app: &adw::Application, present: bool) -> Rc<Ctx> {
    let ctx = Ctx::new();
    load_css(&ctx);
    prefs::apply_style(&ctx.config.borrow().style.clone());

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Bible")
        .default_width(1280)
        .default_height(820)
        .width_request(360)
        .height_request(400)
        .build();
    let _ = ctx.window.set(window.clone());

    let stack = gtk::Stack::new();
    // size to the page on show, so one wide page can't widen the window
    stack.set_hhomogeneous(false);
    stack.set_transition_type(gtk::StackTransitionType::Crossfade);
    stack.set_transition_duration(180);
    let _ = ctx.stack.set(stack.clone());

    let sections: Vec<Rc<dyn Section>> = vec![
        today::Today::new(&ctx),
        reader::Reader::new(&ctx),
        books::Books::new(&ctx),
        plans::PlansView::new(&ctx),
        timeline::TimelineView::new(&ctx),
        search::Search::new(&ctx),
        notes_view::NotesView::new(&ctx),
        bookmarks::BookmarksView::new(&ctx),
        library::LibraryView::new(&ctx),
        prefs::Prefs::new(&ctx),
        about::About::new(&ctx),
        wordstudy::WordStudy::new(&ctx),
        fasting::FastingView::new(&ctx),
        crossrefs::CrossRefs::new(&ctx),
    ];
    for s in &sections {
        stack.add_named(&s.widget(), Some(s.id()));
    }
    *ctx.sections.borrow_mut() = sections.clone();

    // sidebar
    let list = gtk::ListBox::new();
    list.add_css_class("navigation-sidebar");
    let listed: Vec<Rc<dyn Section>> = sections
        .iter()
        .filter(|s| s.in_sidebar())
        .cloned()
        .collect();
    for s in &listed {
        list.append(&sidebar_row(s.as_ref()));
    }
    let _ = ctx.sidebar.set(list.clone());
    let c = ctx.clone();
    let ids: Vec<&'static str> = listed.iter().map(|s| s.id()).collect();
    // small headings group the places: reading, exploring, your own things
    {
        let ids = ids.clone();
        list.set_header_func(move |row, _| {
            let heading = match ids.get(row.index() as usize).copied() {
                Some("timeline") => Some("Explore"),
                Some("notes") => Some("Yours"),
                Some("library") => Some(""),
                _ => None,
            };
            row.set_header(heading.map(|h| {
                if h.is_empty() {
                    let s = gtk::Separator::new(gtk::Orientation::Horizontal);
                    s.set_margin_top(6);
                    s.set_margin_bottom(6);
                    s.set_margin_start(12);
                    s.set_margin_end(12);
                    s.upcast::<gtk::Widget>()
                } else {
                    let l = gtk::Label::new(Some(&crate::i18n::t(h)));
                    l.set_xalign(0.0);
                    l.add_css_class("sidebar-heading");
                    l.upcast::<gtk::Widget>()
                }
            }).as_ref());
        });
    }
    list.connect_row_activated(move |_, row| {
        if let Some(id) = ids.get(row.index() as usize) {
            c.go(id);
        }
    });
    let prefs_list = gtk::ListBox::new();
    prefs_list.add_css_class("navigation-sidebar");
    let prefs_sec = sections
        .iter()
        .find(|s| s.id() == "prefs")
        .cloned()
        .unwrap();
    prefs_list.append(&sidebar_row(prefs_sec.as_ref()));
    let c = ctx.clone();
    prefs_list.connect_row_activated(move |l, _| {
        l.unselect_all();
        c.go("prefs");
    });
    let side_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let scroll = gtk::ScrolledWindow::builder()
        .child(&list)
        .vexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build();
    side_box.append(&scroll);
    side_box.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    side_box.append(&prefs_list);
    let side_header = adw::HeaderBar::new();
    side_header.set_show_end_title_buttons(false);
    side_header.set_title_widget(Some(&adw::WindowTitle::new("Bible", "")));
    let sb = gtk::Button::from_icon_name("system-search-symbolic");
    sb.set_tooltip_text(Some("Search"));
    let c = ctx.clone();
    sb.connect_clicked(move |_| c.go("search"));
    side_header.pack_end(&sb);
    let side = adw::ToolbarView::new();
    side.add_top_bar(&side_header);
    side.set_content(Some(&side_box));

    // "a new version is available" across the top of every page
    let update_banner = adw::Banner::new("");
    let _ = ctx.update_banner.set(update_banner.clone());
    let main = gtk::Box::new(gtk::Orientation::Vertical, 0);
    main.append(&update_banner);
    stack.set_vexpand(true);
    main.append(&stack);
    let toasts = adw::ToastOverlay::new();
    toasts.set_child(Some(&main));
    let _ = ctx.toasts.set(toasts.clone());

    let split = adw::OverlaySplitView::builder()
        .sidebar(&side)
        .content(&toasts)
        .min_sidebar_width(220.0)
        .max_sidebar_width(260.0)
        .build();
    let _ = ctx.split.set(split.clone());
    window.set_content(Some(&split));
    // look for an update a few seconds after start, at most once a day
    {
        let (c, b) = (ctx.clone(), update_banner.clone());
        gtk::glib::timeout_add_seconds_local_once(5, move || crate::update::check(&c, &b, false, None));
    }

    let bp = adw::Breakpoint::new(adw::BreakpointCondition::parse("max-width: 860sp").unwrap());
    bp.add_setter(&split, "collapsed", Some(&true.to_value()));
    window.add_breakpoint(bp);

    // keyboard shortcuts
    let c = ctx.clone();
    let keys = gtk::EventControllerKey::new();
    keys.connect_key_pressed(move |_, key, _, state| {
        if state.contains(gdk::ModifierType::CONTROL_MASK) {
            match key {
                gdk::Key::plus | gdk::Key::equal | gdk::Key::KP_Add => {
                    c.zoom(1);
                    return gtk::glib::Propagation::Stop;
                }
                gdk::Key::minus | gdk::Key::underscore | gdk::Key::KP_Subtract => {
                    c.zoom(-1);
                    return gtk::glib::Propagation::Stop;
                }
                gdk::Key::_0 | gdk::Key::KP_0 => {
                    c.zoom(0);
                    return gtk::glib::Propagation::Stop;
                }
                gdk::Key::f => {
                    c.go("search");
                    return gtk::glib::Propagation::Stop;
                }
                gdk::Key::comma => {
                    c.go("prefs");
                    return gtk::glib::Propagation::Stop;
                }
                _ => {}
            }
        }
        if key == gdk::Key::F9 {
            c.toggle_sidebar();
            return gtk::glib::Propagation::Stop;
        }
        gtk::glib::Propagation::Proceed
    });
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    window.add_controller(keys);

    let start = std::env::var("BIBLE_APP_SECTION").unwrap_or_else(|_| "today".into());
    ctx.go(&start);
    // for testing: BIBLE_APP_OPEN=John.3.16 (bible:// links arrive through GApplication's open)
    if let Some(r) = std::env::var("BIBLE_APP_OPEN")
        .ok()
        .and_then(|a| bible_core::Ref::from_uri(&a).or_else(|| bible_core::refs::parse_osis(&a)))
    {
        ctx.open_ref(&r);
    }
    // name every icon-only button for screen readers (pages do it again when shown)
    crate::ctx::label_icon_buttons(window.upcast_ref());
    if present {
        window.present();
    }
    ctx
}
