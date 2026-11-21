//! Today: the church day and season, and a way back into reading.

use crate::ctx::{page, Ctx, Section};
use adw::prelude::*;
use bible_core::churchyear::{day_info, short_date, Tradition};
use bible_core::Ref;
use bible_sword::Kind;
use chrono::Datelike;
use std::rc::Rc;

pub struct Today {
    ctx: Rc<Ctx>,
    root: adw::ToolbarView,
    body: gtk::Box,
    title: adw::WindowTitle,
}

fn card() -> gtk::Box {
    let b = gtk::Box::new(gtk::Orientation::Vertical, 6);
    b.add_css_class("card");
    b.add_css_class("today-card");
    b
}

fn label(text: &str, classes: &[&str]) -> gtk::Label {
    let l = gtk::Label::new(Some(text));
    l.set_xalign(0.0);
    l.set_wrap(true);
    for c in classes {
        l.add_css_class(c);
    }
    l
}

impl Today {
    pub fn new(ctx: &Rc<Ctx>) -> Rc<Today> {
        let body = gtk::Box::new(gtk::Orientation::Vertical, 18);
        let clamp = adw::Clamp::builder().maximum_size(1180).tightening_threshold(900).child(&body).build();
        clamp.set_margin_top(24);
        clamp.set_margin_bottom(32);
        clamp.set_margin_start(18);
        clamp.set_margin_end(18);
        let scroll = gtk::ScrolledWindow::builder()
            .child(&clamp)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build();
        let (root, hb) = page(ctx, "Today", "", &scroll);
        let title = hb
            .title_widget()
            .and_downcast::<adw::WindowTitle>()
            .unwrap();
        let t = Rc::new(Today {
            ctx: ctx.clone(),
            root,
            body,
            title,
        });
        t.refresh();
        t
    }
}

impl Today {
    fn plan_card(&self, cfg: &crate::config::Config) -> gtk::Box {
        use bible_core::plans;
        let c = card();
        let active = cfg
            .plan
            .as_ref()
            .and_then(|s| plans::plan(&s.id).map(|p| (s.clone(), p)));
        let Some((st, plan)) = active else {
            c.append(&label(&crate::i18n::t("Reading plan"), &["caption-heading", "dim-label"]));
            c.append(&label(&crate::i18n::t("Read through the Bible with a plan"), &["title-3"]));
            c.append(&label(&crate::i18n::t("A year through the whole Bible, chronological order, the New Testament in 90 days, and more."), &[]));
            let b = gtk::Button::with_label(&crate::i18n::t("Choose a plan"));
            b.add_css_class("pill");
            b.set_halign(gtk::Align::Start);
            b.set_margin_top(6);
            let ctx = self.ctx.clone();
            b.connect_clicked(move |_| ctx.go("plans"));
            c.append(&b);
            return c;
        };
        let days = plans::days(plan.id);
        let n = days.len();
        let Some(next) = st.next(n) else {
            c.append(&label(plan.title, &["caption-heading", "dim-label"]));
            c.append(&label(&crate::i18n::t("You finished the plan!"), &["title-3"]));
            let b = gtk::Button::with_label(&crate::i18n::t("Choose another"));
            b.add_css_class("pill");
            b.set_halign(gtk::Align::Start);
            let ctx = self.ctx.clone();
            b.connect_clicked(move |_| ctx.go("plans"));
            c.append(&b);
            return c;
        };
        let now = chrono::Local::now().date_naive();
        let behind = st.behind(now, n);
        c.append(&label(
            &format!("{} · day {} of {n}", plan.title, next + 1),
            &["caption-heading", "dim-label"],
        ));
        c.append(&label(&plans::describe(&days[next]), &["title-3", "serif"]));
        let bar = gtk::ProgressBar::new();
        bar.set_fraction(st.done.len() as f64 / n as f64);
        bar.set_margin_top(4);
        c.append(&bar);
        if behind > 0 {
            c.append(&label(
                &format!(
                    "{behind} day{} behind. No rush, just carry on.",
                    if behind == 1 { "" } else { "s" }
                ),
                &["caption", "dim-label"],
            ));
        }
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        row.set_margin_top(6);
        // one primary action per page ("Continue reading"); this one is quieter
        let read = gtk::Button::with_label(&crate::i18n::t("Read"));
        read.add_css_class("pill");
        let first = days[next].first().cloned();
        let ctx = self.ctx.clone();
        read.connect_clicked(move |_| {
            if let Some((b, ch)) = &first {
                ctx.open_ref(&Ref::new(b, *ch, 0));
            }
        });
        let done = gtk::Button::with_label(&crate::i18n::t("Mark as done"));
        done.add_css_class("pill");
        let ctx = self.ctx.clone();
        done.connect_clicked(move |_| {
            crate::plans::set_done(&ctx, next, true);
            ctx.refresh_all();
        });
        let all = gtk::Button::with_label(&crate::i18n::t("Whole plan"));
        all.add_css_class("pill");
        all.add_css_class("flat");
        let ctx = self.ctx.clone();
        all.connect_clicked(move |_| ctx.go("plans"));
        row.append(&read);
        row.append(&done);
        row.append(&all);
        c.append(&row);
        c
    }
}

