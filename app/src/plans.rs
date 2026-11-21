//! Reading plans: choose one, then tick off each day.

use crate::ctx::{page, Ctx, Section};
use adw::prelude::*;
use bible_core::plans::{self, PlanState, PLANS};
use bible_core::Ref;
use gtk::glib;
use std::rc::Rc;

pub struct PlansView {
    ctx: Rc<Ctx>,
    root: adw::ToolbarView,
    body: gtk::Box,
    scroll: gtk::ScrolledWindow,
    me: std::cell::RefCell<std::rc::Weak<PlansView>>,
}

pub fn today() -> chrono::NaiveDate {
    chrono::Local::now().date_naive()
}

/// Start a plan today, replacing any other.
pub fn start(ctx: &Ctx, id: &str) {
    ctx.config.borrow_mut().plan = Some(PlanState {
        id: id.to_string(),
        start: today().format("%Y-%m-%d").to_string(),
        done: vec![],
    });
    ctx.save_config();
    ctx.refresh_all();
}

/// Mark a day finished or not.
pub fn set_done(ctx: &Ctx, day: usize, done: bool) {
    {
        let mut c = ctx.config.borrow_mut();
        let Some(p) = c.plan.as_mut() else { return };
        p.done.retain(|d| *d as usize != day);
        if done {
            p.done.push(day as u16);
            p.done.sort();
        }
    }
    ctx.save_config();
}

impl PlansView {
    pub fn new(ctx: &Rc<Ctx>) -> Rc<PlansView> {
        let body = gtk::Box::new(gtk::Orientation::Vertical, 18);
        let clamp = adw::Clamp::builder().maximum_size(760).child(&body).build();
        clamp.set_margin_top(24);
        clamp.set_margin_bottom(32);
        clamp.set_margin_start(18);
        clamp.set_margin_end(18);
        let scroll = gtk::ScrolledWindow::builder()
            .child(&clamp)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build();
        let (root, _) = page(ctx, "Reading plans", "", &scroll);
        let v = Rc::new(PlansView {
            ctx: ctx.clone(),
            root,
            body,
            scroll,
            me: Default::default(),
        });
        *v.me.borrow_mut() = Rc::downgrade(&v);
        v.refresh();
        v
    }

    fn render(&self) {
        while let Some(c) = self.body.first_child() {
            self.body.remove(&c);
        }
        let state = self.ctx.config.borrow().plan.clone();
        let active = state
            .as_ref()
            .and_then(|s| plans::plan(&s.id).map(|p| (s.clone(), p)));
        if let Some((st, plan)) = active {
            let days = plans::days(plan.id);
            let n = days.len();
            let now = today();
            let done = st.done.len();
            let g = adw::PreferencesGroup::new();
            g.set_title(plan.title);
            let behind = st.behind(now, n);
            let status = if done >= n {
                "Finished. Well done!".to_string()
            } else if behind > 0 {
                format!(
                    "Day {} of {n} · {done} done · {behind} day{} behind, no rush",
                    st.today(now, n) + 1,
                    if behind == 1 { "" } else { "s" }
                )
            } else {
                format!(
                    "Day {} of {n} · {done} done · on track",
                    st.today(now, n) + 1
                )
            };
            g.set_description(Some(&status));
            let stop = gtk::Button::with_label("Stop plan");
            stop.add_css_class("flat");
            let ctx = self.ctx.clone();
            stop.connect_clicked(move |b| {
                if b.label().as_deref() == Some("Stop plan") {
                    b.set_label("Stop and clear progress");
                    b.add_css_class("destructive-action");
                    return;
                }
                ctx.config.borrow_mut().plan = None;
                ctx.save_config();
                ctx.refresh_all();
            });
            g.set_header_suffix(Some(&stop));
            let bar = gtk::ProgressBar::new();
            bar.set_fraction(done as f64 / n.max(1) as f64);
            bar.set_margin_bottom(6);
            g.add(&bar);
            let start = st.start_date().unwrap_or(now);
            let next = st.next(n);
            let mut next_row: Option<adw::ActionRow> = None;
            for (i, day) in days.iter().enumerate() {
                let date = start + chrono::Duration::days(i as i64);
                let row = adw::ActionRow::builder()
                    .title(glib::markup_escape_text(&plans::describe(day)).as_str())
                    .subtitle(format!("Day {} · {}", i + 1, date.format("%a %b %-d")))
                    .build();
                let check = gtk::CheckButton::new();
                check.set_active(st.done.contains(&(i as u16)));
                check.set_valign(gtk::Align::Center);
                check.set_tooltip_text(Some("Done"));
                let ctx = self.ctx.clone();
                check.connect_toggled(move |c| {
                    set_done(&ctx, i, c.is_active());
                    // other sections (Today) show progress too
                    let ctx = ctx.clone();
                    glib::idle_add_local_once(move || ctx.refresh_except("plans"));
                });
                row.add_prefix(&check);
                row.set_activatable_widget(None::<&gtk::Widget>);
                let read = gtk::Button::with_label("Read");
                read.add_css_class("flat");
                read.set_valign(gtk::Align::Center);
                let first = day.first().cloned();
                let ctx = self.ctx.clone();
                read.connect_clicked(move |_| {
                    if let Some((b, c)) = &first {
                        ctx.open_ref(&Ref::new(b, *c, 0));
                    }
                });
                row.add_suffix(&read);
                if Some(i) == next {
                    row.add_css_class("plan-next");
                    next_row = Some(row.clone());
                }
                g.add(&row);
            }
            self.body.append(&g);
            // bring the next day into view
            if let Some(r) = next_row {
                let sc = self.scroll.clone();
                glib::timeout_add_local_once(std::time::Duration::from_millis(150), move || {
                    if let Some(p) =
                        r.compute_point(&sc.child().unwrap(), &gtk::graphene::Point::new(0.0, 0.0))
                    {
                        sc.vadjustment().set_value((p.y() as f64 - 160.0).max(0.0));
                    }
                });
            }
        }
        let g = adw::PreferencesGroup::new();
        g.set_title(if state.is_some() {
            "Other plans"
        } else {
            "Choose a plan"
        });
        g.set_description(Some(
            "Plans start today. Miss a day and simply carry on: your progress is kept, not judged.",
        ));
        let current = state.map(|s| s.id).unwrap_or_default();
        for p in PLANS.iter().filter(|p| p.id != current) {
            let n = plans::days(p.id).len();
            let row = adw::ActionRow::builder()
                .title(p.title)
                .subtitle(format!("{} · {n} days", p.description))
                .subtitle_lines(3)
                .build();
            let b = gtk::Button::with_label(if current.is_empty() {
                "Start"
            } else {
                "Switch"
            });
            b.set_valign(gtk::Align::Center);
            if current.is_empty() {
                b.add_css_class("suggested-action");
            }
            let ctx = self.ctx.clone();
            let id = p.id;
            b.connect_clicked(move |_| start(&ctx, id));
            row.add_suffix(&b);
            g.add(&row);
        }
        self.body.append(&g);
    }
}

impl Section for PlansView {
    fn id(&self) -> &'static str {
        "plans"
    }
    fn title(&self) -> &'static str {
        "Reading plans"
    }
    fn icon(&self) -> &'static str {
        "x-office-calendar-symbolic"
    }
    fn widget(&self) -> gtk::Widget {
        self.root.clone().upcast()
    }
    fn refresh(&self) {
        self.render();
    }
}
