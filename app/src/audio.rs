//! Chapter audio: a slim player bar under the reader, downloads a chapter
//! (or a whole book) at a time, and plays what has been downloaded.

use crate::ctx::Ctx;
use adw::prelude::*;
use bible_core::audio::{self, AudioSource};
use gstreamer as gst;
use gstreamer::prelude::*;
use gtk::{gio, glib};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Audio playback through GStreamer's classic `playbin` (audio only).
/// GTK's media player uses decodebin3, which aborts the whole program on some
/// MP3 files in GStreamer 1.24 ("mq_slot_handle_stream_start: assertion failed").
pub struct Playback {
    bin: gst::Element,
    _watch: Option<gst::bus::BusWatchGuard>,
    timer: RefCell<Option<glib::SourceId>>,
    playing: Cell<bool>,
    ended: Cell<bool>,
}

impl Playback {
    fn open(path: &std::path::Path, on_tick: impl Fn() + 'static, on_ended: impl Fn() + 'static, on_error: impl Fn(String) + 'static) -> Option<Rc<Playback>> {
        if let Err(e) = gst::init() {
            crate::log::error(&format!("audio: GStreamer unavailable: {e}"));
            return None;
        }
        let bin = gst::ElementFactory::make("playbin").build().ok()?;
        let uri = gio::File::for_path(path).uri();
        bin.set_property("uri", uri.as_str());
        // audio only: cover art in the MP3 must not open a video sink
        bin.set_property_from_str("flags", "audio+soft-volume");
        let bus = bin.bus()?;
        let tick = Rc::new(on_tick);
        let pb = Rc::new_cyclic(|weak: &std::rc::Weak<Playback>| {
            let w = weak.clone();
            let watch = bus
                .add_watch_local(move |_, msg| {
                    use gst::MessageView;
                    let Some(pb) = w.upgrade() else { return glib::ControlFlow::Break };
                    match msg.view() {
                        MessageView::Eos(_) => {
                            pb.playing.set(false);
                            pb.ended.set(true);
                            pb.stop_timer();
                            on_ended();
                        }
                        MessageView::Error(e) => {
                            pb.playing.set(false);
                            pb.stop_timer();
                            let _ = pb.bin.set_state(gst::State::Null);
                            on_error(format!("{} ({:?})", e.error(), e.debug()));
                        }
                        _ => {}
                    }
                    glib::ControlFlow::Continue
                })
                .ok();
            Playback { bin, _watch: watch, timer: RefCell::new(None), playing: Cell::new(false), ended: Cell::new(false) }
        });
        // position updates while playing
        let w = Rc::downgrade(&pb);
        let t2 = tick.clone();
        *pb.timer.borrow_mut() = Some(glib::timeout_add_local(std::time::Duration::from_millis(250), move || {
            if w.upgrade().is_none() {
                return glib::ControlFlow::Break;
            }
            t2();
            glib::ControlFlow::Continue
        }));
        let _ = pb.bin.set_state(gst::State::Paused);
        Some(pb)
    }

    fn stop_timer(&self) {}

    pub fn is_playing(&self) -> bool {
        self.playing.get()
    }

    pub fn is_ended(&self) -> bool {
        self.ended.get()
    }

    pub fn play(&self) {
        self.ended.set(false);
        if self.bin.set_state(gst::State::Playing).is_ok() {
            self.playing.set(true);
        }
    }

    pub fn pause(&self) {
        let _ = self.bin.set_state(gst::State::Paused);
        self.playing.set(false);
    }

    /// microseconds
    pub fn seek(&self, us: i64) {
        self.ended.set(false);
        let _ = self.bin.seek_simple(gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT, gst::ClockTime::from_useconds(us.max(0) as u64));
    }

    /// microseconds
    pub fn timestamp(&self) -> i64 {
        self.bin.query_position::<gst::ClockTime>().map(|t| t.useconds() as i64).unwrap_or(0)
    }

    /// microseconds, 0 if not known yet
    pub fn duration(&self) -> i64 {
        self.bin.query_duration::<gst::ClockTime>().map(|t| t.useconds() as i64).unwrap_or(0)
    }
}

