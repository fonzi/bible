//! Opt-in startup sounds, two seconds or less: the opening notes of
//! public-domain hymn tunes synthesised for this app (tools/mksounds.py), and
//! the opening of Kyrie XI (Orbis Factor) sung by Paterm
//! (https://commons.wikimedia.org/wiki/File:Kyrie_XI_(Orbis_Factor)_sample.ogg,
//! CC BY-SA 3.0), shortened to two seconds with a fade.

use gstreamer as gst;
use gstreamer::prelude::*;
use std::cell::RefCell;

/// (id, name shown in Preferences, file)
pub const SOUNDS: &[(&str, &str, &[u8])] = &[
    ("off", "Off", &[]),
    ("old-hundredth", "Old Hundredth, organ (1551)", include_bytes!("../sounds/old-hundredth.ogg")),
    ("amazing-grace", "Amazing Grace, harp (1829)", include_bytes!("../sounds/amazing-grace.ogg")),
    ("holy-holy-holy", "Holy, Holy, Holy, bells (1861)", include_bytes!("../sounds/holy-holy-holy.ogg")),
    ("bell", "A single bell", include_bytes!("../sounds/bell.ogg")),
    // Paterm, Wikimedia Commons, CC BY-SA 3.0; shortened to 2 s with fades
    ("kyrie", "Kyrie, Gregorian chant (sung)", include_bytes!("../sounds/kyrie.ogg")),
];

thread_local! {
    static PLAYING: RefCell<Option<gst::Element>> = const { RefCell::new(None) };
}

/// Play a startup sound by id; does nothing for "off" or if audio fails.
pub fn play(id: &str) {
    let Some((_, _, bytes)) = SOUNDS.iter().find(|s| s.0 == id && !s.2.is_empty()) else {
        return;
    };
    if gst::init().is_err() {
        return;
    }
    let dir = dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("bible-app/sounds");
    let path = dir.join(format!("{id}.ogg"));
    if std::fs::metadata(&path).map(|m| m.len()).ok() != Some(bytes.len() as u64) {
        let _ = std::fs::create_dir_all(&dir);
        if std::fs::write(&path, bytes).is_err() {
            return;
        }
    }
    let Ok(uri) = gtk::glib::filename_to_uri(&path, None) else {
        return;
    };
    let Ok(pb) = gst::ElementFactory::make("playbin").build() else {
        return;
    };
    pb.set_property("uri", uri.as_str());
    // audio only, as in the chapter player
    pb.set_property_from_str("flags", "audio+soft-volume");
    if let Some(bus) = pb.bus() {
        let _ = bus.add_watch_local(|_, msg| {
            if matches!(msg.view(), gst::MessageView::Eos(_) | gst::MessageView::Error(_)) {
                stop();
                return gtk::glib::ControlFlow::Break;
            }
            gtk::glib::ControlFlow::Continue
        });
    }
    if pb.set_state(gst::State::Playing).is_err() {
        crate::log::error("startup sound failed to play");
        return;
    }
    stop();
    PLAYING.with(|p| *p.borrow_mut() = Some(pb));
}

fn stop() {
    PLAYING.with(|p| {
        if let Some(pb) = p.borrow_mut().take() {
            let _ = pb.set_state(gst::State::Null);
        }
    });
}
