//! GNOME Shell search provider: type "John 3:16" or a phrase in the
//! Activities overview and see verses from your Bible.
//!
//! Exported on the session bus at OBJECT_PATH while the app runs; GNOME Shell
//! starts the app as a background service (see data/*.service) when needed.

use crate::config::Config;
use bible_sword::{Kind, Library, Module};
use gtk::gio;
use gtk::glib::{self, variant::ToVariant, Variant, VariantDict};
use std::cell::RefCell;
use std::sync::Arc;

pub const OBJECT_PATH: &str = "/io/github/fonzi/Bible/SearchProvider";

const XML: &str = r#"<node>
  <interface name="org.gnome.Shell.SearchProvider2">
    <method name="GetInitialResultSet"><arg type="as" name="terms" direction="in"/><arg type="as" name="results" direction="out"/></method>
    <method name="GetSubsearchResultSet"><arg type="as" name="previous_results" direction="in"/><arg type="as" name="terms" direction="in"/><arg type="as" name="results" direction="out"/></method>
    <method name="GetResultMetas"><arg type="as" name="identifiers" direction="in"/><arg type="aa{sv}" name="metas" direction="out"/></method>
    <method name="ActivateResult"><arg type="s" name="identifier" direction="in"/><arg type="as" name="terms" direction="in"/><arg type="u" name="timestamp" direction="in"/></method>
    <method name="LaunchSearch"><arg type="as" name="terms" direction="in"/><arg type="u" name="timestamp" direction="in"/></method>
  </interface>
</node>"#;

thread_local! {
    static BIBLE: RefCell<Option<Arc<Module>>> = const { RefCell::new(None) };
}

/// The reading Bible, found once per run (the shell calls on every keystroke).
pub fn bible() -> Option<Arc<Module>> {
    if let Some(m) = BIBLE.with(|b| b.borrow().clone()) {
        return Some(m);
    }
    let name = Config::load().bible;
    let lib = Library::scan();
    let m = lib
        .get(&name)
        .filter(|m| m.kind == Kind::Bible && m.is_readable())
        .or_else(|| {
            lib.of_kind(Kind::Bible)
                .into_iter()
                .find(|m| m.is_readable())
        })?;
    BIBLE.with(|b| *b.borrow_mut() = Some(m.clone()));
    Some(m)
}

/// Result ids are OSIS references, "John.3.16".
pub fn results(terms: &[String]) -> Vec<String> {
    let q = terms.join(" ");
    let q = q.trim();
    if q.len() < 3 {
        return vec![];
    }
    let mut out = vec![];
    if let Some(r) = bible_core::refs::parse(q) {
        let v = if r.verse == 0 { 1 } else { r.verse };
        out.push(format!("{}.{}.{}", r.book, r.chapter.max(1), v));
        return out;
    }
    // phrases: only when there is something worth searching for
    if q.split_whitespace().count() >= 2 || q.len() >= 5 {
        if let Some(m) = bible() {
            for (r, _) in m.search(q, 5, |_| {}) {
                out.push(format!("{}.{}.{}", r.book, r.chapter, r.verse));
            }
        }
    }
    out
}

fn meta(id: &str) -> Variant {
    let d = VariantDict::new(None);
    d.insert("id", id);
    let r = bible_core::refs::parse_osis(id);
    let name = r
        .as_ref()
        .map(|r| r.to_string())
        .unwrap_or_else(|| id.to_string());
    let text = match (&r, bible()) {
        (Some(r), Some(m)) => m
            .verse_plain(&r.book, r.chapter, r.verse)
            .unwrap_or_default(),
        _ => String::new(),
    };
    let abbr = bible().map(|m| m.abbreviation()).unwrap_or_default();
    d.insert("name", format!("{name} · {abbr}"));
    d.insert("description", text);
    d.insert("gicon", "io.github.fonzi.Bible");
    d.end()
}

/// Export the provider on the app's bus connection.
pub fn register(app: &adw::Application) {
    let Some(conn) = gio::prelude::ApplicationExt::dbus_connection(app) else {
        return;
    };
    let Ok(node) = gio::DBusNodeInfo::for_xml(XML) else {
        return;
    };
    let Some(iface) = node.lookup_interface("org.gnome.Shell.SearchProvider2") else {
        return;
    };
    let app = app.clone();
    let res = conn
        .register_object(OBJECT_PATH, &iface)
        .method_call(move |_, _, _, _, method, params, invocation| match method {
            "GetInitialResultSet" => {
                let terms: Vec<String> = params.child_value(0).get().unwrap_or_default();
                invocation.return_value(Some(&(results(&terms),).to_variant()));
            }
            "GetSubsearchResultSet" => {
                let terms: Vec<String> = params.child_value(1).get().unwrap_or_default();
                invocation.return_value(Some(&(results(&terms),).to_variant()));
            }
            "GetResultMetas" => {
                let ids: Vec<String> = params.child_value(0).get().unwrap_or_default();
                let metas: Vec<Variant> = ids.iter().map(|i| meta(i)).collect();
                let arr = Variant::array_from_iter_with_type(glib::VariantTy::VARDICT, metas);
                invocation.return_value(Some(&Variant::tuple_from_iter([arr])));
            }
            "ActivateResult" => {
                let id: String = params.child_value(0).get().unwrap_or_default();
                let file = gio::File::for_uri(&format!("bible://{id}"));
                gio::prelude::ApplicationExt::open(&app, &[file], "");
                invocation.return_value(None);
            }
            "LaunchSearch" => {
                let terms: Vec<String> = params.child_value(0).get().unwrap_or_default();
                let q = terms.join(" ");
                gio::prelude::ApplicationExt::activate(&app);
                crate::with_ctx(|c| c.open_search(&q));
                invocation.return_value(None);
            }
            _ => invocation.return_error(gio::IOErrorEnum::NotSupported, "unknown method"),
        })
        .build();
    if let Err(e) = res {
        eprintln!("search provider: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn references_become_ids() {
        assert_eq!(results(&["john".into(), "3:16".into()]), vec!["John.3.16"]);
        assert_eq!(results(&["Rom".into(), "8".into()]), vec!["Rom.8.1"]);
        assert!(results(&["jo".into()]).is_empty());
    }
}
