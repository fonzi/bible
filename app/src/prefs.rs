//! Preferences, as a full-window page (no dialogs).

use crate::config::SourceCfg;
use crate::ctx::{page, Ctx, Section};
use adw::prelude::*;
use bible_core::churchyear::Tradition;
use std::cell::RefCell;
use std::rc::Rc;

pub struct Prefs {
    ctx: Rc<Ctx>,
    root: adw::ToolbarView,
    page: adw::PreferencesPage,
    sources_group: adw::PreferencesGroup,
    source_rows: RefCell<Vec<gtk::Widget>>,
    folder_row: adw::ActionRow,
}

pub fn apply_style(style: &str) {
    adw::StyleManager::default().set_color_scheme(match style {
        "light" => adw::ColorScheme::ForceLight,
        "dark" => adw::ColorScheme::ForceDark,
        _ => adw::ColorScheme::Default,
    });
}

impl Prefs {
    pub fn new(ctx: &Rc<Ctx>) -> Rc<Prefs> {
        let page = adw::PreferencesPage::new();
        let (root, _) = page_wrap(ctx, &page);
        let cfg = ctx.config.borrow().clone();

        // Appearance and reading
        // the internet kill switch, first so it's easy to find
        let g = adw::PreferencesGroup::new();
        let off = adw::SwitchRow::builder()
            .title(crate::i18n::t("Work offline"))
            .subtitle(crate::i18n::t("Never connect to the internet: no downloads, update checks or Wikipedia. Everything you've downloaded keeps working"))
            .active(cfg.offline)
            .build();
        off.add_prefix(&gtk::Image::from_icon_name("network-offline-symbolic"));
        let c = ctx.clone();
        off.connect_active_notify(move |r| {
            let on = r.is_active();
            bible_sword::install::set_offline(on);
            bible_step::set_offline(on);
            c.config.borrow_mut().offline = on;
            c.save_config();
        });
        g.add(&off);
        // interface language
        let mut names = vec![crate::i18n::t("Follow the system"), "English".to_string()];
        names.extend(crate::i18n::CATALOGS.iter().map(|c| c.1.to_string()));
        let name_refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let lang = adw::ComboRow::builder()
            .title(crate::i18n::t("Language"))
            .subtitle(crate::i18n::t("Takes effect the next time the app opens"))
            .model(&gtk::StringList::new(&name_refs))
            .build();
        lang.add_prefix(&gtk::Image::from_icon_name("preferences-desktop-locale-symbolic"));
        let current = match cfg.language.as_str() {
            "" | "system" => 0,
            "en" => 1,
            code => crate::i18n::CATALOGS.iter().position(|c| c.0 == code).map(|i| i + 2).unwrap_or(0),
        };
        lang.set_selected(current as u32);
        let c = ctx.clone();
        lang.connect_selected_notify(move |r| {
            let code = match r.selected() {
                0 => String::new(),
                1 => "en".to_string(),
                i => crate::i18n::CATALOGS.get(i as usize - 2).map(|c| c.0.to_string()).unwrap_or_default(),
            };
            c.config.borrow_mut().language = code;
            c.save_config();
        });
        g.add(&lang);
        page.add(&g);

        let g = adw::PreferencesGroup::builder().title(crate::i18n::t("Reading")).build();
        let style = adw::ComboRow::builder()
            .title(crate::i18n::t("Style"))
            .model(&gtk::StringList::new(&["Follow system", "Light", "Dark"]))
            .build();
        style.set_selected(match cfg.style.as_str() {
            "light" => 1,
            "dark" => 2,
            _ => 0,
        });
        let c = ctx.clone();
        style.connect_selected_notify(move |r| {
            let s = ["system", "light", "dark"][r.selected() as usize];
            c.config.borrow_mut().style = s.into();
            c.save_config();
            apply_style(s);
            c.refresh_all();
        });
        g.add(&style);
        let size = adw::SpinRow::with_range(10.0, 36.0, 1.0);
        size.set_title("Text size");
        size.set_subtitle("Also in the reader’s header, or press Ctrl + and Ctrl −");
        size.set_value(cfg.text_size);
        let c = ctx.clone();
        size.connect_value_notify(move |r| {
            if (c.config.borrow().text_size - r.value()).abs() > 0.1 {
                c.zoom(0);
                c.zoom((r.value() - crate::ctx::Ctx::DEFAULT_TEXT) as i32);
            }
        });
        let sw = size.downgrade();
        ctx.on_zoom(move |v| {
            if let Some(s) = sw.upgrade() {
                if (s.value() - v).abs() > 0.1 {
                    s.set_value(v);
                }
            }
        });
        g.add(&size);
        let switch = |title: &str, sub: &str, val: bool, f: Box<dyn Fn(&Ctx, bool)>| {
            let r = adw::SwitchRow::builder()
                .title(title)
                .subtitle(sub)
                .active(val)
                .build();
            let c = ctx.clone();
            r.connect_active_notify(move |r| {
                f(&c, r.is_active());
                c.save_config();
                c.refresh_all();
            });
            r
        };
        g.add(&switch(
            "Verse numbers",
            "",
            cfg.verse_numbers,
            Box::new(|c, v| c.config.borrow_mut().verse_numbers = v),
        ));
        g.add(&switch(
            &crate::i18n::t("Words of Christ in red"),
            &crate::i18n::t("When the version marks them"),
            cfg.red_letters,
            Box::new(|c, v| c.config.borrow_mut().red_letters = v),
        ));
        g.add(&switch(
            &crate::i18n::t("Splash screen"),
            &crate::i18n::t("A short animation while the Bible opens"),
            cfg.splash,
            Box::new(|c, v| c.config.borrow_mut().splash = v),
        ));
        let names: Vec<&str> = crate::splash::STYLES.iter().map(|s| s.1).collect();
        let st = adw::ComboRow::builder().title(crate::i18n::t("Splash style")).model(&gtk::StringList::new(&names)).build();
        st.set_selected(crate::splash::STYLES.iter().position(|s| s.0 == cfg.splash_style).unwrap_or(0) as u32);
        let c = ctx.clone();
        st.connect_selected_notify(move |r| {
            c.config.borrow_mut().splash_style = crate::splash::STYLES[r.selected() as usize].0.to_string();
            c.save_config();
        });
        g.add(&st);
        let lnames: Vec<&str> = crate::loader::STYLES.iter().map(|s| s.1).collect();
        let lrow = adw::ComboRow::builder()
            .title(crate::i18n::t("Loading animation"))
            .subtitle(crate::i18n::t("Shown while the app is busy, such as drawing or downloading"))
            .model(&gtk::StringList::new(&lnames))
            .build();
        lrow.set_selected(crate::loader::STYLES.iter().position(|s| s.0 == cfg.loader).unwrap_or(0) as u32);
        // a live preview of the choice
        let lprev = crate::loader::new(36);
        lrow.add_prefix(&lprev);
        let c = ctx.clone();
        lrow.connect_selected_notify(move |r| {
            let id = crate::loader::STYLES[r.selected() as usize].0;
            crate::loader::set_style(id);
            c.config.borrow_mut().loader = id.to_string();
            c.save_config();
        });
        g.add(&lrow);
        let names: Vec<&str> = crate::sounds::SOUNDS.iter().map(|s| s.1).collect();
        let snd = adw::ComboRow::builder()
            .title(crate::i18n::t("Startup sound"))
            .subtitle(crate::i18n::t("A two-second hymn opening when the app starts, synthesised from public-domain tunes"))
            .model(&gtk::StringList::new(&names))
            .build();
        snd.set_selected(crate::sounds::SOUNDS.iter().position(|s| s.0 == cfg.startup_sound).unwrap_or(0) as u32);
        let preview = gtk::Button::from_icon_name("media-playback-start-symbolic");
        preview.set_tooltip_text(Some("Preview"));
        preview.set_valign(gtk::Align::Center);
        preview.add_css_class("flat");
        preview.set_sensitive(snd.selected() != 0);
        let r = snd.clone();
        preview.connect_clicked(move |_| crate::sounds::play(crate::sounds::SOUNDS[r.selected() as usize].0));
        snd.add_suffix(&preview);
        let c = ctx.clone();
        snd.connect_selected_notify(move |r| {
            let id = crate::sounds::SOUNDS[r.selected() as usize].0;
            c.config.borrow_mut().startup_sound = id.to_string();
            c.save_config();
            preview.set_sensitive(r.selected() != 0);
        });
        g.add(&snd);
        g.add(&switch(
            &crate::i18n::t("Classic typography"),
            &crate::i18n::t("Old-style type like an early printed Bible, with a large initial letter opening each chapter"),
            cfg.vintage,
            Box::new(|c, v| {
                c.config.borrow_mut().vintage = v;
                crate::window::load_css(c);
            }),
        ));
        g.add(&switch(
            &crate::i18n::t("Show the Apocrypha / Deuterocanon"),
            &crate::i18n::t("Only for versions that include these books"),
            cfg.show_apocrypha,
            Box::new(|c, v| c.config.borrow_mut().show_apocrypha = v),
        ));
        g.add(&switch(
            &crate::i18n::t("Feature tips"),
            &crate::i18n::t("Small hints about things you can do"),
            cfg.feature_tips,
            Box::new(|c, v| {
                let mut cfg = c.config.borrow_mut();
                cfg.feature_tips = v;
                if v {
                    cfg.dismissed_tips.clear();
                }
            }),
        ));
        page.add(&g);

        // Notes
        let g = adw::PreferencesGroup::builder()
            .title(crate::i18n::t("Notes"))
            .description("Notes and highlights are plain Markdown files you own.")
            .build();
        let folder_row = adw::ActionRow::builder()
            .title(crate::i18n::t("Notes folder"))
            .subtitle(cfg.notes_dir.display().to_string())
            .build();
        let choose = gtk::Button::with_label(&crate::i18n::t("Change"));
        choose.set_valign(gtk::Align::Center);
        folder_row.add_suffix(&choose);
        g.add(&folder_row);
        g.add(&switch(
            &crate::i18n::t("Obsidian-compatible links"),
            &crate::i18n::t("Save references as bible:// links"),
            cfg.obsidian_links,
            Box::new(|c, v| c.config.borrow_mut().obsidian_links = v),
        ));
        let pal = adw::ExpanderRow::builder()
            .title(crate::i18n::t("Highlight colours"))
            .subtitle(crate::i18n::t("Name each colour so you can tell them apart"))
            .build();
        for (i, col) in cfg.palette.iter().enumerate() {
            let e = adw::EntryRow::builder()
                .title(&col.name)
                .text(&col.name)
                .build();
            let dot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            dot.add_css_class("swatch-dot");
            dot.add_css_class(&format!("swatch-{}", col.id));
            dot.set_valign(gtk::Align::Center);
            e.add_prefix(&dot);
            e.set_show_apply_button(true);
            let c = ctx.clone();
            e.connect_apply(move |e| {
                let t = e.text().trim().to_string();
                if !t.is_empty() {
                    if let Some(p) = c.config.borrow_mut().palette.get_mut(i) {
                        p.name = t.clone();
                    }
                    e.set_title(&t);
                    c.save_config();
                }
            });
            pal.add_row(&e);
        }
        g.add(&pal);
        page.add(&g);

        // Today
        // Wikipedia articles for the book introductions, one by one
        {
            let g = adw::PreferencesGroup::builder()
                .title(crate::i18n::t("Book introductions"))
                .description("Full Wikipedia articles with pictures, saved for offline reading: the 66 books and the 18 deuterocanonical books (Apocrypha). Download them one at a time here if downloading all at once is slowed down by Wikipedia.")
                .build();
            let exp = adw::ExpanderRow::builder()
                .title(crate::i18n::t("Wikipedia articles"))
                .subtitle(format!("{} of 84 saved", crate::wiki::count()))
                .build();
            const APOCRYPHA: &[&str] = &["Tob", "Jdt", "AddEsth", "Wis", "Sir", "Bar", "EpJer", "PrAzar", "Sus", "Bel", "1Macc", "2Macc", "3Macc", "4Macc", "1Esd", "2Esd", "PrMan", "Ps151"];
            for b in bible_core::plans::books().into_iter().chain(APOCRYPHA.iter().copied()) {
                let Some(intro) = bible_core::intros::intro(b) else { continue };
                let article = intro.article.clone();
                let row = adw::ActionRow::builder()
                    .title(bible_core::Ref::new(b, 1, 0).book_name())
                    .build();
                let status = |b: &str| match crate::wiki::load(b) {
                    Some((a, _)) => format!("Saved {}", a.fetched),
                    None => "Not downloaded".to_string(),
                };
                row.set_subtitle(&status(b));
                let get = gtk::Button::from_icon_name("folder-download-symbolic");
                get.set_tooltip_text(Some("Download or update"));
                get.set_valign(gtk::Align::Center);
                get.add_css_class("flat");
                let del = gtk::Button::from_icon_name("user-trash-symbolic");
                del.set_tooltip_text(Some("Remove"));
                del.set_valign(gtk::Align::Center);
                del.add_css_class("flat");
                del.set_visible(crate::wiki::load(b).is_some());
                let busy = crate::loader::new(28);
                busy.set_visible(false);
                row.add_suffix(&busy);
                row.add_suffix(&get);
                row.add_suffix(&del);
                {
                    let (row, del, exp, busy) = (row.clone(), del.clone(), exp.clone(), busy.clone());
                    let b = b.to_string();
                    get.connect_clicked(move |btn| {
                        btn.set_sensitive(false);
                        busy.set_visible(true);
                        row.set_subtitle("Downloading…");
                        let b = b.clone();
                        let (row, del, exp, btn, busy) = (row.clone(), del.clone(), exp.clone(), btn.clone(), busy.clone());
                        let (b2, art) = (b.clone(), article.clone());
                        gtk::glib::spawn_future_local(async move {
                            let res = gtk::gio::spawn_blocking(move || {
                                crate::wiki::fetch(&b2, &art).map(|_| ()).map_err(|e| crate::wiki::reason(&e))
                            })
                            .await;
                            btn.set_sensitive(true);
                            busy.set_visible(false);
                            match res {
                                Ok(Ok(())) => {
                                    row.set_subtitle(&status(&b));
                                    del.set_visible(true);
                                }
                                Ok(Err(why)) => row.set_subtitle(&format!("Failed: {why}")),
                                Err(_) => row.set_subtitle("Failed"),
                            }
                            exp.set_subtitle(&format!("{} of 84 saved", crate::wiki::count()));
                        });
                    });
                }
                {
                    let (row, exp) = (row.clone(), exp.clone());
                    let b = b.to_string();
                    del.connect_clicked(move |d| {
                        crate::wiki::remove(&b);
                        row.set_subtitle("Not downloaded");
                        d.set_visible(false);
                        exp.set_subtitle(&format!("{} of 84 saved", crate::wiki::count()));
                    });
                }
                exp.add_row(&row);
            }
            g.add(&exp);
            page.add(&g);
        }
        let g = adw::PreferencesGroup::builder().title(crate::i18n::t("Today")).build();
        let daily = adw::ComboRow::builder()
            .title(crate::i18n::t("Daily card"))
            .subtitle(crate::i18n::t("What Today shows each morning"))
            .model(&gtk::StringList::new(&["Devotional", "Nothing"]))
            .build();
        daily.set_selected(if cfg.daily_card == "off" { 1 } else { 0 });
        let c = ctx.clone();
        daily.connect_selected_notify(move |r| {
            c.config.borrow_mut().daily_card = if r.selected() == 1 {
                "off".into()
            } else {
                "devotional".into()
            };
            c.save_config();
            c.refresh_all();
        });
        g.add(&daily);
        g.add(&switch(
            &crate::i18n::t("Passage of the day"),
            &crate::i18n::t("A passage from your Bible on Today, built in — nothing to download"),
            cfg.daily_passage,
            Box::new(|c, v| c.config.borrow_mut().daily_passage = v),
        ));
        g.add(&switch(
            &crate::i18n::t("Fasting calendar"),
            &crate::i18n::t("Fast and abstinence days for your tradition on Today, with a month view"),
            cfg.fasting,
            Box::new(|c, v| c.config.borrow_mut().fasting = v),
        ));
        let devs: Vec<_> = ctx.lib().of_kind(bible_sword::Kind::Devotional);
        if devs.len() > 1 {
            let names: Vec<String> = devs.iter().map(|m| m.description()).collect();
            let nr: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
            let which = adw::ComboRow::builder()
                .title(crate::i18n::t("Devotional"))
                .model(&gtk::StringList::new(&nr))
                .build();
            which.set_selected(
                devs.iter()
                    .position(|m| m.name == cfg.devotional)
                    .unwrap_or(0) as u32,
            );
            let c = ctx.clone();
            let ids: Vec<String> = devs.iter().map(|m| m.name.clone()).collect();
            which.connect_selected_notify(move |r| {
                c.config.borrow_mut().devotional = ids[r.selected() as usize].clone();
                c.save_config();
                c.refresh_all();
            });
            g.add(&which);
        }
        page.add(&g);

        // Church calendar
        let g = adw::PreferencesGroup::builder()
            .title(crate::i18n::t("Church calendar"))
            .build();
        let names: Vec<&str> = Tradition::all().iter().map(|(_, n)| *n).collect();
        let trad = adw::ComboRow::builder()
            .title(crate::i18n::t("Tradition"))
            .subtitle(crate::i18n::t("Sets the date of Easter and which feasts appear on Today"))
            .model(&gtk::StringList::new(&names))
            .build();
        trad.set_selected(
            names
                .iter()
                .position(|n| n.eq_ignore_ascii_case(&cfg.tradition))
                .unwrap_or(0) as u32,
        );
        let c = ctx.clone();
        trad.connect_selected_notify(move |r| {
            c.config.borrow_mut().tradition = Tradition::all()[r.selected() as usize].1.to_string();
            c.save_config();
            c.refresh_all();
        });
        g.add(&trad);
        page.add(&g);

        // Library sources
        let sources_group = adw::PreferencesGroup::builder()
            .title(crate::i18n::t("Library sources"))
            .description("SWORD repositories the Library downloads from. Add a third-party server by its address — the folder that contains mods.d.tar.gz.")
            .build();
        page.add(&sources_group);
        let g = adw::PreferencesGroup::new();
        g.add(&switch(
            &crate::i18n::t("Show the legal notice in the Library"),
            &crate::i18n::t("A reminder that religious texts are restricted in some countries"),
            cfg.legal_notice,
            Box::new(|c, v| c.config.borrow_mut().legal_notice = v),
        ));
        page.add(&g);

        // updates
        let g = adw::PreferencesGroup::builder().title(crate::i18n::t("Updates")).build();
        g.add(&switch(
            &crate::i18n::t("Check for updates"),
            &crate::i18n::t("Once a day, ask GitHub whether a new version is out. Nothing else is sent"),
            cfg.check_updates,
            Box::new(|c, v| c.config.borrow_mut().check_updates = v),
        ));
        let now = adw::ActionRow::builder()
            .title(crate::i18n::t("Check now"))
            .subtitle(format!("You have version {}", crate::update::CURRENT))
            .activatable(true)
            .build();
        let busy = crate::loader::new(28);
        busy.set_visible(false);
        now.add_suffix(&busy);
        now.add_suffix(&gtk::Image::from_icon_name("view-refresh-symbolic"));
        let c = ctx.clone();
        now.connect_activated(move |row| {
            let Some(banner) = c.update_banner.get() else { return };
            busy.set_visible(true);
            row.set_subtitle("Checking…");
            let (row, busy) = (row.clone(), busy.clone());
            crate::update::check(
                &c,
                banner,
                true,
                Some(Box::new(move |msg| {
                    busy.set_visible(false);
                    row.set_subtitle(&msg);
                })),
            );
        });
        g.add(&now);
        page.add(&g);

        // About
        let g = adw::PreferencesGroup::new();
        let logs = adw::ActionRow::builder()
            .title(crate::i18n::t("Open log folder"))
            .subtitle(crate::i18n::t("Crash reports and errors, for reporting problems"))
            .activatable(true)
            .build();
        logs.add_suffix(&gtk::Image::from_icon_name("folder-open-symbolic"));
        let c = ctx.clone();
        logs.connect_activated(move |_| c.launch(&crate::log::dir()));
        let about = adw::ActionRow::builder()
            .title(crate::i18n::t("About Bible"))
            .subtitle(concat!(
                "Version ",
                env!("CARGO_PKG_VERSION"),
                " · GPL-3.0-or-later"
            ))
            .activatable(true)
            .build();
        about.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
        let c = ctx.clone();
        about.connect_activated(move |_| c.go("about"));
        g.add(&about);
        g.add(&logs);
        page.add(&g);

        let p = Rc::new(Prefs {
            ctx: ctx.clone(),
            root,
            page,
            sources_group,
            source_rows: RefCell::new(vec![]),
            folder_row,
        });
        let w = Rc::downgrade(&p);
        choose.connect_clicked(move |_| {
            let Some(p) = w.upgrade() else { return };
            let dlg = gtk::FileDialog::builder()
                .title(crate::i18n::t("Choose a notes folder"))
                .build();
            let parent = p.ctx.window.get().cloned();
            let w2 = Rc::downgrade(&p);
            dlg.select_folder(
                parent.as_ref(),
                None::<&gtk::gio::Cancellable>,
                move |res| {
                    if let (Ok(f), Some(p)) = (res, w2.upgrade()) {
                        if let Some(path) = f.path() {
                            p.ctx.config.borrow_mut().notes_dir = path.clone();
                            p.ctx.save_config();
                            p.folder_row.set_subtitle(&path.display().to_string());
                            p.ctx.refresh_all();
                        }
                    }
                },
            );
        });
        p.render_sources();
        p
    }