impl Drop for Playback {
    fn drop(&mut self) {
        if let Some(t) = self.timer.borrow_mut().take() {
            t.remove();
        }
        let _ = self.bin.set_state(gst::State::Null);
    }
}

pub struct Player {
    ctx: Rc<Ctx>,
    /// The bar, revealed under the text
    pub root: gtk::Revealer,
    /// "You can listen to this chapter" tip, shown above the text
    pub tip: gtk::Revealer,
    play: gtk::Button,
    title: gtk::Label,
    sub: gtk::Label,
    scale: gtk::Scale,
    time: gtk::Label,
    dl: gtk::Button,
    dl_book: gtk::Button,
    progress: gtk::ProgressBar,
    controls: gtk::Box,
    continuous: gtk::ToggleButton,
    media: RefCell<Option<Rc<Playback>>>,
    /// the downloaded file for the current chapter, opened only when played
    pending: RefCell<Option<std::path::PathBuf>>,
    /// (source id, book, chapter, chapters in book)
    current: RefCell<(String, String, u16, u16)>,
    /// set when a chapter ended and the next should start by itself
    autoplay: Cell<bool>,
    seeking: Cell<bool>,
    busy: Cell<bool>,
    on_next: RefCell<Option<Box<dyn Fn()>>>,
    on_prev: RefCell<Option<Box<dyn Fn()>>>,
    me: RefCell<std::rc::Weak<Player>>,
}

fn clock(us: i64) -> String {
    let s = (us / 1_000_000).max(0);
    format!("{}:{:02}", s / 60, s % 60)
}

impl Player {
    pub fn new(ctx: &Rc<Ctx>) -> Rc<Player> {
        let play = gtk::Button::from_icon_name("media-playback-start-symbolic");
        play.add_css_class("circular");
        play.add_css_class("suggested-action");
        play.set_valign(gtk::Align::Center);
        play.set_tooltip_text(Some("Play"));
        let back = gtk::Button::from_icon_name("media-skip-backward-symbolic");
        back.add_css_class("flat");
        back.set_valign(gtk::Align::Center);
        back.set_tooltip_text(Some("Back 15 seconds"));
        let fwd = gtk::Button::from_icon_name("media-skip-forward-symbolic");
        fwd.add_css_class("flat");
        fwd.set_valign(gtk::Align::Center);
        fwd.set_tooltip_text(Some("Forward 15 seconds"));

        let title = gtk::Label::new(None);
        title.set_xalign(0.0);
        title.add_css_class("heading");
        title.set_ellipsize(gtk::pango::EllipsizeMode::End);
        title.set_max_width_chars(18);
        let sub = gtk::Label::new(None);
        sub.set_xalign(0.0);
        sub.add_css_class("caption");
        sub.add_css_class("dim-label");
        sub.set_ellipsize(gtk::pango::EllipsizeMode::End);
        sub.set_max_width_chars(24);
        sub.set_width_chars(8);
        title.set_width_chars(8);
        let labels = gtk::Box::new(gtk::Orientation::Vertical, 0);
        labels.set_valign(gtk::Align::Center);
        labels.set_hexpand(false);
        labels.set_width_request(120);
        labels.append(&title);
        labels.append(&sub);

        let scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, 1.0, 0.001);
        scale.set_draw_value(false);
        scale.set_hexpand(true);
        scale.set_width_request(140);
        scale.set_valign(gtk::Align::Center);
        let time = gtk::Label::new(Some("0:00"));
        time.add_css_class("numeric");
        time.add_css_class("caption");
        time.set_width_chars(9);

        let controls = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        controls.set_hexpand(true);
        controls.append(&scale);
        controls.append(&time);

        let dl = gtk::Button::with_label("Download");
        dl.set_tooltip_text(Some("Download this chapter’s audio"));
        dl.add_css_class("suggested-action");
        dl.set_valign(gtk::Align::Center);
        let dl_book = gtk::Button::with_label("Whole book");
        dl_book.set_valign(gtk::Align::Center);
        dl_book.set_tooltip_text(Some("Download every chapter of this book"));
        let progress = gtk::ProgressBar::new();
        progress.set_valign(gtk::Align::Center);
        progress.set_hexpand(true);
        progress.set_show_text(true);
        progress.set_width_request(80);
        progress.set_visible(false);

