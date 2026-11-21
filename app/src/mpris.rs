//! MPRIS media controls, so chapter audio shows in GNOME's notification list
//! (and on the lock screen) with a cover, play/pause and next/previous chapter.

use gtk::gio;
use gtk::gio::prelude::*;
use gtk::glib::{self, variant::ToVariant, Variant, VariantDict};
use gtk::{cairo, pango};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;

const PATH: &str = "/org/mpris/MediaPlayer2";

const XML: &str = r#"<node>
  <interface name="org.mpris.MediaPlayer2">
    <method name="Raise"/>
    <method name="Quit"/>
    <property name="CanQuit" type="b" access="read"/>
    <property name="CanRaise" type="b" access="read"/>
    <property name="HasTrackList" type="b" access="read"/>
    <property name="Identity" type="s" access="read"/>
    <property name="DesktopEntry" type="s" access="read"/>
    <property name="SupportedUriSchemes" type="as" access="read"/>
    <property name="SupportedMimeTypes" type="as" access="read"/>
  </interface>
  <interface name="org.mpris.MediaPlayer2.Player">
    <method name="Next"/>
    <method name="Previous"/>
    <method name="Pause"/>
    <method name="PlayPause"/>
    <method name="Stop"/>
    <method name="Play"/>
    <method name="Seek"><arg type="x" name="Offset" direction="in"/></method>
    <method name="SetPosition"><arg type="o" name="TrackId" direction="in"/><arg type="x" name="Position" direction="in"/></method>
    <method name="OpenUri"><arg type="s" name="Uri" direction="in"/></method>
    <signal name="Seeked"><arg type="x" name="Position"/></signal>
    <property name="PlaybackStatus" type="s" access="read"/>
    <property name="Rate" type="d" access="read"/>
    <property name="Metadata" type="a{sv}" access="read"/>
    <property name="Volume" type="d" access="read"/>
    <property name="Position" type="x" access="read"/>
    <property name="MinimumRate" type="d" access="read"/>
    <property name="MaximumRate" type="d" access="read"/>
    <property name="CanGoNext" type="b" access="read"/>
    <property name="CanGoPrevious" type="b" access="read"/>
    <property name="CanPlay" type="b" access="read"/>
    <property name="CanPause" type="b" access="read"/>
    <property name="CanSeek" type="b" access="read"/>
    <property name="CanControl" type="b" access="read"/>
  </interface>
</node>"#;

#[derive(Clone, Default)]
pub struct Track {
    pub title: String,
    pub artist: String,
    pub album: String,
    /// microseconds
    pub length: i64,
}

/// What the media keys and the GNOME panel can ask the player to do.
pub enum Command {
    PlayPause,
    Play,
    Pause,
    Next,
    Previous,
    Raise,
}

struct State {
    conn: Option<gio::DBusConnection>,
    status: &'static str,
    track: Track,
    art: Option<PathBuf>,
    position: Box<dyn Fn() -> i64>,
    handler: Option<Box<dyn Fn(Command)>>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State {
        conn: None,
        status: "Stopped",
        track: Track::default(),
        art: None,
        position: Box::new(|| 0),
        handler: None,
    });
}

fn metadata(track: &Track, art: &Option<PathBuf>) -> Variant {
    let d = VariantDict::new(None);
    d.insert_value("mpris:trackid", &glib::variant::ObjectPath::try_from("/io/github/fonzi/Bible/track/1").unwrap().to_variant());
    d.insert("xesam:title", track.title.as_str());
    d.insert_value("xesam:artist", &vec![track.artist.clone()].to_variant());
    d.insert("xesam:album", track.album.as_str());
    if track.length > 0 {
        d.insert("mpris:length", track.length);
    }
    if let Some(a) = art {
        d.insert("mpris:artUrl", gio::File::for_path(a).uri().as_str());
    }
    d.end()
}

fn property(iface: &str, name: &str) -> Option<Variant> {
    STATE.with(|s| {
        let s = s.borrow();
        Some(match (iface, name) {
            (_, "CanQuit") => false.to_variant(),
            (_, "CanRaise") => true.to_variant(),
            (_, "HasTrackList") => false.to_variant(),
            (_, "Identity") => "Bible".to_variant(),
            (_, "DesktopEntry") => "io.github.fonzi.Bible".to_variant(),
            (_, "SupportedUriSchemes") | (_, "SupportedMimeTypes") => Vec::<String>::new().to_variant(),
            (_, "PlaybackStatus") => s.status.to_variant(),
            (_, "Rate") | (_, "MinimumRate") | (_, "MaximumRate") | (_, "Volume") => 1.0f64.to_variant(),
            (_, "Metadata") => metadata(&s.track, &s.art),
            (_, "Position") => (s.position)().to_variant(),
            (_, "CanGoNext") | (_, "CanGoPrevious") | (_, "CanPlay") | (_, "CanPause") | (_, "CanControl") => true.to_variant(),
            (_, "CanSeek") => false.to_variant(),
            _ => return None,
        })
    })
}

