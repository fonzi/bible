//! Remote module sources (SWORD "raw" repositories over HTTPS): fetching the
//! catalogue, installing and removing modules.

use crate::conf::{decode, Conf};
use crate::module::{user_root, Module};
use flate2::read::GzDecoder;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    pub name: String,
    /// Base URL of the raw repository, e.g. https://crosswire.org/ftpmirror/pub/sword/raw
    pub url: String,
    pub enabled: bool,
}

impl Default for Source {
    fn default() -> Self {
        Source {
            name: String::new(),
            url: String::new(),
            enabled: true,
        }
    }
}

pub fn default_sources() -> Vec<Source> {
    vec![
        Source {
            name: "CrossWire".into(),
            url: "https://crosswire.org/ftpmirror/pub/sword/raw".into(),
            enabled: true,
        },
        Source {
            name: "eBible.org".into(),
            url: "https://ebible.org/sword".into(),
            enabled: true,
        },
        Source {
            name: "CrossWire Beta".into(),
            url: "https://crosswire.org/ftpmirror/pub/sword/betaraw".into(),
            enabled: false,
        },
        Source {
            name: "CrossWire Attic".into(),
            url: "https://crosswire.org/ftpmirror/pub/sword/atticraw".into(),
            enabled: false,
        },
        Source {
            name: "Bible.org".into(),
            url: "https://crosswire.org/ftpmirror/pub/bible.org/sword".into(),
            enabled: false,
        },
        Source {
            name: "Xiphos".into(),
            url: "http://ftp.xiphos.org/pub/xiphos".into(),
            enabled: false,
        },
    ]
}

#[derive(Debug, Clone)]
pub struct RemoteModule {
    pub source: Source,
    pub conf: Conf,
}

impl RemoteModule {
    pub fn name(&self) -> &str {
        &self.conf.name
    }
    pub fn description(&self) -> String {
        self.conf
            .get("Description")
            .unwrap_or(&self.conf.name)
            .to_string()
    }
    pub fn version(&self) -> String {
        self.conf.get_or("Version", "1.0").to_string()
    }
    pub fn lang(&self) -> String {
        self.conf.get_or("Lang", "en").to_string()
    }
}

/// Whether a host should skip the proxy: local addresses and anything in NO_PROXY.
fn bypass_proxy(url: &str) -> bool {
    let host = url
        .split("://")
        .nth(1)
        .unwrap_or(url)
        .split(['/', ':'])
        .next()
        .unwrap_or("")
        .to_lowercase();
    if host == "localhost" || host.starts_with("127.") || host == "[::1]" {
        return true;
    }
    let list = std::env::var("NO_PROXY")
        .or_else(|_| std::env::var("no_proxy"))
        .unwrap_or_default();
    list.split(',')
        .map(|e| e.trim().trim_start_matches('.').to_lowercase())
        .filter(|e| !e.is_empty())
        .any(|e| e == "*" || host == e || host.ends_with(&format!(".{e}")))
}


/// Work offline: when set, every download refuses before touching the network.
static OFFLINE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn set_offline(on: bool) {
    OFFLINE.store(on, std::sync::atomic::Ordering::Relaxed);
}

pub fn is_offline() -> bool {
    OFFLINE.load(std::sync::atomic::Ordering::Relaxed)
}

fn offline_error() -> io::Error {
    io::Error::other("Work offline is on (turn it off in Preferences to download)")
}

fn agent(url: &str) -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(20))
        .timeout_read(Duration::from_secs(120))
        .try_proxy_from_env(!bypass_proxy(url))
        .user_agent(concat!("bible-app/", env!("CARGO_PKG_VERSION"), " (GNOME Bible study app; GPL-3)"))
        .build()
}

/// Fetch a URL into memory (None on 404).
pub fn get_bytes(url: &str) -> io::Result<Option<Vec<u8>>> {
    get(url)
}

fn get(url: &str) -> io::Result<Option<Vec<u8>>> {
    if is_offline() {
        return Err(offline_error());
    }
    match agent(url).get(url).call() {
        Ok(resp) => {
            let mut buf = Vec::new();
            resp.into_reader()
                .take(512 * 1024 * 1024)
                .read_to_end(&mut buf)?;
            Ok(Some(buf))
        }
        Err(ureq::Error::Status(404, _)) => Ok(None),
        Err(e) => Err(io::Error::other(e.to_string())),
    }
}