        let continuous = gtk::ToggleButton::new();
        continuous.set_icon_name("media-playlist-consecutive-symbolic");
        continuous.add_css_class("flat");
        continuous.set_valign(gtk::Align::Center);
        continuous.set_tooltip_text(Some("Keep playing into the next chapter"));
        continuous.set_active(true);
        let close = gtk::Button::from_icon_name("window-close-symbolic");
        close.add_css_class("flat");
        close.add_css_class("circular");
        close.set_valign(gtk::Align::Center);
        close.set_tooltip_text(Some("Close the player"));

        let bar = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        bar.add_css_class("player-bar");
        bar.append(&back);
        bar.append(&play);
        bar.append(&fwd);
        bar.append(&labels);
        bar.append(&controls);
        bar.append(&progress);
        bar.append(&dl);
        bar.append(&dl_book);
        bar.append(&continuous);
        bar.append(&close);
        let root = gtk::Revealer::builder()
            .child(&bar)
            .transition_type(gtk::RevealerTransitionType::SlideUp)
            .build();

        // the listen-along tip
        let tip_label = gtk::Label::new(None);
        tip_label.set_markup("<b>You can listen along.</b> Download this chapter’s audio and it plays here, offline.");
        tip_label.set_wrap(true);
        tip_label.set_xalign(0.0);
        tip_label.set_hexpand(true);
        let tip_btn = gtk::Button::with_label("Listen");
        tip_btn.add_css_class("pill");
        tip_btn.set_valign(gtk::Align::Center);
        let tip_close = gtk::Button::from_icon_name("window-close-symbolic");
        tip_close.add_css_class("flat");
        tip_close.add_css_class("circular");
        tip_close.set_tooltip_text(Some("Dismiss"));
        let tip_row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        tip_row.add_css_class("tip-bar");
        tip_row.append(&gtk::Image::from_icon_name("audio-headphones-symbolic"));
        tip_row.append(&tip_label);
        tip_row.append(&tip_btn);
        tip_row.append(&tip_close);
        let tip = gtk::Revealer::builder()
            .child(&tip_row)
            .transition_type(gtk::RevealerTransitionType::SlideDown)
            .build();

        let p = Rc::new(Player {
            ctx: ctx.clone(),
            root,
            tip,
            play,
            title,
            sub,
            scale,
            time,
            dl,
            dl_book,
            progress,
            controls,
            continuous,
            media: RefCell::new(None),
            pending: RefCell::new(None),
            current: RefCell::new((String::new(), String::new(), 0, 0)),
            autoplay: Cell::new(false),
            seeking: Cell::new(false),
            busy: Cell::new(false),
            on_next: RefCell::new(None),
            on_prev: RefCell::new(None),
            me: RefCell::new(std::rc::Weak::new()),
        });
        *p.me.borrow_mut() = Rc::downgrade(&p);
        p.register_mpris();

