//! The fasting calendar: a month at a time, for the user's tradition. Opt in.

use crate::ctx::{page, Ctx, Section};
use adw::prelude::*;
use bible_core::churchyear::Tradition;
use bible_core::fasting::{self, Kind};
use chrono::{Datelike, Duration, NaiveDate};
use std::cell::Cell;
use std::rc::Rc;

pub struct FastingView {
    ctx: Rc<Ctx>,
    root: adw::ToolbarView,
    title: adw::WindowTitle,
    body: gtk::Box,
    /// first day of the month shown
    month: Cell<NaiveDate>,
}

fn first_of(d: NaiveDate) -> NaiveDate {
    d.with_day(1).unwrap_or(d)
}

fn kind_class(k: Kind) -> &'static str {
    match k {
        Kind::Strict => "fast-strict",
        Kind::Fast => "fast-day",
        Kind::Abstinence => "fast-abstinence",
        Kind::FastFree => "fast-free",
    }
}

fn kind_name(k: Kind) -> &'static str {
    match k {
        Kind::Strict => "Strict fast",
        Kind::Fast => "Fast",
        Kind::Abstinence => "Abstinence",
        Kind::FastFree => "No fasting",
    }
}

impl FastingView {
    pub fn new(ctx: &Rc<Ctx>) -> Rc<FastingView> {
        let body = gtk::Box::new(gtk::Orientation::Vertical, 14);
        let clamp = adw::Clamp::builder().maximum_size(760).child(&body).build();
        clamp.set_margin_top(24);
        clamp.set_margin_bottom(32);
        clamp.set_margin_start(18);
        clamp.set_margin_end(18);
        let scroll = gtk::ScrolledWindow::builder()
            .child(&clamp)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build();
        let (root, hb) = page(ctx, "Fasting calendar", "", &scroll);
        let title = adw::WindowTitle::new("Fasting calendar", "");
        hb.set_title_widget(Some(&title));
        let prev = gtk::Button::from_icon_name("go-previous-symbolic");
        prev.set_tooltip_text(Some("Previous month"));
        let next = gtk::Button::from_icon_name("go-next-symbolic");
        next.set_tooltip_text(Some("Next month"));
        let today = gtk::Button::with_label("Today");
        hb.pack_end(&next);
        hb.pack_end(&prev);
        hb.pack_end(&today);
        let v = Rc::new(FastingView {
            ctx: ctx.clone(),
            root,
            title,
            body,
            month: Cell::new(first_of(chrono::Local::now().date_naive())),
        });
        let w = Rc::downgrade(&v);
        prev.connect_clicked(move |_| {
            if let Some(v) = w.upgrade() {
                v.month.set(first_of(v.month.get() - Duration::days(1)));
                v.render();
            }
        });
        let w = Rc::downgrade(&v);
        next.connect_clicked(move |_| {
            if let Some(v) = w.upgrade() {
                v.month.set(first_of(v.month.get() + Duration::days(32)));
                v.render();
            }
        });
        let w = Rc::downgrade(&v);
        today.connect_clicked(move |_| {
            if let Some(v) = w.upgrade() {
                v.month.set(first_of(chrono::Local::now().date_naive()));
                v.render();
            }
        });
        v.render();
        v
    }