impl Today {
    /// "What's happening": when, headings, timeline events and places of a chapter.
    fn story(&self, book: &str, chapter: u16) -> Option<gtk::Box> {
        let b = gtk::Box::new(gtk::Orientation::Vertical, 4);
        b.append(&label(&crate::i18n::t("What’s happening"),
            &["caption-heading", "dim-label"],
        ));
        let mut lines = 0;
        let tl = bible_core::timeline::timeline();
        if let Some((s, e)) = tl.when(book, chapter) {
            let era = tl
                .era_at(s)
                .map(|x| format!(" · {}", x.title))
                .unwrap_or_default();
            b.append(&label(
                &format!("{}{era}", bible_core::timeline::span(s, e, true)),
                &["heading"],
            ));
            lines += 1;
        }
        // section headings from the reading Bible
        if let Some(m) = self.ctx.bible() {
            let heads: Vec<String> = m
                .chapter(book, chapter)
                .into_iter()
                .flat_map(|(_, sp)| {
                    sp.into_iter()
                        .filter(|x| x.title)
                        .map(|x| x.text.trim().to_string())
                })
                .filter(|t| {
                    !t.is_empty()
                        && !t.to_lowercase().starts_with("chapter")
                        && !t.to_lowercase().starts_with("psalm ")
                })
                .take(4)
                .collect();
            if !heads.is_empty() {
                b.append(&label(&heads.join(" · "), &[]));
                lines += 1;
            } else if let Some(first) = m.verse_plain(book, chapter, 1) {
                let t: String = first.trim().chars().take(140).collect();
                b.append(&label(
                    &format!(
                        "“{t}{}”",
                        if first.chars().count() > 140 {
                            "…"
                        } else {
                            ""
                        }
                    ),
                    &["serif"],
                ));
                lines += 1;
            }
        }
        // events on the timeline that point into this chapter
        let events: Vec<&str> = tl
            .events
            .iter()
            .filter(|e| {
                e.refs.iter().any(|r| {
                    let (a, z) = r.split_once('-').unwrap_or((r, r));
                    match (
                        bible_core::refs::parse_osis(a),
                        bible_core::refs::parse_osis(z),
                    ) {
                        (Some(a), Some(z)) => {
                            a.book == book && a.chapter <= chapter && z.chapter >= chapter
                        }
                        _ => false,
                    }
                })
            })
            .map(|e| e.title.as_str())
            .take(3)
            .collect();
        if !events.is_empty() {
            b.append(&label(
                &format!("On the timeline: {}", events.join(" · ")),
                &["dim-label"],
            ));
            lines += 1;
        }
        let places: Vec<String> = bible_core::places::places()
            .in_chapter(book, chapter)
            .iter()
            .map(|(p, _)| p.name.clone())
            .take(5)
            .collect();
        if !places.is_empty() {
            b.append(&label(
                &format!("Places: {}", places.join(", ")),
                &["dim-label"],
            ));
            lines += 1;
        }
        (lines > 0).then_some(b)
    }

