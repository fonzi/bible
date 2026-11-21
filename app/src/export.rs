//! Export a chapter, with your highlights and notes, as Markdown.

use crate::ctx::Ctx;
use bible_sword::Module;

/// "# John 3 (WEB)" then each verse; highlighted verses as ==text== (Obsidian's
/// highlight syntax) with the colour's name, notes as quotes under their verse.
pub fn chapter_markdown(ctx: &Ctx, m: &Module, book: &str, chapter: u16) -> String {
    let notes = ctx.notes().load(book, chapter);
    let cfg = ctx.config.borrow();
    let name = bible_core::Ref::new(book, chapter, 0).book_name();
    let mut out = format!(
        "# {name} {chapter}\n\n*{}* · exported {}\n\n",
        m.description(),
        chrono::Local::now().format("%B %-d, %Y")
    );
    for (v, _) in m.chapter(book, chapter) {
        let text = m.verse_plain(book, chapter, v).unwrap_or_default();
        let text = text.trim();
        if text.is_empty() {
            continue;
        }
        let mut line = text.to_string();
        let mut colour = None;
        if let Some(id) = notes.highlights.get(&v) {
            colour = cfg.colour(id).map(|c| c.name.clone());
            line = format!("=={line}==");
        } else {
            // part-verse highlights: mark each phrase
            for r in notes.ranges_for(v) {
                if !r.text.is_empty() && line.contains(&r.text) {
                    line = line.replacen(&r.text, &format!("=={}==", r.text), 1);
                    colour = colour.or_else(|| cfg.colour(&r.color).map(|c| c.name.clone()));
                }
            }
        }
        out.push_str(&format!("**{v}** {line}"));
        if let Some(c) = colour {
            out.push_str(&format!(" *({c})*"));
        }
        out.push_str("\n\n");
        if let Some(n) = notes.notes.get(&v) {
            for l in n.lines() {
                out.push_str(&format!("> {l}\n"));
            }
            out.push('\n');
        }
    }
    out
}

/// Save to "<notes>/Exports/John 3 (WEB).md" and offer to open it.
pub fn export_chapter(ctx: &std::rc::Rc<Ctx>, m: &Module, book: &str, chapter: u16) {
    let md = chapter_markdown(ctx, m, book, chapter);
    let name = bible_core::Ref::new(book, chapter, 0).book_name();
    let dir = ctx.config.borrow().notes_dir.join("Exports");
    let path = dir.join(format!("{name} {chapter} ({}).md", m.abbreviation()));
    match std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&path, md)) {
        Ok(_) => {
            let c = ctx.clone();
            ctx.toast_action(
                &format!("Saved {}", path.file_name().unwrap().to_string_lossy()),
                "Open",
                move || c.launch(&path),
            );
        }
        Err(e) => ctx.toast(&format!("Couldn’t export: {e}")),
    }
}
