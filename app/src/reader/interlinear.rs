//! The Original pane: the Greek and Hebrew word by word, and a tapped
//! word's meaning, grammar and lexicon entry underneath.

use super::*;

impl Reader {
    pub(super) fn il_verse(&self) -> u16 {
        let st = self.state.borrow();
        if st.sel > 0 {
            st.sel
        } else {
            st.verses.first().copied().unwrap_or(1)
        }
    }

    pub(super) fn step_verse(&self, d: i32) {
        let next = {
            let st = self.state.borrow();
            let cur = if st.sel > 0 {
                st.sel
            } else {
                st.verses.first().copied().unwrap_or(1)
            };
            let i = st.verses.iter().position(|v| *v == cur).unwrap_or(0) as i32 + d;
            if i < 0 || i as usize >= st.verses.len() {
                None
            } else {
                Some(st.verses[i as usize])
            }
        };
        if let Some(v) = next {
            self.state.borrow_mut().sel = v;
            self.mark_selected();
            self.update_panel();
            self.scroll_to_verse(v);
        }
    }

    pub(super) fn update_interlinear(self: &Rc<Self>) {
        if !self.il_toggle.is_active() {
            return;
        }
        let (book, chapter) = {
            let st = self.state.borrow();
            (st.book.clone(), st.chapter)
        };
        let v = self.il_verse();
        self.il_title
            .set_text(&format!("Interlinear · {}", Ref::new(&book, chapter, v)));
        let Some(pack) = Pack::for_book(&book) else {
            self.il_sub.set_text("");
            self.il_missing.set_title(crate::i18n::t("No interlinear for this book").as_str());
            self.il_missing.set_description(Some(
                "Word-by-word data covers the Hebrew Old Testament and the Greek New Testament.",
            ));
            self.il_dl_btn.set_visible(false);
            self.il_stack.set_visible_child_name("missing");
            return;
        };
        self.il_sub.set_text(match pack {
            Pack::GreekNt => "Greek New Testament · STEP Bible",
            Pack::HebrewOt => "Hebrew Old Testament · STEP Bible",
        });
        if !bible_step::is_installed(pack) {
            self.il_missing.set_title(match pack {
                Pack::GreekNt => "Get the Greek interlinear",
                Pack::HebrewOt => "Get the Hebrew interlinear",
            });
            self.il_missing.set_description(Some(pack.description()));
            self.il_dl_btn.set_visible(true);
            self.il_dl_btn.set_sensitive(true);
            self.il_dl_btn.set_label(crate::i18n::t("Download").as_str());
            self.il_stack.set_visible_child_name("missing");
            return;
        }
        let me = self.clone();
        self.il_stack.set_visible_child_name("loading");
        glib::spawn_future_local(async move {
            let il = gtk::gio::spawn_blocking(move || Interlinear::get(pack))
                .await
                .ok()
                .flatten();
            let Some(il) = il else {
                me.il_missing
                    .set_title(crate::i18n::t("Couldn’t open the interlinear data").as_str());
                me.il_missing.set_description(Some(
                    "Try removing and downloading it again from Downloads.",
                ));
                me.il_stack.set_visible_child_name("missing");
                return;
            };
            // the passage may have changed while loading
            let (book, chapter) = {
                let st = me.state.borrow();
                (st.book.clone(), st.chapter)
            };
            if Pack::for_book(&book) != Some(pack) {
                me.update_interlinear();
                return;
            }
            let v = me.il_verse();
            me.fill_interlinear(il.verse(&book, chapter, v), pack);
            me.il_stack.set_visible_child_name("words");
        });
    }

    pub(super) fn fill_interlinear(self: &Rc<Self>, words: &[Word], pack: Pack) {
        while let Some(c) = self.il_flow.first_child() {
            self.il_flow.remove(&c);
        }
        self.il_flow.set_direction(if pack == Pack::HebrewOt {
            gtk::TextDirection::Rtl
        } else {
            gtk::TextDirection::Ltr
        });
        if words.is_empty() {
            let l = gtk::Label::new(Some(crate::i18n::t("No word-by-word data for this verse.").as_str()));
            l.add_css_class("dim-label");
            self.il_flow.insert(&l, -1);
            return;
        }
        for w in words {
            let b = gtk::Button::new();
            b.add_css_class("flat");
            b.add_css_class("word-card");
            let col = gtk::Box::new(gtk::Orientation::Vertical, 1);
            col.set_direction(gtk::TextDirection::Ltr);
            let t = gtk::Label::new(Some(&w.text));
            t.add_css_class("orig");
            let tr = gtk::Label::new(Some(&w.translit));
            tr.add_css_class("caption");
            tr.add_css_class("dim-label");
            let g = gtk::Label::new(Some(&w.gloss));
            g.add_css_class("caption-heading");
            g.set_max_width_chars(16);
            g.set_wrap(true);
            g.set_justify(gtk::Justification::Center);
            let st = gtk::Label::new(Some(&w.strong));
            st.add_css_class("caption");
            st.add_css_class("accent");
            for x in [&t, &tr, &g, &st] {
                col.append(x);
            }
            b.set_child(Some(&col));
            if w.variant {
                b.add_css_class("variant");
                b.set_tooltip_text(Some(crate::i18n::t("Not in the main critical text (NA28); found in the Textus Receptus or other editions").as_str()));
            } else {
                b.set_tooltip_text(Some(&bible_step::morph::describe(&w.morph)));
            }
            let me = self.clone();
            let word = w.clone();
            b.connect_clicked(move |_| me.show_word(&word, pack));
            self.il_flow.insert(&b, -1);
        }
    }