    fn passage_card(&self, now: chrono::NaiveDate, trad: Tradition) -> Option<gtk::Box> {
        let m = self.ctx.bible()?;
        let (occasion, osis) = bible_core::daily::passage(now, trad);
        let (book, chapter, v0, v1) = bible_core::daily::verses(osis)?;
        let mut text = String::new();
        let mut marked = String::new();
        for v in v0..=v1 {
            if let Some(t) = m.verse_plain(&book, chapter, v) {
                if v1 > v0 {
                    marked.push_str(&format!(
                        "<span size=\"small\" alpha=\"55%\" rise=\"3pt\">{v}</span> "
                    ));
                }
                text.push_str(t.trim());
                text.push(' ');
                marked.push_str(&gtk::glib::markup_escape_text(t.trim()));
                marked.push(' ');
            }
        }
        if text.trim().is_empty() {
            return None;
        }
        let c = card();
        c.add_css_class("passage-card");
        let name = Ref::new(&book, chapter, 0).book_name();
        let r = if v1 > v0 {
            format!("{name} {chapter}:{v0}–{v1}")
        } else {
            format!("{name} {chapter}:{v0}")
        };
        let kicker = match occasion {
            Some(o) => format!("Passage for {o}"),
            None => "Passage of the day".to_string(),
        };
        c.append(&label(&kicker, &["caption-heading", "dim-label"]));
        c.append(&label(&r, &["title-3", "serif"]));
        let body = label("", &["serif", "daily-passage"]);
        body.set_markup(marked.trim());
        body.set_selectable(true);
        c.append(&body);
        c.append(&label(&m.abbreviation(), &["caption", "dim-label"]));
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        row.set_margin_top(6);
        let open = gtk::Button::with_label(&crate::i18n::t("Read in context"));
        open.add_css_class("pill");
        let ctx = self.ctx.clone();
        let rf = Ref::new(&book, chapter, v0);
        open.connect_clicked(move |_| ctx.open_ref(&rf));
        let copy = gtk::Button::with_label(&crate::i18n::t("Copy"));
        copy.add_css_class("pill");
        copy.add_css_class("flat");
        let t = format!("{} — {r} ({})", text.trim(), m.abbreviation());
        let ctx = self.ctx.clone();
        copy.connect_clicked(move |b| {
            b.clipboard().set_text(&t);
            ctx.toast("Passage copied");
        });
        row.append(&open);
        row.append(&copy);
        c.append(&row);
        Some(c)
    }

    fn fasting_card(&self, now: chrono::NaiveDate, trad: Tradition) -> gtk::Box {
        use bible_core::fasting::{self, Kind as F};
        let c = card();
        c.append(&label(
            &format!("Fasting · {} practice", fasting::family(trad)),
            &["caption-heading", "dim-label"],
        ));
        match fasting::fast_day(now, trad) {
            Some(f) => {
                let kind = match f.kind {
                    F::Strict => "Strict fast",
                    F::Fast => "Fast day",
                    F::Abstinence => "Abstinence",
                    F::FastFree => "No fasting",
                };
                c.append(&label(&format!("{} · {kind}", f.title), &["title-3"]));
                c.append(&label(&f.detail, &[]));
            }
            None => {
                c.append(&label(&crate::i18n::t("No fast today"), &["title-3"]));
            }
        }
        // the next fast day, for planning
        let current = fasting::fast_day(now, trad).map(|f| f.title);
        let next = (1..=60)
            .map(|k| now + chrono::Duration::days(k))
            .find_map(|d| {
                fasting::fast_day(d, trad)
                    .filter(|f| f.kind != F::FastFree && Some(&f.title) != current.as_ref())
                    .map(|f| (d, f))
            });
        if let Some((d, f)) = next {
            c.append(&label(
                &format!("Next: {} · {}", f.title, short_date(d)),
                &["dim-label"],
            ));
        }
        let b = gtk::Button::with_label(&crate::i18n::t("This month"));
        b.add_css_class("pill");
        b.set_halign(gtk::Align::Start);
        b.set_margin_top(6);
        let ctx = self.ctx.clone();
        b.connect_clicked(move |_| ctx.go("fasting"));
        c.append(&b);
        c
    }

