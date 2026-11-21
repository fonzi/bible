//! Updates: once a day (if allowed in Preferences) ask GitHub for the latest
//! release. When a newer version exists, a banner offers to update: the new
//! .deb is downloaded and installed with the system's own password prompt,
//! then the app offers to restart.

use crate::ctx::Ctx;
use adw::prelude::*;
use gtk::{gio, glib};
use std::rc::Rc;

const LATEST: &str = "https://api.github.com/repos/fonzi/bible/releases/latest";
pub const CURRENT: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Debug)]
pub struct Release {
    pub version: String,
    pub deb_url: Option<String>,
    pub page: String,
}

/// "0.6.10" → (0, 6, 10); anything unparsable counts as 0.
fn parts(v: &str) -> (u32, u32, u32) {
    let mut it = v.trim_start_matches('v').split(['.', '-']).map(|p| p.parse::<u32>().unwrap_or(0));
    (it.next().unwrap_or(0), it.next().unwrap_or(0), it.next().unwrap_or(0))
}

pub fn is_newer(candidate: &str, current: &str) -> bool {
    parts(candidate) > parts(current)
}

/// Ask GitHub for the latest release (blocking; run off the main thread).
pub fn fetch_latest() -> Result<Release, String> {
    // for testing the banner: BIBLE_APP_FAKE_UPDATE=0.9.0
    if let Ok(v) = std::env::var("BIBLE_APP_FAKE_UPDATE") {
        return Ok(Release { version: v, deb_url: None, page: "https://github.com/fonzi/bible/releases".into() });
    }
    let bytes = bible_sword::install::get_bytes(LATEST)
        .map_err(|e| e.to_string())?
        .ok_or("no releases yet")?;
    let j: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    let tag = j["tag_name"].as_str().ok_or("no version in the release")?;
    let deb_url = j["assets"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|a| a["name"].as_str().is_some_and(|n| n.ends_with("_amd64.deb")))
        .and_then(|a| a["browser_download_url"].as_str())
        .map(str::to_string);
    Ok(Release {
        version: tag.trim_start_matches('v').to_string(),
        deb_url,
        page: j["html_url"].as_str().unwrap_or("https://github.com/fonzi/bible/releases").to_string(),
    })
}

/// Download the package and install it with the system password prompt.
fn install(url: &str) -> Result<(), String> {
    let dir = dirs::cache_dir().unwrap_or_else(std::env::temp_dir).join("bible-app/update");
    let _ = std::fs::create_dir_all(&dir);
    let file = dir.join(url.rsplit('/').next().unwrap_or("bible-app.deb"));
    bible_sword::install::download_file(url, &file, &mut |_, _| {}).map_err(|e| format!("download failed: {e}"))?;
    let status = std::process::Command::new("pkexec")
        .args(["apt-get", "install", "-y", "--allow-downgrades"])
        .arg(&file)
        .status()
        .map_err(|e| format!("couldn't start the installer: {e}"))?;
    let _ = std::fs::remove_file(&file);
    match status.code() {
        Some(0) => Ok(()),
        Some(126) | Some(127) => Err("the password prompt was cancelled".into()),
        _ => Err("the installer reported a problem".into()),
    }
}

/// Check now if allowed and due (once a day), or always when `force`.
/// `report` gets a line for Preferences ("You have the latest version", …).
pub fn check(ctx: &Rc<Ctx>, banner: &adw::Banner, force: bool, report: Option<Box<dyn Fn(String)>>) {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    {
        let cfg = ctx.config.borrow();
        if !force && (!cfg.check_updates || cfg.update_checked == today) {
            return;
        }
    }
    ctx.config.borrow_mut().update_checked = today;
    ctx.save_config();
    let banner = banner.clone();
    let ctx = ctx.clone();
    glib::spawn_future_local(async move {
        let res = gio::spawn_blocking(fetch_latest).await.unwrap_or(Err("check failed".into()));
        match res {
            Ok(r) if is_newer(&r.version, CURRENT) => {
                if let Some(f) = &report {
                    f(format!("Version {} is available", r.version));
                }
                offer(&ctx, &banner, r);
            }
            Ok(_) => {
                if let Some(f) = &report {
                    f(format!("You have the latest version ({CURRENT})"));
                }
            }
            Err(e) => {
                crate::log::info(&format!("update check: {e}"));
                if let Some(f) = &report {
                    f("Couldn’t check. Are you online?".into());
                }
            }
        }
    });
}

fn offer(_ctx: &Rc<Ctx>, banner: &adw::Banner, r: Release) {
    banner.set_title(&format!("Bible {} is available. You have {CURRENT}.", r.version));
    let can_install = r.deb_url.is_some() && glib::find_program_in_path("pkexec").is_some();
    banner.set_button_label(Some(if can_install { "Update" } else { "Download" }));
    banner.set_revealed(true);
    // 0: offering, 1: updating, 2: updated and ready to restart
    let state = Rc::new(std::cell::Cell::new(0u8));
    banner.connect_button_clicked(move |b| match state.get() {
        2 => restart(),
        1 => {}
        _ if !can_install => {
            gtk::UriLauncher::new(&r.page).launch(gtk::Window::NONE, gio::Cancellable::NONE, |_| {});
            b.set_revealed(false);
        }
        _ => {
            state.set(1);
            b.set_title(&format!("Updating to {}…", r.version));
            b.set_button_label(None);
            let url = r.deb_url.clone().unwrap_or_default();
            let (b, version, state) = (b.clone(), r.version.clone(), state.clone());
            glib::spawn_future_local(async move {
                let res = gio::spawn_blocking(move || install(&url))
                    .await
                    .unwrap_or(Err("update failed".into()));
                match res {
                    Ok(()) => {
                        state.set(2);
                        b.set_title(&format!("Updated to {version}. Restart to use it."));
                        b.set_button_label(Some("Restart"));
                    }
                    Err(e) => {
                        crate::log::error(&format!("update: {e}"));
                        state.set(0);
                        b.set_title(&format!("Couldn’t update: {e}."));
                        b.set_button_label(Some("Try again"));
                    }
                }
            });
        }
    });
}

/// Start the newly installed app and close this one.
fn restart() {
    if let Ok(exe) = std::env::current_exe() {
        let exe = std::path::PathBuf::from(exe.to_string_lossy().trim_end_matches(" (deleted)"));
        let _ = std::process::Command::new(exe).spawn();
    }
    if let Some(app) = gio::Application::default() {
        app.quit();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_versions_numerically() {
        assert!(is_newer("0.6.2", "0.6.1"));
        assert!(is_newer("v0.10.0", "0.9.9"));
        assert!(is_newer("1.0.0", "0.99.99"));
        assert!(!is_newer("0.6.1", "0.6.1"));
        assert!(!is_newer("0.5.9", "0.6.0"));
        assert!(!is_newer("garbage", "0.1.0"));
    }
}