/// Publish the media player on the session bus.
pub fn register(app: &adw::Application, position: impl Fn() -> i64 + 'static, handler: impl Fn(Command) + 'static) {
    let Some(conn) = gio::prelude::ApplicationExt::dbus_connection(app) else { return };
    let Ok(node) = gio::DBusNodeInfo::for_xml(XML) else { return };
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.position = Box::new(position);
        s.handler = Some(Box::new(handler));
        s.conn = Some(conn.clone());
    });
    for name in ["org.mpris.MediaPlayer2", "org.mpris.MediaPlayer2.Player"] {
        let Some(iface) = node.lookup_interface(name) else { continue };
        let res = conn
            .register_object(PATH, &iface)
            .method_call(|_, _, _, _, method, params, inv| {
                let cmd = match method {
                    "PlayPause" => Some(Command::PlayPause),
                    "Play" => Some(Command::Play),
                    "Pause" | "Stop" => Some(Command::Pause),
                    "Next" => Some(Command::Next),
                    "Previous" => Some(Command::Previous),
                    "Raise" => Some(Command::Raise),
                    _ => None,
                };
                let _ = params;
                inv.return_value(None);
                if let Some(c) = cmd {
                    // run outside the D-Bus call, on the main loop
                    glib::idle_add_local_once(move || STATE.with(|s| {
                        if let Some(h) = s.borrow().handler.as_ref() {
                            h(c);
                        }
                    }));
                }
            })
            .property(|_, _, _, iface, prop| property(iface, prop).unwrap_or_else(|| false.to_variant()))
            .build();
        if let Err(e) = res {
            crate::log::error(&format!("mpris: {e}"));
            return;
        }
    }
    // a bus name per instance, as the MPRIS spec asks
    gio::bus_own_name_on_connection(&conn, "org.mpris.MediaPlayer2.bibleapp", gio::BusNameOwnerFlags::NONE, |_, _| {}, |_, _| {});
}

fn changed(props: &[(&str, Variant)]) {
    STATE.with(|s| {
        let s = s.borrow();
        let Some(conn) = s.conn.as_ref() else { return };
        let map: HashMap<String, Variant> = props.iter().map(|(k, v)| (k.to_string(), v.clone())).collect();
        let args = ("org.mpris.MediaPlayer2.Player", map, Vec::<String>::new()).to_variant();
        let _ = conn.emit_signal(None, PATH, "org.freedesktop.DBus.Properties", "PropertiesChanged", Some(&args));
    });
}

/// Set what is playing (draws a cover for it).
pub fn set_track(track: Track) {
    let art = cover(&track.title, &track.album);
    let meta = metadata(&track, &art);
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.track = track;
        s.art = art;
    });
    changed(&[("Metadata", meta)]);
}

/// "Playing", "Paused" or "Stopped".
pub fn set_status(status: &'static str) {
    let differs = STATE.with(|s| {
        let mut s = s.borrow_mut();
        let d = s.status != status;
        s.status = status;
        d
    });
    if differs {
        changed(&[("PlaybackStatus", status.to_variant())]);
    }
}

/// A square cover: the chapter on a deep blue field with an open book.
fn cover(title: &str, album: &str) -> Option<PathBuf> {
    let dir = dirs::cache_dir()?.join("bible-app/covers");
    std::fs::create_dir_all(&dir).ok()?;
    let safe: String = title.chars().map(|c| if c.is_alphanumeric() { c } else { '_' }).collect();
    let path = dir.join(format!("{safe}.png"));
    if path.exists() {
        return Some(path);
    }
    let s = 512.0;
    let surf = cairo::ImageSurface::create(cairo::Format::ARgb32, s as i32, s as i32).ok()?;
    let cr = cairo::Context::new(&surf).ok()?;
    let g = cairo::LinearGradient::new(0.0, 0.0, 0.0, s);
    g.add_color_stop_rgb(0.0, 0.21, 0.52, 0.89);
    g.add_color_stop_rgb(1.0, 0.10, 0.30, 0.62);
    let _ = cr.set_source(&g);
    let _ = cr.paint();
    // open book
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.95);
    cr.set_line_width(14.0);
    cr.set_line_join(cairo::LineJoin::Round);
    let (cx, top) = (s / 2.0, 110.0);
    cr.move_to(cx, top + 30.0);
    cr.curve_to(cx - 50.0, top, cx - 110.0, top, cx - 150.0, top + 18.0);
    cr.line_to(cx - 150.0, top + 190.0);
    cr.curve_to(cx - 110.0, top + 172.0, cx - 50.0, top + 172.0, cx, top + 202.0);
    cr.curve_to(cx + 50.0, top + 172.0, cx + 110.0, top + 172.0, cx + 150.0, top + 190.0);
    cr.line_to(cx + 150.0, top + 18.0);
    cr.curve_to(cx + 110.0, top, cx + 50.0, top, cx, top + 30.0);
    cr.line_to(cx, top + 202.0);
    let _ = cr.stroke();
    let layout = pangocairo::functions::create_layout(&cr);
    layout.set_font_description(Some(&pango::FontDescription::from_string("EB Garamond, Cantarell, serif Bold 54")));
    layout.set_text(title);
    let (tw, _) = layout.pixel_size();
    cr.set_source_rgb(1.0, 1.0, 1.0);
    cr.move_to((s - tw as f64) / 2.0, 345.0);
    pangocairo::functions::show_layout(&cr, &layout);
    layout.set_font_description(Some(&pango::FontDescription::from_string("Cantarell, sans 22")));
    layout.set_text(album);
    let (tw, _) = layout.pixel_size();
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.8);
    cr.move_to((s - tw as f64) / 2.0, 430.0);
    pangocairo::functions::show_layout(&cr, &layout);
    drop(cr);
    let mut f = std::fs::File::create(&path).ok()?;
    surf.write_to_png(&mut f).ok()?;
    Some(path)
}
