//! A book's introduction: its date and setting, the Wikipedia article and
//! the map of the places it names.

use super::*;

impl Reader {
    /// Chapter 0: the book's introduction from the installed Bible and commentaries.
    pub(super) fn render_intro(&self, m: &Arc<Module>, book: &str) {
        let sys = m.versification();
        let Some(b) = sys.book(book) else { return };
        self.passage_btn.set_label(&format!("{} · Intro", b.name));
        self.verse_btn.set_visible(true);
        self.verse_btn.set_label(crate::i18n::t("Intro").as_str());
        self.player.tip.set_reveal_child(false);
        let dark = self.dark();
        let add = |w: &gtk::Widget| {
            let r = gtk::ListBoxRow::new();
            r.set_activatable(false);
            r.set_focusable(false);
            r.set_child(Some(w));
            self.list.append(&r);
        };
        let h = gtk::Label::new(Some(&format!("Introduction to {}", b.name)));
        h.add_css_class("chapter-heading");
        h.set_xalign(0.0);
        h.set_wrap(true);
        add(h.upcast_ref());
        let idx = sys.book_index(book).unwrap_or(0);
        let testament = if v11n::is_apocrypha(book) { crate::i18n::ts("Apocrypha") } else if idx < sys.ot_books { crate::i18n::ts("Old Testament") } else { crate::i18n::ts("New Testament") };
        let verses: u32 = b.verses.iter().map(|v| *v as u32).sum();
        let facts = gtk::Label::new(Some(&format!(
            "{testament} · {} chapters · {verses} verses",
            b.chapters()
        )));
        facts.add_css_class("dim-label");
        facts.set_xalign(0.0);
        add(facts.upcast_ref());
        let mut found = 0;
        // built-in overview, so every book has an introduction offline
        if let Some(intro) = bible_core::intros::intro(book) {
            let heading = |t: &str| {
                let l = gtk::Label::new(Some(t));
                l.add_css_class("caption-heading");
                l.add_css_class("dim-label");
                l.set_xalign(0.0);
                l.set_margin_top(18);
                l
            };
            let para = |t: &str| {
                let l = gtk::Label::new(Some(t));
                l.set_wrap(true);
                l.set_xalign(0.0);
                l.set_selectable(true);
                l.add_css_class("commentary");
                l
            };
            add(heading("Traditional view").upcast_ref());
            add(para(&intro.traditional).upcast_ref());
            if let Some((s, e)) = bible_core::timeline::timeline()
                .when(book, 1)
                .or_else(|| bible_core::timeline::timeline().when(book, 12))
            {
                let era = bible_core::timeline::timeline()
                    .era_at(s)
                    .map(|x| format!(" · {}", x.title))
                    .unwrap_or_default();
                add(heading("On the timeline").upcast_ref());
                add(para(&format!("{}{era}", bible_core::timeline::span(s, e, true))).upcast_ref());
            }
            if let Some((art, dir)) = crate::wiki::load(book) {
                // the full Wikipedia article, saved for offline reading
                let note = gtk::Label::new(Some(&format!(
                    "From Wikipedia’s article “{}”, saved {}. Links open Wikipedia and need internet.",
                    art.title, art.fetched
                )));
                note.set_wrap(true);
                note.set_xalign(0.0);
                note.add_css_class("caption");
                note.add_css_class("dim-label");
                note.set_margin_top(18);
                add(note.upcast_ref());
                add(heading("Overview (scholarly view)").upcast_ref());
                for b in &art.blocks {
                    match b {
                        crate::wiki::Block::Heading { level, text } => {
                            let l = gtk::Label::new(Some(text));
                            l.add_css_class(if *level <= 2 { "title-4" } else { "heading" });
                            l.set_xalign(0.0);
                            l.set_wrap(true);
                            l.set_margin_top(if *level <= 2 { 20 } else { 10 });
                            add(l.upcast_ref());
                        }
                        crate::wiki::Block::Para { markup } => {
                            let l = gtk::Label::new(None);
                            l.set_markup(&crate::wiki::verse_links(markup));
                            l.set_wrap(true);
                            l.set_xalign(0.0);
                            l.set_selectable(true);
                            l.add_css_class("commentary");
                            // a verse opens here; nothing opens the browser
                            let c = self.ctx.clone();
                            l.connect_activate_link(move |_, uri| {
                                if let Some(rf) = Ref::from_uri(uri) {
                                    c.open_ref(&rf);
                                }
                                glib::Propagation::Stop
                            });
                            add(l.upcast_ref());
                        }
                        crate::wiki::Block::Image { file, caption, artist, license, page } => {
                            let bx = gtk::Box::new(gtk::Orientation::Vertical, 4);
                            bx.set_margin_top(10);
                            bx.set_margin_bottom(6);
                            let pic = gtk::Picture::for_filename(dir.join(file));
                            pic.set_can_shrink(true);
                            pic.set_content_fit(gtk::ContentFit::Contain);
                            pic.set_size_request(-1, 320);
                            pic.set_halign(gtk::Align::Fill);
                            bx.append(&pic);
                            let cap = gtk::Label::new(None);
                            let mut m = caption.clone();
                            let who = [artist.as_str(), license.as_str()]
                                .iter()
                                .filter(|x| !x.is_empty())
                                .map(|x| glib::markup_escape_text(x).to_string())
                                .collect::<Vec<_>>()
                                .join(" · ");
                            if !page.is_empty() {
                                m.push_str(&format!(
                                    "\n<small><a href=\"{}\">{}</a></small>",
                                    glib::markup_escape_text(page),
                                    if who.is_empty() { "Wikimedia Commons".to_string() } else { who }
                                ));
                            }
                            cap.set_markup(m.trim());
                            cap.set_wrap(true);
                            cap.set_xalign(0.5);
                            cap.set_justify(gtk::Justification::Center);
                            cap.add_css_class("caption");
                            cap.add_css_class("dim-label");
                            bx.append(&cap);
                            add(bx.upcast_ref());
                        }
                    }
                }
                let credit = gtk::Label::new(None);
                credit.set_markup(&format!(
                    "Text from <a href=\"{}\">Wikipedia: {}</a>, CC BY-SA 4.0. Pictures from Wikimedia Commons, each under the licence shown. Bible references open here.",
                    glib::markup_escape_text(&art.url),
                    glib::markup_escape_text(&art.title)
                ));
                credit.set_xalign(0.0);
                credit.set_wrap(true);
                credit.add_css_class("caption");
                credit.add_css_class("dim-label");
                credit.set_margin_top(12);
                add(credit.upcast_ref());
            } else {
                add(heading("Overview (scholarly view)").upcast_ref());
                for p in intro.text.split('\n').filter(|p| !p.trim().is_empty()) {
                    add(para(p.trim()).upcast_ref());
                }
                // offer the full article with its pictures
                let row = gtk::Box::new(gtk::Orientation::Vertical, 8);
                row.set_margin_top(12);
                let l = gtk::Label::new(Some(crate::i18n::t("Get the full Wikipedia article with its pictures and maps. It downloads once and then reads offline; its links open Wikipedia and need internet.").as_str()));
                l.set_wrap(true);
                l.set_xalign(0.0);
                l.add_css_class("dim-label");
                row.append(&l);
                let btns = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                let one = gtk::Button::with_label(crate::i18n::t("Get this article").as_str());
                one.add_css_class("pill");
                one.add_css_class("suggested-action");
                let all = gtk::Button::with_label(crate::i18n::t("Get all books").as_str());
                all.add_css_class("pill");
                let status = gtk::Label::new(None);
                status.add_css_class("dim-label");
                let busy = crate::loader::new(56);
                busy.set_halign(gtk::Align::Start);
                busy.set_visible(false);
                status.set_wrap(true);
                status.set_xalign(0.0);
                btns.append(&one);
                btns.append(&all);
                row.append(&btns);
                row.append(&busy);
                row.append(&status);
                for (btn, every) in [(&one, false), (&all, true)] {
                    let w = self.me.borrow().clone();
                    let book = book.to_string();
                    let status = status.clone();
                    let btns = btns.clone();
                    let busy = busy.clone();
                    btn.connect_clicked(move |_| {
                        btns.set_sensitive(false);
                        busy.set_visible(true);
                        status.set_text("Downloading from Wikipedia…");
                        let list: Vec<(String, String)> = if every {
                            // every book of this Bible that has an introduction,
                            // apocrypha included when the Bible has them
                            let books: Vec<String> = w
                                .upgrade()
                                .and_then(|r| r.ctx.bible())
                                .map(|m| m.versification().books.iter().map(|b| b.osis.clone()).collect())
                                .unwrap_or_else(|| bible_core::plans::books().iter().map(|b| b.to_string()).collect());
                            books
                                .iter()
                                .map(|b| b.as_str())
                                .filter_map(|b| bible_core::intros::intro(b).map(|i| (b.to_string(), i.article.clone())))
                                .collect()
                        } else {
                            bible_core::intros::intro(&book)
                                .map(|i| vec![(book.clone(), i.article.clone())])
                                .unwrap_or_default()
                        };
                        let (tx, rx) = std::sync::mpsc::channel::<String>();
                        let st = status.clone();
                        let tick = glib::timeout_add_local(std::time::Duration::from_millis(150), move || {
                            while let Ok(m) = rx.try_recv() {
                                st.set_text(&m);
                            }
                            glib::ControlFlow::Continue
                        });
                        let w = w.clone();
                        let status = status.clone();
                        let btns = btns.clone();
                        let busy = busy.clone();
                        glib::spawn_future_local(async move {
                            let n = list.len();
                            let res = gtk::gio::spawn_blocking(move || {
                                let mut failed: Vec<String> = vec![];
                                for (k, (b, art)) in list.iter().enumerate() {
                                    let _ = tx.send(format!("Downloading {} of {n}: {art}…", k + 1));
                                    if let Err(e) = crate::wiki::fetch(b, art) {
                                        crate::log::error(&format!("wikipedia {art}: {e}"));
                                        failed.push(format!("{art}: {}", crate::wiki::reason(&e)));
                                    }
                                    // be gentle with Wikipedia
                                    std::thread::sleep(std::time::Duration::from_millis(3000));
                                }
                                failed
                            })
                            .await;
                            tick.remove();
                            btns.set_sensitive(true);
                            busy.set_visible(false);
                            match res {
                                Ok(f) if f.is_empty() => {
                                    if let Some(r) = w.upgrade() {
                                        r.render();
                                    }
                                }
                                Ok(f) => status.set_text(&format!(
                                    "{} of {n} could not be downloaded:\n{}\nThe rest were saved. You can retry single books in Preferences → Book introductions.",
                                    f.len(),
                                    f.iter().take(8).cloned().collect::<Vec<_>>().join("\n")
                                )),
                                Err(_) => status.set_text("The download stopped unexpectedly."),
                            }
                        });
                    });
                }
                add(row.upcast_ref());
            }
            // where the book happens
            if !bible_core::places::places().in_book(book).is_empty() {
                add(heading("Places in this book").upcast_ref());
                let map = crate::map::MapView::new(&self.ctx);
                map.set_book(book);
                // inside the page's own scrolling, the map needs a fixed height
                map.root.set_size_request(-1, 440);
                add(map.root.upcast_ref());
                *self.intro_map.borrow_mut() = Some(map);
            }
            let credit = gtk::Label::new(None);
            credit.set_visible(crate::wiki::load(book).is_none());
            credit.set_markup(&format!(
                "<a href=\"{}\">{}</a>",
                glib::markup_escape_text(&intro.url()),
                glib::markup_escape_text(&intro.credit())
            ));
            credit.set_xalign(0.0);
            credit.set_wrap(true);
            credit.add_css_class("caption");
            credit.add_css_class("dim-label");
            credit.set_margin_top(6);
            add(credit.upcast_ref());
            found += 1;
        }
        let mut section = |title: String, spans: Vec<bible_sword::Span>| {
            let t = gtk::Label::new(Some(&title));
            t.add_css_class("caption-heading");
            t.add_css_class("dim-label");
            t.set_xalign(0.0);
            t.set_margin_top(18);
            add(t.upcast_ref());
            let body = gtk::Label::new(None);
            body.set_markup(&long_markup(&spans, dark));
            body.set_wrap(true);
            body.set_xalign(0.0);
            body.set_selectable(true);
            body.add_css_class("commentary");
            let ctx = self.ctx.clone();
            body.connect_activate_link(move |_, uri| {
                if let Some(rf) = Ref::from_uri(uri) {
                    ctx.open_ref(&rf);
                }
                glib::Propagation::Stop
            });
            add(body.upcast_ref());
            found += 1;
        };
        let has_text = |sp: &Vec<bible_sword::Span>| sp.iter().any(|x| !x.text.trim().is_empty());
        if let Some(sp) = m
            .raw_entry(book, 0, 0)
            .map(|r| m.render(&r))
            .filter(|sp| has_text(sp))
        {
            section(m.description(), sp);
        }
        for c in self
            .ctx
            .lib()
            .of_kind(Kind::Commentary)
            .into_iter()
            .filter(|c| c.is_readable())
        {
            // book introductions live at the book heading, the chapter-1 heading, or
            // (as in Matthew Henry) at the start of the 1:1 entry before "Chapter 1"
            let direct = [(0u16, 0u16), (1, 0)].iter().find_map(|(ch, v)| {
                c.raw_entry(book, *ch, *v)
                    .map(|r| c.render(&r))
                    .filter(|sp| has_text(sp))
            });
            let sp = direct.or_else(|| {
                let all = c.raw_entry(book, 1, 1).map(|r| c.render(&r))?;
                let cut = all
                    .iter()
                    .position(|x| x.title && x.text.trim().to_lowercase().starts_with("chapter"))
                    .unwrap_or(0);
                let intro: Vec<bible_sword::Span> = all.into_iter().take(cut).collect();
                Some(intro).filter(|sp| has_text(sp))
            });
            if let Some(sp) = sp {
                section(c.description(), sp);
            }
        }
        if found == 0 {
            let l = gtk::Label::new(Some(crate::i18n::t("None of your installed books has an introduction to this book yet. Many commentaries do (Matthew Henry, for example) — get them from Downloads.").as_str()));
            l.set_wrap(true);
            l.set_xalign(0.0);
            l.set_margin_top(18);
            l.add_css_class("dim-label");
            add(l.upcast_ref());
        }
        {
            let mut st = self.state.borrow_mut();
            st.verses = vec![];
            st.sel = 0;
            st.notes = ChapterNotes::default();
        }
        self.update_panel();
    }
}
