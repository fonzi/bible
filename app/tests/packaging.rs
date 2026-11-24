//! The .deb, the Flatpak and the Arch package must install the same files.
//! The .deb's list (app/Cargo.toml) is the reference.

use std::path::Path;

fn read(p: &str) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    std::fs::read_to_string(root.join(p)).unwrap_or_else(|e| panic!("{p}: {e}"))
}

/// File names the .deb installs, from its assets list.
fn deb_files() -> Vec<String> {
    let toml = read("app/Cargo.toml");
    let start = toml.find("assets = [").expect("deb assets");
    let end = start + toml[start..].find("\n]").expect("end of assets");
    toml[start..end]
        .lines()
        .filter_map(|l| l.trim().strip_prefix("[\""))
        .map(|l| l.split('"').next().unwrap())
        .map(|src| src.rsplit('/').next().unwrap().to_string())
        .collect()
}

#[test]
fn deb_lists_the_app_and_its_data() {
    let f = deb_files();
    assert!(f.contains(&"bible-app".to_string()));
    assert!(f.iter().any(|x| x.ends_with(".desktop")));
    assert!(f.iter().any(|x| x.ends_with(".metainfo.xml")));
    assert!(f.len() >= 10, "{f:?}");
}

#[test]
fn flatpak_installs_what_the_deb_installs() {
    let manifest = read("packaging/flatpak/io.github.fonzi.Bible.json");
    let json: serde_json::Value = serde_json::from_str(&manifest).expect("manifest is JSON");
    assert_eq!(json["id"], "io.github.fonzi.Bible");
    assert_eq!(json["command"], "bible-app");
    let cmds: String = json["modules"][0]["build-commands"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c.as_str().unwrap().to_string() + "\n")
        .collect();
    for f in deb_files() {
        assert!(cmds.contains(&f), "the Flatpak doesn't install {f}");
    }
    // the D-Bus service must point inside the Flatpak
    assert!(cmds.contains("s|/usr/bin/bible-app|/app/bin/bible-app|"));
    // what the app needs from outside its sandbox
    let args: Vec<&str> = json["finish-args"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a.as_str().unwrap())
        .collect();
    for need in [
        "--share=network",
        "--socket=wayland",
        "--socket=fallback-x11",
        "--socket=pulseaudio",
    ] {
        assert!(args.contains(&need), "missing {need}");
    }
    // Flathub asks for as little access as possible: no home folders
    assert!(!args.iter().any(|a| a.starts_with("--filesystem")), "no filesystem access");
}

#[test]
fn arch_installs_what_the_deb_installs() {
    let pkg = read("packaging/arch/PKGBUILD");
    for f in deb_files() {
        let covered = pkg.contains(&f)
            || (f.ends_with(".svg") && f.contains("symbolic") && pkg.contains("symbolic/apps/*.svg"));
        assert!(covered, "the PKGBUILD doesn't install {f}");
    }
}

#[test]
fn package_versions_match_the_workspace() {
    let ws = read("Cargo.toml");
    let ver = ws
        .lines()
        .find_map(|l| l.strip_prefix("version = \""))
        .and_then(|l| l.split('"').next())
        .unwrap();
    assert!(
        read("packaging/arch/PKGBUILD").contains(&format!("pkgver={ver}")),
        "PKGBUILD pkgver isn't {ver}"
    );
    assert!(
        read("CHANGELOG.md").contains(&format!("## {ver}")),
        "CHANGELOG.md has no section for {ver}"
    );
}

#[test]
fn store_listing_has_this_version() {
    // run scripts/metainfo-releases.py after adding the CHANGELOG section
    let meta = read("data/io.github.fonzi.Bible.metainfo.xml");
    let cargo = read("Cargo.toml");
    let ver = cargo
        .lines()
        .find_map(|l| l.strip_prefix("version = \""))
        .and_then(|v| v.split('"').next())
        .unwrap();
    let first = meta.split("<release version=\"").nth(1).and_then(|r| r.split('"').next()).unwrap();
    assert_eq!(first, ver, "the MetaInfo's newest release must be the app's version");
    assert!(meta.contains("<screenshot"), "the store listing needs screenshots");
}
