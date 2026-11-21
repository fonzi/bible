//! About: what the app is, who made it, and its licences. A full-window page.

use crate::ctx::{Ctx, Section};
use adw::prelude::*;
use std::rc::Rc;

pub const APP_ID: &str = "io.github.fonzi.Bible";

pub struct About {
    root: adw::ToolbarView,
}

fn row(title: &str, sub: &str) -> adw::ActionRow {
    adw::ActionRow::builder()
        .title(title)
        .subtitle(sub)
        .subtitle_selectable(true)
        .build()
}

impl About {
    pub fn new(ctx: &Rc<Ctx>) -> Rc<About> {
        let body = gtk::Box::new(gtk::Orientation::Vertical, 18);
        let icon = gtk::Image::from_icon_name(APP_ID);
        icon.set_pixel_size(128);
        icon.add_css_class("icon-dropshadow");
        let name = gtk::Label::new(Some("Bible"));
        name.add_css_class("title-1");
        let ver = gtk::Label::new(Some(concat!("Version ", env!("CARGO_PKG_VERSION"))));
        ver.add_css_class("app-version");
        ver.set_halign(gtk::Align::Center);
        let desc = gtk::Label::new(Some("An offline Bible study app for GNOME. Read many versions side by side, study the Greek and Hebrew word by word, follow the commentaries, and keep your notes and highlights as plain Markdown files."));
        desc.set_wrap(true);
        desc.set_justify(gtk::Justification::Center);
        desc.set_max_width_chars(60);
        let head = gtk::Box::new(gtk::Orientation::Vertical, 8);
        head.set_halign(gtk::Align::Center);
        head.append(&icon);
        head.append(&name);
        head.append(&ver);
        head.append(&desc);
        body.append(&head);

        let g = adw::PreferencesGroup::new();
        g.add(&row(
            "Licence",
            "GNU General Public License, version 3 or later",
        ));
        for (title, url) in [
            ("Website", "https://fonzi.github.io/bible/"),
            ("Source code", "https://github.com/fonzi/bible"),
            ("Report a problem", "https://github.com/fonzi/bible/issues"),
        ] {
            let r = adw::ActionRow::builder()
                .title(title)
                .subtitle(url)
                .activatable(true)
                .build();
            r.add_suffix(&gtk::Image::from_icon_name("adw-external-link-symbolic"));
            r.connect_activated(move |_| {
                gtk::UriLauncher::new(url).launch(gtk::Window::NONE, gtk::gio::Cancellable::NONE, |_| {});
            });
            g.add(&r);
        }
        body.append(&g);

        let g = adw::PreferencesGroup::builder()
            .title("Texts and data")
            .build();
        g.add(&row("CrossWire Bible Society · SWORD modules", "Bibles, commentaries and books. Each module keeps its own licence, shown in Downloads."));
        g.add(&row("STEP Bible · Tyndale House, Cambridge", "Interlinear Greek and Hebrew, glosses, grammar and brief lexicons (TAGNT, TAHOT, TBESG, TBESH). CC BY 4.0 · www.STEPBible.org"));
        g.add(&row("Places · STEPBible TIPNR and OpenBible.info", "Biblical places with every reference (TIPNR, CC BY 4.0, www.STEPBible.org); locations from OpenBible.info (CC BY 4.0)."));
        g.add(&row("Book introductions", "Overviews from Wikipedia (CC BY-SA 4.0), each credited on its page, with traditional author and date added for this app."));
        g.add(&row("Audio", "Berean Standard Bible narrated by Bob Souer, public domain, from OpenBible.com. Downloaded only when you ask."));
        g.add(&row("Startup sounds", "Hymn openings (Old Hundredth, New Britain, Nicaea) synthesised for this app from public-domain tunes. “Kyrie” is the opening of Kyrie XI (Orbis Factor) sung by Paterm, from Wikimedia Commons, CC BY-SA 3.0 (creativecommons.org/licenses/by-sa/3.0), shortened to two seconds with a fade."));
        g.add(&row("Cross-references and Wikipedia articles", "Cross-references from OpenBible.info (CC BY 4.0). Full book articles from Wikipedia (CC BY-SA 4.0) with Wikimedia Commons pictures, each credited with its own licence."));
        g.add(&row("Church history and fasting", "Today in church history and the fasting calendar were compiled for this app from standard church histories and each tradition's published norms."));
        g.add(&row(
            "Natural Earth",
            "Coastlines, lakes and rivers for the maps. Public domain · naturalearthdata.com",
        ));
        g.add(&row("Timeline", "Compiled for this app from the biblical text and standard chronologies: Thiele for the kings of Israel and Judah, the usual ancient Near Eastern dates for empires and rulers, and common church-history dates. Uncertain dates are marked c."));
        g.add(&row(
            "SWORD file formats",
            "Read by this app’s own GPL-3 implementation; no SWORD library code is used.",
        ));
        g.add(&row(
            "Versification data",
            "Book, chapter and verse counts for 18 versification systems",
        ));
        body.append(&g);

        let g = adw::PreferencesGroup::builder().title("Built with").build();
        g.add(&row(
            "Rust, GTK 4, libadwaita",
            "gtk-rs bindings, flate2, tar, ureq, serde, chrono, regex",
        ));
        body.append(&g);

        let warn = gtk::Label::new(Some("Owning or reading religious texts is restricted or illegal in some countries. Please take precautions when downloading."));
        warn.set_wrap(true);
        warn.add_css_class("warning-bar");
        warn.set_xalign(0.0);
        body.append(&warn);
        let legal = gtk::Label::new(Some("This program comes with absolutely no warranty. It is free software, and you are welcome to redistribute it under the terms of the GNU General Public License, version 3 or later."));
        legal.set_wrap(true);
        legal.set_justify(gtk::Justification::Center);
        legal.add_css_class("dim-label");
        legal.add_css_class("caption");
        body.append(&legal);

        let clamp = adw::Clamp::builder().maximum_size(600).child(&body).build();
        clamp.set_margin_top(24);
        clamp.set_margin_bottom(32);
        clamp.set_margin_start(18);
        clamp.set_margin_end(18);
        let scroll = gtk::ScrolledWindow::builder()
            .child(&clamp)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build();
        let tv = adw::ToolbarView::new();
        let hb = adw::HeaderBar::new();
        let back = gtk::Button::from_icon_name("go-previous-symbolic");
        back.set_tooltip_text(Some("Back to Preferences"));
        let c = ctx.clone();
        back.connect_clicked(move |_| c.go("prefs"));
        hb.pack_start(&back);
        hb.set_title_widget(Some(&adw::WindowTitle::new("About", "")));
        tv.add_top_bar(&hb);
        tv.set_content(Some(&scroll));
        Rc::new(About { root: tv })
    }
}

impl Section for About {
    fn id(&self) -> &'static str {
        "about"
    }
    fn title(&self) -> &'static str {
        "About"
    }
    fn icon(&self) -> &'static str {
        "help-about-symbolic"
    }
    fn widget(&self) -> gtk::Widget {
        self.root.clone().upcast()
    }
    fn in_sidebar(&self) -> bool {
        false
    }
}