    pub(super) fn show_word(&self, w: &Word, pack: Pack) {
        let entry = Interlinear::get(pack).and_then(|il| il.lookup(&w.strong).cloned());
        let lemma = entry
            .as_ref()
            .map(|e| e.lemma.clone())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| {
                if w.lemma.is_empty() {
                    w.text.clone()
                } else {
                    w.lemma.clone()
                }
            });
        self.w_lemma.set_text(&lemma);
        let tr = entry
            .as_ref()
            .map(|e| e.translit.clone())
            .unwrap_or_else(|| w.translit.clone());
        self.w_sub.set_text(&format!("{tr} · {}", w.strong));
        self.w_form.set_markup(&format!(
            "<b>In this verse:</b> <span size=\"large\">{}</span> “{}” — {}",
            glib::markup_escape_text(&w.text),
            glib::markup_escape_text(&w.gloss),
            glib::markup_escape_text(&bible_step::morph::describe(&w.morph))
        ));
        let gloss = entry
            .as_ref()
            .map(|e| e.gloss.clone())
            .filter(|g| !g.is_empty())
            .unwrap_or_else(|| w.lemma_gloss.clone());
        self.w_gloss.set_text(&gloss);
        self.w_gloss.set_visible(!gloss.is_empty());
        let def = entry
            .as_ref()
            .map(|e| e.definition.clone())
            .unwrap_or_default();
        if crate::render::markup_ok(&def) {
            self.w_def.set_markup(&def);
        } else {
            self.w_def.set_text(&def);
        }
        self.w_def.set_visible(!def.is_empty());
        // installed Strong's dictionaries
        while let Some(c) = self.w_dicts.first_child() {
            self.w_dicts.remove(&c);
        }
        let lang = w.strong.chars().next().unwrap_or('G');
        let dark = self.dark();
        for m in self.ctx.lib().of_kind(Kind::Lexicon) {
            if m.strongs_lang() != Some(lang) {
                continue;
            }
            let Some((_, raw)) = m.ld().and_then(|ld| ld.get(&w.strong)) else {
                continue;
            };
            let head = gtk::Label::new(Some(&m.description()));
            head.add_css_class("caption-heading");
            head.add_css_class("dim-label");
            head.set_xalign(0.0);
            let body = gtk::Label::new(None);
            body.set_markup(&long_markup(&m.render(&raw), dark));
            body.set_wrap(true);
            body.set_xalign(0.0);
            body.set_selectable(true);
            let c = self.ctx.clone();
            body.connect_activate_link(move |_, uri| {
                if let Some(rf) = Ref::from_uri(uri) {
                    c.open_ref(&rf);
                }
                glib::Propagation::Stop
            });
            self.w_dicts
                .append(&gtk::Separator::new(gtk::Orientation::Horizontal));
            self.w_dicts.append(&head);
            self.w_dicts.append(&body);
        }
        let num: String = w
            .strong
            .chars()
            .skip(1)
            .take_while(|c| c.is_ascii_digit())
            .collect();
        let num = num.trim_start_matches('0');
        *self.w_query.borrow_mut() = format!("{lang}{num}");
        self.w_stack.set_visible_child_name("info");
        self.w_stack.set_visible(true);
    }

    pub(super) fn download_pack(self: &Rc<Self>) {
        let book = self.state.borrow().book.clone();
        let Some(pack) = Pack::for_book(&book) else {
            return;
        };
        self.il_dl_btn.set_sensitive(false);
        self.il_dl_btn.set_label(crate::i18n::t("Downloading…").as_str());
        self.il_progress.set_visible(true);
        self.il_progress.set_fraction(0.02);
        let (tx, rx) = std::sync::mpsc::channel::<(usize, usize)>();
        let bar = self.il_progress.clone();
        let tick = glib::timeout_add_local(std::time::Duration::from_millis(120), move || {
            while let Ok((d, t)) = rx.try_recv() {
                bar.set_fraction((d as f64 / t.max(1) as f64).max(0.02));
            }
            glib::ControlFlow::Continue
        });
        let me = self.clone();
        glib::spawn_future_local(async move {
            let res = gtk::gio::spawn_blocking(move || {
                bible_step::download(pack, &mut |d, t| {
                    let _ = tx.send((d, t));
                })
                .map_err(|e| e.to_string())
            })
            .await;
            tick.remove();
            me.il_progress.set_visible(false);
            match res {
                Ok(Ok(())) => {
                    me.ctx
                        .toast(&format!("Downloaded the {}", pack.title().to_lowercase()));
                    me.ctx.refresh_all();
                    me.update_interlinear();
                }
                Ok(Err(e)) => {
                    me.il_dl_btn.set_sensitive(true);
                    me.il_dl_btn.set_label(crate::i18n::t("Try again").as_str());
                    me.ctx.toast(&format!("Download failed: {e}"));
                }
                Err(_) => {
                    me.il_dl_btn.set_sensitive(true);
                    me.il_dl_btn.set_label(crate::i18n::t("Try again").as_str());
                }
            }
        });
    }
}
