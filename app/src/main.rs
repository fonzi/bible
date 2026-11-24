//! Bible — offline Bible study for GNOME.
#![allow(clippy::type_complexity)] // GTK callbacks are stored as boxed closures

mod about;
mod audio;
mod bookmarks;
mod books;
mod config;
mod crossrefs;
mod ctx;
mod export;
mod fasting;
mod fathers;
mod initial;
mod library;
mod i18n;
mod loader;
mod log;
mod map;
mod mpris;
mod notes_view;
mod plans;
mod prefs;
mod present;
mod reader;
mod render;
mod search;
mod sounds;
mod splash;
mod searchprovider;
mod timeline;
mod today;
mod window;
mod update;
mod wiki;
mod wordstudy;
mod xref3d;

use adw::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    static CTX: RefCell<Option<Rc<ctx::Ctx>>> = const { RefCell::new(None) };
}

/// Run something with the window's context, if the window exists.
pub fn with_ctx(f: impl FnOnce(&Rc<ctx::Ctx>)) {
    if let Some(c) = CTX.with(|c| c.borrow().clone()) {
        f(&c);
    }
}

fn ensure_window(app: &adw::Application) -> Rc<ctx::Ctx> {
    if let Some(c) = CTX.with(|c| c.borrow().clone()) {
        if let Some(w) = c.window.get() {
            w.present();
        }
        return c;
    }
    let c = window::build(app, true);
    CTX.with(|cell| *cell.borrow_mut() = Some(c.clone()));
    c
}

/// Windows has none of GNOME's fonts, so the Windows zip carries them in
/// share/fonts with a fontconfig setup in etc/fonts. Pango is pointed at it
/// before GTK starts.
#[cfg(windows)]
fn use_bundled_fonts() {
    let Some(root) = std::env::current_exe()
        .ok()
        .and_then(|e| Some(e.parent()?.parent()?.to_path_buf()))
    else {
        return;
    };
    let fonts = root.join("etc").join("fonts");
    if fonts.join("fonts.conf").exists() {
        if std::env::var_os("PANGOCAIRO_BACKEND").is_none() {
            std::env::set_var("PANGOCAIRO_BACKEND", "fc");
        }
        std::env::set_var("FONTCONFIG_PATH", &fonts);
    }
}

/// `--check-fonts`: exits with an error unless the fonts the app is designed
/// around can be found. The Windows build runs it, since we can't look.
fn check_fonts() -> gtk::glib::ExitCode {
    use pangocairo::prelude::*;
    let map = pangocairo::FontMap::default();
    let have: Vec<String> = map.list_families().iter().map(|f| f.name().to_string()).collect();
    let mut ok = true;
    for want in ["Literata", "Adwaita Sans", "Adwaita Mono"] {
        let found = have.iter().any(|h| h.eq_ignore_ascii_case(want));
        println!("{} {want}", if found { "found  " } else { "MISSING" });
        ok &= found;
    }
    println!("{} font families in all", have.len());
    if ok { gtk::glib::ExitCode::SUCCESS } else { gtk::glib::ExitCode::FAILURE }
}

fn main() -> gtk::glib::ExitCode {
    #[cfg(windows)]
    use_bundled_fonts();
    if std::env::args().any(|a| a == "--check-fonts") {
        return check_fonts();
    }
    log::init();
    // Only Linux desktops have a D-Bus session for single-instance handling;
    // elsewhere GLib warns "win32 session dbus binary not found" and the
    // like, so each launch simply runs on its own.
    let flags = if cfg!(target_os = "linux") {
        gtk::gio::ApplicationFlags::HANDLES_OPEN
    } else {
        gtk::gio::ApplicationFlags::HANDLES_OPEN | gtk::gio::ApplicationFlags::NON_UNIQUE
    };
    let app = adw::Application::builder()
        .application_id(about::APP_ID)
        .flags(flags)
        .build();
    app.connect_startup(|app| {
        searchprovider::register(app);
        // when running from a source checkout, find the icon next to the binary's data
        if let Some(display) = gtk::gdk::Display::default() {
            let theme = gtk::IconTheme::for_display(&display);
            if cfg!(debug_assertions) {
                theme.add_search_path(concat!(env!("CARGO_MANIFEST_DIR"), "/../data/icons"));
            }
        }
    });
    app.connect_activate(|app| {
        let first = CTX.with(|c| c.borrow().is_none());
        let cfg = config::Config::load();
        i18n::init(&i18n::pick(&cfg.language));
        loader::set_style(&cfg.loader);
        bible_sword::install::set_offline(cfg.offline);
        bible_step::set_offline(cfg.offline);
        if first {
            sounds::play(&cfg.startup_sound);
        }
        if first && cfg.splash {
            // the splash shows while the window is built behind it
            let a = app.clone();
            let verse = if cfg.splash_style == "verse" { splash::verse_of_the_day() } else { Default::default() };
            splash::show(app, &cfg.splash_style, verse, 1800, move || {
                with_ctx(|c| {
                    if let Some(w) = c.window.get() {
                        w.present();
                    }
                });
                let _ = &a;
            });
            let a = app.clone();
            gtk::glib::idle_add_local_once(move || {
                let c = window::build(&a, false);
                CTX.with(|cell| *cell.borrow_mut() = Some(c));
            });
            return;
        }
        ensure_window(app);
    });
    // bible://John.3.16 links, e.g. clicked in Obsidian
    app.connect_open(|app, files, _| {
        let ctx = ensure_window(app);
        for f in files {
            let uri = f.uri().to_string();
            if let Some(r) = bible_core::Ref::from_uri(&uri) {
                ctx.open_ref(&r);
            }
        }
    });
    app.run()
}