    fn render(&self) {
        while let Some(c) = self.body.first_child() {
            self.body.remove(&c);
        }
        let trad = Tradition::from_name(&self.ctx.config.borrow().tradition);
        let first = self.month.get();
        self.title.set_title(&first.format("%B %Y").to_string());
        self.title
            .set_subtitle(&format!("{} practice", fasting::family(trad)));
        let today = chrono::Local::now().date_naive();

        // the month grid, weeks starting on Sunday
        let grid = gtk::Grid::new();
        grid.set_column_homogeneous(true);
        grid.set_row_spacing(6);
        grid.set_column_spacing(6);
        for (i, d) in ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"]
            .iter()
            .enumerate()
        {
            let l = gtk::Label::new(Some(d));
            l.add_css_class("caption-heading");
            l.add_css_class("dim-label");
            grid.attach(&l, i as i32, 0, 1, 1);
        }
        let offset = first.weekday().num_days_from_sunday() as i32;
        let mut d = first;
        while d.month() == first.month() {
            let idx = offset + d.day() as i32 - 1;
            let cell = gtk::Box::new(gtk::Orientation::Vertical, 2);
            cell.add_css_class("fast-cell");
            let n = gtk::Label::new(Some(&d.day().to_string()));
            n.set_xalign(0.0);
            n.add_css_class("heading");
            cell.append(&n);
            if let Some(f) = fasting::fast_day(d, trad) {
                cell.add_css_class(kind_class(f.kind));
                let t = gtk::Label::new(Some(kind_name(f.kind)));
                t.set_xalign(0.0);
                t.add_css_class("caption");
                t.set_ellipsize(gtk::pango::EllipsizeMode::End);
                cell.append(&t);
                cell.set_tooltip_text(Some(&format!("{}\n{}", f.title, f.detail)));
            }
            if d == today {
                cell.add_css_class("fast-today");
            }
            grid.attach(&cell, idx % 7, 1 + idx / 7, 1, 1);
            d += Duration::days(1);
        }
        self.body.append(&grid);

        // legend
        let legend = gtk::Box::new(gtk::Orientation::Horizontal, 14);
        for k in [Kind::Strict, Kind::Fast, Kind::Abstinence, Kind::FastFree] {
            let b = gtk::Box::new(gtk::Orientation::Horizontal, 6);
            let sw = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            sw.add_css_class("fast-swatch");
            sw.add_css_class(kind_class(k));
            b.append(&sw);
            let l = gtk::Label::new(Some(kind_name(k)));
            l.add_css_class("caption");
            b.append(&l);
            legend.append(&b);
        }
        self.body.append(&legend);

        // the month's days in words, grouped where a run shares a title
        let list = gtk::ListBox::new();
        list.add_css_class("boxed-list");
        list.set_selection_mode(gtk::SelectionMode::None);
        let days = fasting::month(first.year(), first.month(), trad);
        let mut i = 0;
        while i < days.len() {
            let (start, f) = &days[i];
            let mut j = i;
            while j + 1 < days.len()
                && days[j + 1].1 == *f
                && days[j + 1].0 == days[j].0 + Duration::days(1)
            {
                j += 1;
            }
            let when = if j > i {
                format!(
                    "{} – {}",
                    start.format("%b %-d"),
                    days[j].0.format("%b %-d")
                )
            } else {
                start.format("%a %b %-d").to_string()
            };
            let row = adw::ActionRow::builder()
                .title(gtk::glib::markup_escape_text(&f.title).as_str())
                .subtitle(gtk::glib::markup_escape_text(&format!("{when} · {}", f.detail)).as_str())
                .subtitle_lines(2)
                .build();
            let sw = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            sw.add_css_class("fast-swatch");
            sw.add_css_class(kind_class(f.kind));
            sw.set_valign(gtk::Align::Center);
            row.add_prefix(&sw);
            list.append(&row);
            i = j + 1;
        }
        if days.is_empty() {
            let row = adw::ActionRow::builder()
                .title("No fast days this month")
                .build();
            list.append(&row);
        }
        self.body.append(&list);
        let note = gtk::Label::new(Some(&format!(
            "{} Change your tradition in Preferences.",
            fasting::note(trad)
        )));
        note.set_wrap(true);
        note.set_xalign(0.0);
        note.add_css_class("caption");
        note.add_css_class("dim-label");
        self.body.append(&note);
    }
}

impl Section for FastingView {
    fn id(&self) -> &'static str {
        "fasting"
    }
    fn title(&self) -> &'static str {
        "Fasting calendar"
    }
    fn icon(&self) -> &'static str {
        "x-office-calendar-symbolic"
    }
    fn widget(&self) -> gtk::Widget {
        self.root.clone().upcast()
    }
    fn in_sidebar(&self) -> bool {
        false
    }
    fn refresh(&self) {
        self.render();
    }
}