/// Download a file to `dest` (written to a temporary file first, so a broken
/// download never leaves a half file). `progress` gets (bytes, total bytes or 0).
pub fn download_file(
    url: &str,
    dest: &Path,
    progress: &mut dyn FnMut(u64, u64),
) -> io::Result<u64> {
    if is_offline() {
        return Err(offline_error());
    }
    let resp = match agent(url).get(url).call() {
        Ok(r) => r,
        Err(ureq::Error::Status(404, _)) => {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Not found: {url}"),
            ))
        }
        Err(e) => return Err(io::Error::other(e.to_string())),
    };
    let total: u64 = resp
        .header("Content-Length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    if let Some(dir) = dest.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = dest.with_extension("part");
    let mut out = fs::File::create(&tmp)?;
    let mut reader = resp.into_reader().take(2 * 1024 * 1024 * 1024);
    let mut buf = vec![0u8; 64 * 1024];
    let mut done = 0u64;
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        std::io::Write::write_all(&mut out, &buf[..n])?;
        done += n as u64;
        progress(done, total);
    }
    drop(out);
    if done == 0 {
        let _ = fs::remove_file(&tmp);
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "Empty download",
        ));
    }
    fs::rename(&tmp, dest)?;
    Ok(done)
}

/// Download and parse a source's catalogue (mods.d.tar.gz).
pub fn fetch_catalogue(src: &Source) -> io::Result<Vec<RemoteModule>> {
    let url = format!("{}/mods.d.tar.gz", src.url.trim_end_matches('/'));
    let bytes = get(&url)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("No catalogue at {url}")))?;
    parse_catalogue(src, &bytes)
}

pub fn parse_catalogue(src: &Source, targz: &[u8]) -> io::Result<Vec<RemoteModule>> {
    let mut ar = tar::Archive::new(GzDecoder::new(targz));
    let mut out = Vec::new();
    for e in ar.entries()? {
        let mut e = e?;
        let is_conf = e
            .path()
            .map(|p| p.extension().map(|x| x == "conf").unwrap_or(false))
            .unwrap_or(false);
        if !is_conf {
            continue;
        }
        let mut buf = Vec::new();
        e.read_to_end(&mut buf)?;
        if let Some(c) = Conf::parse(&decode(&buf)) {
            out.push(RemoteModule {
                source: src.clone(),
                conf: c,
            });
        }
    }
    out.sort_by_key(|a| a.description().to_lowercase());
    Ok(out)
}

/// Files a module of this driver consists of, relative to the library root.
fn module_files(conf: &Conf) -> Vec<String> {
    let dp = conf
        .get_or("DataPath", "")
        .trim_start_matches("./")
        .to_string();
    let drv = conf.get_or("ModDrv", "").to_ascii_lowercase();
    let dir = dp.trim_end_matches('/');
    let in_dir = |names: &[&str]| {
        names
            .iter()
            .map(|n| format!("{dir}/{n}"))
            .collect::<Vec<_>>()
    };
    let with_prefix = |exts: &[&str]| exts.iter().map(|e| format!("{dir}{e}")).collect::<Vec<_>>();
    match drv.as_str() {
        "ztext" | "zcom" | "ztext4" | "zcom4" => {
            let l = crate::module::block_letter(conf);
            in_dir(&["ot.Xzs", "ot.Xzv", "ot.Xzz", "nt.Xzs", "nt.Xzv", "nt.Xzz"])
                .into_iter()
                .map(|f| f.replace(".X", &format!(".{l}")))
                .collect()
        }
        "rawtext" | "rawcom" | "rawtext4" | "rawcom4" | "rawfiles" => {
            in_dir(&["ot", "ot.vss", "nt", "nt.vss"])
        }
        "zld" => with_prefix(&[".idx", ".dat", ".zdx", ".zdt"]),
        "rawld" | "rawld4" => with_prefix(&[".idx", ".dat"]),
        "rawgenbook" => with_prefix(&[".bdt", ".dat", ".idx"]),
        _ => vec![],
    }
}

/// Install into ~/.sword. `progress` receives (files done, files total).
pub fn install(m: &RemoteModule, mut progress: impl FnMut(usize, usize)) -> io::Result<PathBuf> {
    install_into(m, &user_root(), &mut progress)
}

pub fn install_into(
    m: &RemoteModule,
    root: &Path,
    progress: &mut dyn FnMut(usize, usize),
) -> io::Result<PathBuf> {
    let files = module_files(&m.conf);
    if files.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "This kind of module can’t be installed yet",
        ));
    }
    let base = m.source.url.trim_end_matches('/');
    let mut got = 0;
    for (i, rel) in files.iter().enumerate() {
        progress(i, files.len());
        if let Some(bytes) = get(&format!("{base}/{rel}"))? {
            let dest = root.join(rel);
            if let Some(d) = dest.parent() {
                fs::create_dir_all(d)?;
            }
            fs::write(dest, bytes)?;
            got += 1;
        }
    }
    if got == 0 {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "The server didn’t have this module’s files",
        ));
    }
    progress(files.len(), files.len());
    fs::create_dir_all(root.join("mods.d"))?;
    let conf_path = root
        .join("mods.d")
        .join(format!("{}.conf", m.conf.name.to_lowercase()));
    fs::write(&conf_path, m.conf.raw.as_bytes())?;
    Ok(conf_path)
}