        let w = Rc::downgrade(&p);
        p.play.connect_clicked(move |_| {
            if let Some(p) = w.upgrade() {
                p.toggle();
            }
        });
        let w = Rc::downgrade(&p);
        back.connect_clicked(move |_| {
            if let Some(p) = w.upgrade() {
                p.skip(-15);
            }
        });
        let w = Rc::downgrade(&p);
        fwd.connect_clicked(move |_| {
            if let Some(p) = w.upgrade() {
                p.skip(15);
            }
        });
        let w = Rc::downgrade(&p);
        p.scale.connect_change_value(move |_, _, v| {
            if let Some(p) = w.upgrade() {
                if let Some(m) = p.media.borrow().as_ref() {
                    if !p.seeking.get() && m.duration() > 0 {
                        m.seek((v.clamp(0.0, 1.0) * m.duration() as f64) as i64);
                    }
                }
            }
            glib::Propagation::Proceed
        });
        let w = Rc::downgrade(&p);
        p.dl.connect_clicked(move |_| {
            if let Some(p) = w.upgrade() {
                p.download(false);
            }
        });
        let w = Rc::downgrade(&p);
        p.dl_book.connect_clicked(move |_| {
            if let Some(p) = w.upgrade() {
                p.download(true);
            }
        });
        let w = Rc::downgrade(&p);
        close.connect_clicked(move |_| {
            if let Some(p) = w.upgrade() {
                p.hide();
            }
        });
        let w = Rc::downgrade(&p);
        tip_btn.connect_clicked(move |_| {
            if let Some(p) = w.upgrade() {
                p.dismiss_tip();
                p.show();
            }
        });
        let w = Rc::downgrade(&p);
        tip_close.connect_clicked(move |_| {
            if let Some(p) = w.upgrade() {
                p.dismiss_tip();
            }
        });
        p
    }

    pub fn set_on_next(&self, f: impl Fn() + 'static) {
        *self.on_next.borrow_mut() = Some(Box::new(f));
    }

    pub fn set_on_prev(&self, f: impl Fn() + 'static) {
        *self.on_prev.borrow_mut() = Some(Box::new(f));
    }

    /// Hook up GNOME's media controls (MPRIS) once the app is on the bus.
    fn register_mpris(self: &Rc<Self>) {
        let Some(app) = gio::Application::default().and_downcast::<adw::Application>() else { return };
        let w = Rc::downgrade(self);
        let w2 = Rc::downgrade(self);
        crate::mpris::register(
            &app,
            move || w.upgrade().and_then(|p| p.media.borrow().as_ref().map(|m| m.timestamp())).unwrap_or(0),
            move |cmd| {
                let Some(p) = w2.upgrade() else { return };
                use crate::mpris::Command as C;
                match cmd {
                    C::PlayPause => p.toggle(),
                    C::Play => p.start(),
                    C::Pause => {
                        if p.media.borrow().as_ref().is_some_and(|m| m.is_playing()) {
                            p.toggle();
                        }
                    }
                    C::Next | C::Previous => {
                        p.autoplay.set(true);
                        let f = if matches!(cmd, C::Next) { &p.on_next } else { &p.on_prev };
                        if let Some(f) = f.borrow().as_ref() {
                            f();
                        }
                    }
                    C::Raise => {
                        if let Some(w) = p.ctx.window.get() {
                            w.present();
                        }
                    }
                }
            },
        );
    }

    pub fn is_shown(&self) -> bool {
        self.root.reveals_child()
    }

    pub fn show(&self) {
        self.root.set_reveal_child(true);
        self.tip.set_reveal_child(false);
    }

    pub fn hide(&self) {
        let m = self.media.borrow().clone();
        if let Some(m) = m {
            m.pause();
        }
        self.root.set_reveal_child(false);
    }

    fn dismiss_tip(&self) {
        self.tip.set_reveal_child(false);
        let mut c = self.ctx.config.borrow_mut();
        if !c.dismissed_tips.iter().any(|t| t == "audio") {
            c.dismissed_tips.push("audio".into());
        }
        drop(c);
        self.ctx.save_config();
    }

    fn source(&self) -> &'static AudioSource {
        audio::source(&self.current.borrow().0).unwrap_or(&audio::SOURCES[0])
    }

    /// Follow the reader to a chapter. `module` is the text being read.
    pub fn set_chapter(
        &self,
        module: &str,
        book: &str,
        chapter: u16,
        chapters: u16,
        tip_allowed: bool,
    ) {
        let src = audio::for_version(module).unwrap_or(&audio::SOURCES[0]);
        let same = {
            let c = self.current.borrow();
            c.0 == src.id && c.1 == book && c.2 == chapter
        };
        *self.current.borrow_mut() = (src.id.to_string(), book.to_string(), chapter, chapters);
        let available = src.url(book, chapter).is_some();
        let cfg = self.ctx.config.borrow();
        let show_tip = tip_allowed
            && available
            && cfg.feature_tips
            && !cfg.dismissed_tips.iter().any(|t| t == "audio")
            && !self.is_shown();
        drop(cfg);
        self.tip.set_reveal_child(show_tip);
        let name = bible_core::Ref::new(book, chapter, 0).book_name();
        self.title.set_text(&format!("{name} {chapter}"));
        let note = if src.reads(module) {
            String::new()
        } else {
            format!(" · reads the {}", src.versions[0])
        };
        self.sub
            .set_text(&format!("{}, read by {}{note}", src.title, src.narrator));
        if same && (self.media.borrow().is_some() || self.pending.borrow().is_some()) {
            return;
        }
        self.load(available);
    }

    /// Show the current chapter's state: ready to play, or offer to download it.
    /// The audio file itself is opened only when playback starts.
    fn load(&self, available: bool) {
        let old = self.media.borrow_mut().take();
        if let Some(m) = old {
            m.pause();
        }
        let (_, book, chapter, _) = self.current.borrow().clone();
        let src = self.source();
        let path = audio::path(src, &book, chapter);
        let have = path.is_file();
        self.controls.set_visible(have);
        self.play.set_sensitive(have);
        self.dl.set_visible(!have && available && !self.busy.get());
        self.dl_book.set_visible(!have && available && !self.busy.get());
        if !available {
            self.sub.set_text("No recording of this book yet");
        }
        self.set_playing_icon(false);
        self.scale.set_value(0.0);
        self.time.set_text("0:00");
        *self.pending.borrow_mut() = have.then_some(path);
        if !have {
            self.autoplay.set(false);
            return;
        }
        if self.autoplay.replace(false) {
            self.start();
        }
    }

    /// The media for the current chapter, opening the file on first use.
    fn media(&self) -> Option<Rc<Playback>> {
        if let Some(m) = self.media.borrow().as_ref() {
            return Some(m.clone());
        }
        let path = self.pending.borrow().clone()?;
        crate::log::info(&format!("audio: opening {}", path.display()));
        let w1 = self.me.borrow().clone();
        let w2 = self.me.borrow().clone();
        let w3 = self.me.borrow().clone();
        let m = Playback::open(
            &path,
            move || {
                if let Some(p) = w1.upgrade() {
                    p.tick();
                }
            },
            move || {
                let Some(p) = w2.upgrade() else { return };
                p.set_playing_icon(false);
                if p.continuous.is_active() {
                    p.autoplay.set(true);
                    let pw = p.me.borrow().clone();
                    glib::idle_add_local_once(move || {
                        if let Some(p) = pw.upgrade() {
                            if let Some(f) = p.on_next.borrow().as_ref() {
                                f();
                            }
                        }
                    });
                }
            },
            move |e| {
                if let Some(p) = w3.upgrade() {
                    crate::log::error(&format!("audio: playback error: {e}"));
                    p.ctx.toast("Couldn’t play this audio file");
                    p.set_playing_icon(false);
                }
            },
        )?;
        *self.media.borrow_mut() = Some(m.clone());
        let (_, book, chapter, _) = self.current.borrow().clone();
        let src = self.source();
        crate::mpris::set_track(crate::mpris::Track {
            title: format!("{} {chapter}", bible_core::Ref::new(&book, chapter, 0).book_name()),
            artist: src.narrator.to_string(),
            album: src.title.to_string(),
            length: 0,
        });
        Some(m)
    }

    fn start(&self) {
        if let Some(m) = self.media() {
            if m.is_ended() {
                m.seek(0);
            }
            m.play();
            self.set_playing_icon(m.is_playing());
        }
    }

    fn tick(&self) {
        let Some(m) = self.media.borrow().clone() else { return };
        let d = m.duration();
        let t = m.timestamp();
        self.seeking.set(true);
        self.scale
            .set_value(if d > 0 { t as f64 / d as f64 } else { 0.0 });
        self.seeking.set(false);
        self.time.set_text(&format!("{} / {}", clock(t), clock(d)));
    }

    fn set_playing_icon(&self, playing: bool) {
        crate::mpris::set_status(if playing { "Playing" } else if self.media.borrow().is_some() { "Paused" } else { "Stopped" });
        self.play.set_icon_name(if playing {
            "media-playback-pause-symbolic"
        } else {
            "media-playback-start-symbolic"
        });
        self.play
            .set_tooltip_text(Some(if playing { "Pause" } else { "Play" }));
    }

    pub fn toggle(&self) {
        let playing = self.media.borrow().as_ref().is_some_and(|m| m.is_playing());
        if playing {
            if let Some(m) = self.media.borrow().clone() {
                m.pause();
            }
            self.set_playing_icon(false);
        } else {
            self.start();
        }
    }

    /// Start the current chapter (used by the Library's play buttons).
    pub fn play_now(&self) {
        self.show();
        self.start();
    }

    fn skip(&self, secs: i64) {
        let m = self.media.borrow().clone();
        if let Some(m) = m {
            let t = (m.timestamp() + secs * 1_000_000).clamp(0, m.duration().max(0));
            m.seek(t);
        }
    }

    fn download(&self, whole_book: bool) {
        let (sid, book, chapter, chapters) = self.current.borrow().clone();
        let Some(src) = audio::source(&sid) else {
            return;
        };
        let list: Vec<u16> = if whole_book {
            (1..=chapters.max(1))
                .filter(|c| !audio::is_downloaded(src, &book, *c))
                .collect()
        } else {
            vec![chapter]
        };
        if list.is_empty() {
            return;
        }
        self.busy.set(true);
        self.dl.set_visible(false);
        self.dl_book.set_visible(false);
        self.progress.set_visible(true);
        self.progress.set_fraction(0.0);
        self.progress.set_text(Some("Starting…"));
        let (tx, rx) = std::sync::mpsc::channel::<(usize, usize, u64, u64)>();
        let bar = self.progress.clone();
        let tick = glib::timeout_add_local(std::time::Duration::from_millis(120), move || {
            while let Ok((i, n, d, t)) = rx.try_recv() {
                let part = if t > 0 { d as f64 / t as f64 } else { 0.0 };
                bar.set_fraction((i as f64 + part) / n as f64);
                bar.set_text(Some(&if n > 1 {
                    format!("Chapter {} of {n}", i + 1)
                } else {
                    format!(
                        "{} of {}",
                        audio::size(d),
                        if t > 0 { audio::size(t) } else { "?".into() }
                    )
                }));
            }
            glib::ControlFlow::Continue
        });
        let w = self.me.borrow().clone();
        let total = list.len();
        glib::spawn_future_local(async move {
            let b = book.clone();
            let res = gio::spawn_blocking(move || -> Result<usize, String> {
                let mut got = 0;
                for (i, c) in list.iter().enumerate() {
                    let Some(url) = src.url(&b, *c) else { continue };
                    let dest = audio::path(src, &b, *c);
                    bible_sword::install::download_file(&url, &dest, &mut |d, t| {
                        let _ = tx.send((i, total, d, t));
                    })
                    .map_err(|e| {
                        format!("{} {c}: {e}", bible_core::Ref::new(&b, *c, 0).book_name())
                    })?;
                    got += 1;
                }
                Ok(got)
            })
            .await;
            tick.remove();
            let Some(p) = w.upgrade() else { return };
            p.busy.set(false);
            p.progress.set_visible(false);
            match res {
                Ok(Ok(n)) => {
                    crate::log::info(&format!("audio: downloaded {n} chapter(s) of {book}"));
                    p.ctx.toast(&if n == 1 {
                        "Chapter audio downloaded".to_string()
                    } else {
                        format!("Downloaded {n} chapters")
                    });
                    let first = !whole_book;
                    p.autoplay.set(first);
                    p.load(true);
                    p.ctx.refresh_all();
                }
                Ok(Err(e)) => {
                    crate::log::error(&format!("audio: download failed: {e}"));
                    p.ctx.toast(&format!("Download failed: {e}"));
                    p.load(true);
                }
                Err(_) => p.load(true),
            }
        });
    }
}