    fn history_card(&self, now: chrono::NaiveDate) -> gtk::Box {
        let c = card();
        let (exact, events) = bible_core::onthisday::for_date(now);
        if events.is_empty() {
            c.append(&label(&crate::i18n::t("Today in church history"),
                &["caption-heading", "dim-label"],
            ));
            c.append(&label(&crate::i18n::t("Nothing recorded for this week yet."),
                &["dim-label"],
            ));
            return c;
        }
        let when = if exact {
            "Today in church history".to_string()
        } else {
            let e = &events[0];
            format!(
                "Coming up in church history · {}",
                chrono::NaiveDate::from_ymd_opt(now.year_ce().1 as i32, e.month, e.day)
                    .map(short_date)
                    .unwrap_or_default()
            )
        };
        c.append(&label(&when, &["caption-heading", "dim-label"]));
        for e in events {
            let y = if e.year < 1000 {
                format!("AD {}", e.year)
            } else {
                e.year.to_string()
            };
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
            let yl = label(&y, &["title-3", "numeric"]);
            yl.set_width_chars(6);
            yl.set_valign(gtk::Align::Start);
            row.append(&yl);
            let t = label(e.text, &[]);
            t.set_hexpand(true);
            row.append(&t);
            c.append(&row);
        }
        let b = gtk::Button::with_label(&crate::i18n::t("Open the timeline"));
        b.add_css_class("pill");
        b.add_css_class("flat");
        b.set_halign(gtk::Align::Start);
        b.set_margin_top(6);
        let ctx = self.ctx.clone();
        b.connect_clicked(move |_| ctx.go("timeline"));
        c.append(&b);
        c
    }
}