/// Remove a module installed in the user library. System modules (from packages) are left alone.
pub fn remove(m: &Module) -> io::Result<()> {
    if !m.user_installed {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "This module was installed by your system and can’t be removed here",
        ));
    }
    for rel in module_files(&m.conf) {
        let _ = fs::remove_file(m.root.join(rel));
    }
    let dp = m
        .conf
        .get_or("DataPath", "")
        .trim_start_matches("./")
        .trim_end_matches('/')
        .to_string();
    let dir = m.root.join(&dp);
    if dir.is_dir() {
        let _ = fs::remove_dir(&dir);
    }
    let mods = m.root.join("mods.d");
    if let Ok(rd) = fs::read_dir(&mods) {
        for e in rd.flatten() {
            if let Ok(bytes) = fs::read(e.path()) {
                if Conf::parse(&decode(&bytes))
                    .map(|c| c.name.eq_ignore_ascii_case(&m.name))
                    .unwrap_or(false)
                {
                    fs::remove_file(e.path())?;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{write::GzEncoder, Compression};

    #[test]
    fn catalogue_and_file_lists() {
        // build a fake mods.d.tar.gz
        let conf = b"[KJV]\nDataPath=./modules/texts/ztext/kjv/\nModDrv=zText\nDescription=King James Version\nVersion=3.1\n";
        let mut tarbuf = Vec::new();
        {
            let mut b = tar::Builder::new(&mut tarbuf);
            let mut h = tar::Header::new_gnu();
            h.set_size(conf.len() as u64);
            h.set_mode(0o644);
            h.set_cksum();
            b.append_data(&mut h, "mods.d/kjv.conf", &conf[..]).unwrap();
            b.finish().unwrap();
        }
        let mut gz = GzEncoder::new(Vec::new(), Compression::default());
        std::io::Write::write_all(&mut gz, &tarbuf).unwrap();
        let src = Source {
            name: "Test".into(),
            url: "https://example.org/raw".into(),
            enabled: true,
        };
        let cat = parse_catalogue(&src, &gz.finish().unwrap()).unwrap();
        assert_eq!(cat.len(), 1);
        assert_eq!(cat[0].description(), "King James Version");
        assert_eq!(
            module_files(&cat[0].conf)[0],
            "modules/texts/ztext/kjv/ot.bzs"
        );
        let ld = Conf::parse(
            "[Strongs]\nDataPath=./modules/lexdict/zld/strongsgreek/dict\nModDrv=zLD\n",
        )
        .unwrap();
        assert_eq!(
            module_files(&ld)[0],
            "modules/lexdict/zld/strongsgreek/dict.idx"
        );
    }
}

#[cfg(test)]
mod live {
    use super::*;
    /// Runs only when TEST_REPO_URL points at a raw SWORD repository.
    #[test]
    fn install_from_repo() {
        let Ok(url) = std::env::var("TEST_REPO_URL") else {
            return;
        };
        let src = Source {
            name: "Local".into(),
            url,
            enabled: true,
        };
        let cat = fetch_catalogue(&src).unwrap();
        assert!(cat.iter().any(|m| m.name() == "engKJV2006eb"));
        let root = std::env::temp_dir().join(format!("sword-install-{}", std::process::id()));
        for m in &cat {
            let mut p = |_d: usize, _t: usize| {};
            install_into(m, &root, &mut p).unwrap();
        }
        let lib = crate::Library::scan_roots(&[(root.clone(), true)]);
        let kjv = lib.get("engKJV2006eb").unwrap();
        assert!(kjv
            .verse_plain("John", 3, 16)
            .unwrap()
            .contains("God so loved"));
        let tr = lib.get("TR").unwrap();
        assert!(tr.verse_plain("John", 1, 1).is_some());
        remove(&kjv).unwrap();
        assert!(crate::Library::scan_roots(&[(root.clone(), true)])
            .get("engKJV2006eb")
            .is_none());
        std::fs::remove_dir_all(root).unwrap();
    }
}

/// Install a SWORD module the person already has, from a .zip with the usual
/// `mods.d/*.conf` and `modules/…` folders, into `root` (normally
/// `~/.sword`). Only those two folders are unpacked, and no path may leave
/// `root`. Returns the names of the modules it installed.
pub fn import_zip(zip_path: &Path, root: &Path) -> io::Result<Vec<String>> {
    let f = std::fs::File::open(zip_path)?;
    let mut ar = zip::ZipArchive::new(f).map_err(io::Error::other)?;
    let mut names = Vec::new();
    for i in 0..ar.len() {
        let e = ar.by_index(i).map_err(io::Error::other)?;
        let Some(rel) = e.enclosed_name() else { continue };
        let first = rel.components().next().map(|c| c.as_os_str().to_string_lossy().to_lowercase());
        if !matches!(first.as_deref(), Some("mods.d")) {
            continue;
        }
        if rel.extension().and_then(|x| x.to_str()) == Some("conf") {
            names.push(rel.file_stem().unwrap_or_default().to_string_lossy().into_owned());
        }
    }
    if names.is_empty() {
        return Err(io::Error::other(
            "This zip has no SWORD module in it (no mods.d folder with a .conf file).",
        ));
    }
    for i in 0..ar.len() {
        let mut e = ar.by_index(i).map_err(io::Error::other)?;
        let Some(rel) = e.enclosed_name() else { continue };
        let first = rel.components().next().map(|c| c.as_os_str().to_string_lossy().to_lowercase());
        if !matches!(first.as_deref(), Some("mods.d") | Some("modules")) {
            continue;
        }
        let out = root.join(&rel);
        if e.is_dir() {
            std::fs::create_dir_all(&out)?;
            continue;
        }
        if let Some(p) = out.parent() {
            std::fs::create_dir_all(p)?;
        }
        let mut w = std::fs::File::create(&out)?;
        io::copy(&mut e, &mut w)?;
    }
    Ok(names)
}

#[cfg(test)]
mod import_tests {
    use super::*;
    use std::io::Write;

    fn make_zip(entries: &[(&str, &str)]) -> tempfile_path::P {
        let p = tempfile_path::P::new("zip");
        let mut z = zip::ZipWriter::new(std::fs::File::create(&p.0).unwrap());
        for (name, body) in entries {
            z.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
            z.write_all(body.as_bytes()).unwrap();
        }
        z.finish().unwrap();
        p
    }

    mod tempfile_path {
        pub struct P(pub std::path::PathBuf);
        impl P {
            pub fn new(ext: &str) -> P {
                let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
                P(std::env::temp_dir().join(format!("bible-import-{n}.{ext}")))
            }
        }
        impl Drop for P {
            fn drop(&mut self) {
                let _ = std::fs::remove_file(&self.0);
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
    }

    #[test]
    fn imports_only_module_folders() {
        let z = make_zip(&[
            ("mods.d/mine.conf", "[Mine]\nDataPath=./modules/texts/ztext/mine/\n"),
            ("modules/texts/ztext/mine/nt.bzz", "x"),
            ("readme.txt", "ignored"),
            ("../escape.txt", "never written"),
        ]);
        let root = tempfile_path::P::new("d");
        let names = import_zip(&z.0, &root.0).unwrap();
        assert_eq!(names, vec!["mine".to_string()]);
        assert!(root.0.join("mods.d/mine.conf").exists());
        assert!(root.0.join("modules/texts/ztext/mine/nt.bzz").exists());
        assert!(!root.0.join("readme.txt").exists());
        assert!(!root.0.parent().unwrap().join("escape.txt").exists());
    }

    #[test]
    fn refuses_a_zip_without_a_module() {
        let z = make_zip(&[("photo.jpg", "x")]);
        let root = tempfile_path::P::new("d");
        assert!(import_zip(&z.0, &root.0).is_err());
    }
}

/// Ask a server whether a download is still there, without downloading it:
/// a HEAD request, falling back to GET for servers that refuse HEAD. Used by
/// the download-URL checks (`cargo test -- --ignored download_urls`).
pub fn url_status(url: &str) -> Result<u16, String> {
    let a = agent(url);
    match a.head(url).call() {
        Ok(r) => Ok(r.status()),
        Err(ureq::Error::Status(405 | 403 | 501, _)) => match a.get(url).call() {
            Ok(r) => Ok(r.status()),
            Err(ureq::Error::Status(c, _)) => Ok(c),
            Err(e) => Err(e.to_string()),
        },
        Err(ureq::Error::Status(c, _)) => Ok(c),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(test)]
mod url_checks {
    #[test]
    #[ignore = "needs the internet; run with --ignored download_urls"]
    fn download_urls_sword_sources() {
        let mut bad = vec![];
        for s in super::default_sources() {
            let u = format!("{}/mods.d.tar.gz", s.url.trim_end_matches('/'));
            match super::url_status(&u) {
                Ok(200) => println!("ok   {u}"),
                other => bad.push(format!("{u}: {other:?}")),
            }
        }
        assert!(bad.is_empty(), "unreachable downloads:\n{}", bad.join("\n"));
    }
}