    fn render_sources(self: &Rc<Self>) {
        for r in self.source_rows.borrow_mut().drain(..) {
            self.sources_group.remove(&r);
        }
        let sources = self.ctx.config.borrow().sources.clone();
        let mut rows: Vec<gtk::Widget> = Vec::new();
        for (i, s) in sources.iter().enumerate() {
            let row = adw::SwitchRow::builder()
                .title(&s.name)
                .subtitle(&s.url)
                .active(s.enabled)
                .build();
            let c = self.ctx.clone();
            row.connect_active_notify(move |r| {
                if let Some(x) = c.config.borrow_mut().sources.get_mut(i) {
                    x.enabled = r.is_active();
                }
                c.save_config();
            });
            if s.custom {
                let rm = gtk::Button::from_icon_name("user-trash-symbolic");
                rm.add_css_class("flat");
                rm.set_valign(gtk::Align::Center);
                rm.set_tooltip_text(Some("Remove this source"));
                let w = Rc::downgrade(self);
                rm.connect_clicked(move |_| {
                    if let Some(p) = w.upgrade() {
                        p.ctx.config.borrow_mut().sources.remove(i);
                        p.ctx.save_config();
                        p.render_sources();
                    }
                });
                row.add_suffix(&rm);
            }
            self.sources_group.add(&row);
            rows.push(row.upcast());
        }
        let add = adw::EntryRow::builder()
            .title(crate::i18n::t("Add a source: name, then its address (https://…)"))
            .show_apply_button(true)
            .build();
        let w = Rc::downgrade(self);
        add.connect_apply(move |e| {
            let Some(p) = w.upgrade() else { return };
            let t = e.text().to_string();
            // accept "Name https://url" or just a URL
            let (name, url) = match t.rsplit_once(' ') {
                Some((n, u)) if u.starts_with("http") => {
                    (n.trim().to_string(), u.trim().to_string())
                }
                _ => (String::new(), t.trim().to_string()),
            };
            if !url.starts_with("http://") && !url.starts_with("https://") {
                p.ctx.toast("Enter an address starting with https://");
                return;
            }
            let name = if name.is_empty() {
                url.split('/').nth(2).unwrap_or("Custom source").to_string()
            } else {
                name
            };
            p.ctx.config.borrow_mut().sources.push(SourceCfg {
                name,
                url: url.trim_end_matches('/').to_string(),
                enabled: true,
                custom: true,
            });
            p.ctx.save_config();
            e.set_text("");
            p.render_sources();
            p.ctx
                .toast("Source added. Refresh the Library catalogue to see its modules.");
        });
        self.sources_group.add(&add);
        rows.push(add.upcast());
        let reset = adw::ActionRow::builder()
            .title(crate::i18n::t("Restore default sources"))
            .activatable(true)
            .build();
        let w = Rc::downgrade(self);
        reset.connect_activated(move |_| {
            if let Some(p) = w.upgrade() {
                let custom: Vec<SourceCfg> = p
                    .ctx
                    .config
                    .borrow()
                    .sources
                    .iter()
                    .filter(|s| s.custom)
                    .cloned()
                    .collect();
                let mut fresh = crate::config::Config::default().sources;
                fresh.extend(custom);
                p.ctx.config.borrow_mut().sources = fresh;
                p.ctx.save_config();
                p.render_sources();
            }
        });
        self.sources_group.add(&reset);
        rows.push(reset.upcast());
        *self.source_rows.borrow_mut() = rows;
        let _ = &self.page;
    }
}

fn page_wrap(ctx: &Rc<Ctx>, content: &adw::PreferencesPage) -> (adw::ToolbarView, adw::HeaderBar) {
    page(ctx, "Preferences", concat!("Version ", env!("CARGO_PKG_VERSION")), content)
}

impl Section for Prefs {
    fn id(&self) -> &'static str {
        "prefs"
    }
    fn title(&self) -> &'static str {
        "Preferences"
    }
    fn icon(&self) -> &'static str {
        "preferences-system-symbolic"
    }
    fn widget(&self) -> gtk::Widget {
        self.root.clone().upcast()
    }
    fn in_sidebar(&self) -> bool {
        false
    }
}