impl Section for Today {
    fn id(&self) -> &'static str {
        "today"
    }
    fn title(&self) -> &'static str {
        "Today"
    }
    fn icon(&self) -> &'static str {
        "weather-clear-symbolic"
    }
    fn widget(&self) -> gtk::Widget {
        self.root.clone().upcast()
    }
    fn shown(&self) {
        self.refresh();
    }
    fn refresh(&self) {
        while let Some(c) = self.body.first_child() {
            self.body.remove(&c);
        }
        let now = chrono::Local::now().date_naive();
        self.title
            .set_subtitle(&now.format("%A, %B %-d, %Y").to_string());
        let cfg = self.ctx.config.borrow().clone();
        let trad = Tradition::from_name(&cfg.tradition);
        let d = day_info(now, trad);

        // church day
        let c = card();
        c.add_css_class("season-card");
        // the liturgical colour as a slim edge down the card, named in words too
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 16);
        let sw = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        sw.add_css_class("season-swatch");
        sw.add_css_class(&format!("season-{}", d.colour.name()));
        sw.set_valign(gtk::Align::Fill);
        let txt = gtk::Box::new(gtk::Orientation::Vertical, 2);
        txt.append(&label(
            &format!("{} · liturgical colour {}", d.season, d.colour.name()),
            &["caption-heading", "dim-label"],
        ));
        txt.append(&label(&d.title, &["title-2", "serif"]));
        if !d.upcoming.is_empty() {
            let up: Vec<String> = d
                .upcoming
                .iter()
                .map(|(n, dt)| format!("{n}, {}", short_date(*dt)))
                .collect();
            txt.append(&label(
                &format!("Coming up: {}", up.join(" · ")),
                &["dim-label"],
            ));
        }
        txt.append(&label(
            &format!("{} calendar · change it in Preferences", trad.name()),
            &["caption", "dim-label"],
        ));
        row.append(&sw);
        row.append(&txt);
        c.append(&row);
        self.body.append(&c);

        // continue reading
        let lib = self.ctx.lib();
        let bibles = lib.of_kind(Kind::Bible);
        let c = card();
        if bibles.is_empty() {
            c.append(&label(&crate::i18n::t("Get started"), &["caption-heading", "dim-label"]));
            c.append(&label(&crate::i18n::t("Download a Bible"), &["title-3"]));
            c.append(&label(&crate::i18n::t("Downloads has hundreds of versions in many languages, plus commentaries and classic books. Download once, read offline."), &[]));
            let b = gtk::Button::with_label(&crate::i18n::t("Open Downloads"));
            b.add_css_class("suggested-action");
            b.add_css_class("pill");
            b.set_halign(gtk::Align::Start);
            b.set_margin_top(6);
            let ctx = self.ctx.clone();
            b.connect_clicked(move |_| ctx.go("library"));
            c.append(&b);
        } else {
            let r = Ref::new(&cfg.book, cfg.chapter.max(1), 0);
            c.append(&label(&crate::i18n::t("Pick up where you left off"),
                &["caption-heading", "dim-label"],
            ));
            c.append(&label(&r.to_string(), &["title-3", "serif"]));
            let notes = self.ctx.notes().load(&cfg.book, cfg.chapter.max(1));
            if let Some((v, n)) = notes.notes.iter().next() {
                let first: String = n.chars().take(90).collect();
                c.append(&label(
                    &format!(
                        "Your note on verse {v}: “{first}{}”",
                        if n.chars().count() > 90 { "…" } else { "" }
                    ),
                    &["dim-label"],
                ));
            }
            let b = gtk::Button::with_label(&crate::i18n::t("Continue reading"));
            b.add_css_class("suggested-action");
            b.add_css_class("pill");
            b.set_halign(gtk::Align::Start);
            b.set_margin_top(6);
            let ctx = self.ctx.clone();
            b.connect_clicked(move |_| ctx.open_ref(&r));
            c.append(&b);
            if let Some(story) = self.story(&cfg.book, cfg.chapter.max(1)) {
                let row = gtk::Box::new(gtk::Orientation::Horizontal, 24);
                let left = gtk::Box::new(gtk::Orientation::Vertical, 4);
                while let Some(w) = c.first_child() {
                    c.remove(&w);
                    left.append(&w);
                }
                left.set_hexpand(true);
                story.set_hexpand(true);
                row.append(&left);
                row.append(&gtk::Separator::new(gtk::Orientation::Vertical));
                row.append(&story);
                row.set_homogeneous(false);
                c.append(&row);
            }
        }
        self.body.append(&c);

        // everything else is tiled in two masonry columns, each card going
        // to the shorter column, so the day fits on about one screen
        let cols = gtk::Box::new(gtk::Orientation::Horizontal, 18);
        cols.set_homogeneous(true);
        cols.add_css_class("today-tiles");
        let col = [
            gtk::Box::new(gtk::Orientation::Vertical, 18),
            gtk::Box::new(gtk::Orientation::Vertical, 18),
        ];
        for c in &col {
            c.set_valign(gtk::Align::Start);
            cols.append(c);
        }
        let pending: std::cell::RefCell<Vec<gtk::Box>> = Default::default();
        let tile = |w: &gtk::Box, _h: u32| pending.borrow_mut().push(w.clone());

        // the passage of the day, built in
        if cfg.daily_passage && !bibles.is_empty() {
            if let Some(card) = self.passage_card(now, trad) {
                tile(&card, 5);
            }
        }

        // reading plan
        if !bibles.is_empty() {
            tile(&self.plan_card(&cfg), 3);
        }

        // devotional
        let devs = lib.of_kind(Kind::Devotional);
        if cfg.daily_card != "off" {
            let c = card();
            match devs
                .iter()
                .find(|m| m.name == cfg.devotional)
                .or(devs.first())
            {
                Some(m) => {
                    c.append(&label(
                        &format!("Devotional · {}", m.description()),
                        &["caption-heading", "dim-label"],
                    ));
                    c.append(&label(
                        &now.format("%B %-d").to_string(),
                        &["title-3", "serif"],
                    ));
                    let key = now.format("%m.%d").to_string();
                    if let Some((_, raw)) = m.ld().and_then(|ld| ld.get(&key)) {
                        let plain = bible_sword::markup::plain(m.source_type(), &raw);
                        let first: String = plain.chars().take(160).collect();
                        c.append(&label(
                            &format!(
                                "{first}{}",
                                if plain.chars().count() > 160 {
                                    "…"
                                } else {
                                    ""
                                }
                            ),
                            &["dim-label"],
                        ));
                    }
                    let b = gtk::Button::with_label(&crate::i18n::t("Read today’s entry"));
                    b.add_css_class("pill");
                    b.set_halign(gtk::Align::Start);
                    b.set_margin_top(6);
                    let ctx = self.ctx.clone();
                    let name = m.name.clone();
                    b.connect_clicked(move |_| ctx.open_book(&name, Some("today")));
                    c.append(&b);
                }
                None => {
                    // nothing installed: one quiet line, not a full card
                    c.add_css_class("compact-card");
                    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
                    let l = label(&crate::i18n::t("Add a daily devotional: Spurgeon’s Morning and Evening, Daily Light and others are free."), &["dim-label"]);
                    l.set_hexpand(true);
                    row.append(&l);
                    let b = gtk::Button::with_label(&crate::i18n::t("Browse"));
                    b.add_css_class("flat");
                    b.set_valign(gtk::Align::Center);
                    let ctx = self.ctx.clone();
                    b.connect_clicked(move |_| ctx.go("library"));
                    row.append(&b);
                    c.append(&row);
                }
            }
            tile(&c, 3);
        }

        // what you have
        let coms = lib.of_kind(Kind::Commentary).len();
        // fasting (opt in)
        if cfg.fasting {
            tile(&self.fasting_card(now, trad), 3);
        }

        // today in church history
        tile(&self.history_card(now), 4);

        // balance the columns by each card's real height at the column width
        let total = self.body.width();
        let colw = if total > 200 { (total - 18) / 2 } else { 480 };
        let mut height = [0i32; 2];
        for w in pending.take() {
            let (_, nat, _, _) = w.measure(gtk::Orientation::Vertical, colw);
            let i = if height[1] < height[0] { 1 } else { 0 };
            height[i] += nat + 18;
            col[i].append(&w);
        }
        self.body.append(&cols);

        // what you have, as a quiet footer rather than a card
        let il: Vec<&str> = bible_step::Pack::all()
            .iter()
            .filter(|p| bible_step::is_installed(**p))
            .map(|p| match p {
                bible_step::Pack::GreekNt => "Greek",
                bible_step::Pack::HebrewOt => "Hebrew",
            })
            .collect();
        let foot = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        foot.set_halign(gtk::Align::Center);
        foot.set_margin_top(6);
        let summary = format!(
            "{} {} · {} {} · {} other · interlinear: {}",
            bibles.len(),
            if bibles.len() == 1 { "Bible" } else { "Bibles" },
            coms,
            if coms == 1 { "commentary" } else { "commentaries" },
            lib.modules.len().saturating_sub(bibles.len() + coms),
            if il.is_empty() { "none yet".to_string() } else { il.join(" and ") }
        );
        foot.append(&label(&summary, &["caption", "dim-label"]));
        let b = gtk::Button::with_label(&crate::i18n::t("Downloads"));
        b.add_css_class("flat");
        b.add_css_class("caption");
        let ctx = self.ctx.clone();
        b.connect_clicked(move |_| ctx.go("library"));
        foot.append(&b);
        self.body.append(&foot);
    }
}
