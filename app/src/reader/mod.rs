//! The Bible reader: passage and version pickers, parallel versions, highlights,
//! Markdown notes and a study panel that follows the selected verse.

use crate::ctx::{sidebar_button, Ctx, Section};
use crate::render::{long_markup, spans_to_markup, verse_number, Style};
use adw::prelude::*;
use bible_core::notes::ChapterNotes;
use bible_core::{v11n, Ref};
use bible_step::{Interlinear, Pack, Word};
use bible_sword::{Kind, Module};
use gtk::{gdk, glib};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::sync::Arc;

mod interlinear;
mod intro;

struct State {
    book: String,
    chapter: u16,
    sel: u16,
    notes: ChapterNotes,
    verses: Vec<u16>,
}

pub struct Reader {
    ctx: Rc<Ctx>,
    /// when a link in the text was last activated, so the verse menu stays shut
    link_at: Cell<Option<std::time::Instant>>,
    root: adw::OverlaySplitView,
    split: adw::OverlaySplitView,
    passage_btn: gtk::MenuButton,
    verse_btn: gtk::MenuButton,
    verse_flow: gtk::FlowBox,
    nav_box: gtk::Box,
    panel_toggle: gtk::ToggleButton,
    passage_popover: gtk::Popover,
    book_list: gtk::ListBox,
    chapter_box: gtk::FlowBox,
    picker_stack: gtk::Stack,
    picker_book_title: gtk::Label,
    version_dd: gtk::DropDown,
    versions: RefCell<Vec<String>>,
    version_notes: Rc<RefCell<Vec<String>>>,
    par_toggle: gtk::ToggleButton,
    par_dd: gtk::DropDown,
    clamp: adw::Clamp,
    list: gtk::ListBox,
    scroller: gtk::ScrolledWindow,
    content_stack: gtk::Stack,
    tip: gtk::Revealer,
    // study panel
    panel_stack: adw::ViewStack,
    map: Rc<crate::map::MapView>,
    /// the map on an introduction page (kept alive while the page shows)
    intro_map: RefCell<Option<Rc<crate::map::MapView>>>,
    com_dd: gtk::DropDown,
    coms: RefCell<Vec<String>>,
    com_ref: gtk::Label,
    com_text: gtk::Label,
    com_stack: gtk::Stack,
    note_ref: gtk::Label,
    note_view: gtk::TextView,
    note_status: gtk::Label,
    hl_box: gtk::Box,
    // interlinear
    il_toggle: gtk::ToggleButton,
    listen_toggle: gtk::ToggleButton,
    player: Rc<crate::audio::Player>,
    il_revealer: gtk::Revealer,
    il_title: gtk::Label,
    il_sub: gtk::Label,
    il_flow: gtk::FlowBox,
    il_stack: gtk::Stack,
    il_missing: adw::StatusPage,
    il_dl_btn: gtk::Button,
    il_progress: gtk::ProgressBar,
    side_was: Cell<bool>,
    // words tab
    w_stack: gtk::Stack,
    w_lemma: gtk::Label,
    w_sub: gtk::Label,
    w_form: gtk::Label,
    w_gloss: gtk::Label,
    w_def: gtk::Label,
    w_dicts: gtk::Box,
    w_query: RefCell<String>,
    me: RefCell<Weak<Reader>>,
    state: RefCell<State>,
    loading_note: Cell<bool>,
    save_gen: Cell<u64>,
    filling: Cell<bool>,
}

/// Where a part-verse highlight falls in this version's text: at its saved offsets if the
/// words are still there, otherwise wherever its words occur; None if they don't.
fn locate(plain: &str, r: &bible_core::notes::RangeHighlight) -> Option<(usize, usize)> {
    let chars: Vec<char> = plain.chars().collect();
    if r.to <= chars.len() {
        let here: String = chars[r.from..r.to].iter().collect();
        if r.text.is_empty() || here == r.text {
            return Some((r.from, r.to));
        }
    }
    if r.text.is_empty() {
        return None;
    }
    let byte = plain.find(&r.text)?;
    let from = plain[..byte].chars().count();
    Some((from, from + r.text.chars().count()))
}

/// Close a popover and detach it from its parent before that parent can be destroyed.
fn close_popover(p: &gtk::Popover) {
    p.popdown();
    if p.parent().is_some() {
        p.unparent();
    }
}

fn icon_button(icon: &str, tip: &str) -> gtk::Button {
    let b = gtk::Button::from_icon_name(icon);
    b.set_tooltip_text(Some(tip));
    b
}

impl Reader {
    pub fn new(ctx: &Rc<Ctx>) -> Rc<Reader> {
        // ----- header -----
        let header = adw::HeaderBar::new();
        header.pack_start(&sidebar_button(ctx));
        let prev = icon_button("go-previous-symbolic", "Previous chapter");
        let next = icon_button("go-next-symbolic", "Next chapter");

        let passage_btn = gtk::MenuButton::new();
        passage_btn.set_always_show_arrow(true);
        let passage_popover = gtk::Popover::new();
        passage_btn.set_popover(Some(&passage_popover));
        let version_dd = gtk::DropDown::from_strings(&[]);
        // full name and coverage shown under each short name in the menu
        let version_notes: Rc<RefCell<Vec<String>>> = Rc::default();
        version_dd.set_tooltip_text(Some(crate::i18n::t("Version").as_str()));
        version_dd.set_enable_search(true);
        let title_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        title_box.append(&prev);
        title_box.append(&passage_btn);
        // jump to a verse in this chapter
        let verse_btn = gtk::MenuButton::new();
        verse_btn.set_always_show_arrow(true);
        verse_btn.set_label(crate::i18n::t("Verse").as_str());
        verse_btn.set_tooltip_text(Some(crate::i18n::t("Go to a verse").as_str()));
        let verse_flow = gtk::FlowBox::new();
        verse_flow.set_max_children_per_line(8);
        verse_flow.set_min_children_per_line(6);
        verse_flow.set_selection_mode(gtk::SelectionMode::None);
        verse_flow.set_homogeneous(true);
        verse_flow.set_row_spacing(4);
        verse_flow.set_column_spacing(4);
        let verse_scroll = gtk::ScrolledWindow::builder()
            .child(&verse_flow)
            .min_content_width(300)
            .max_content_height(420)
            .propagate_natural_height(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build();
        let verse_pop = gtk::Popover::new();
        verse_pop.set_child(Some(&verse_scroll));
        verse_btn.set_popover(Some(&verse_pop));
        title_box.append(&verse_btn);
        title_box.append(&next);
        // the header shows a menu button with a short, ellipsized name, so a
        // long abbreviation never widens the window; it drives version_dd
        let version_btn = gtk::MenuButton::new();
        version_btn.set_always_show_arrow(true);
        version_btn.set_tooltip_text(Some(crate::i18n::t("Version").as_str()));
        let version_lab = gtk::Label::new(None);
        version_lab.set_ellipsize(gtk::pango::EllipsizeMode::End);
        version_lab.set_max_width_chars(10);
        version_lab.set_width_chars(4);
        version_btn.set_child(Some(&version_lab));
        let version_list = gtk::ListBox::new();
        version_list.set_selection_mode(gtk::SelectionMode::None);
        version_list.add_css_class("navigation-sidebar");
        let version_scroll = gtk::ScrolledWindow::builder()
            .child(&version_list)
            .max_content_height(420)
            .propagate_natural_height(true)
            .propagate_natural_width(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build();
        let version_pop = gtk::Popover::new();
        version_pop.set_child(Some(&version_scroll));
        version_btn.set_popover(Some(&version_pop));
        {
            let dd = version_dd.clone();
            let pop = version_pop.clone();
            version_list.connect_row_activated(move |_, row| {
                dd.set_selected(row.index() as u32);
                pop.popdown();
            });
        }
        {
            let lab = version_lab.clone();
            let btn = version_btn.clone();
            let list = version_list.clone();
            let notes = version_notes.clone();
            let update = move |dd: &gtk::DropDown| {
                let text = dd
                    .selected_item()
                    .and_downcast::<gtk::StringObject>()
                    .map(|o| o.string().to_string())
                    .unwrap_or_default();
                lab.set_text(&text);
                btn.set_tooltip_text(Some(&format!("Version: {text}")));
                while let Some(c) = list.first_child() {
                    list.remove(&c);
                }
                if let Some(m) = dd.model() {
                    for i in 0..m.n_items() {
                        let s = m
                            .item(i)
                            .and_downcast::<gtk::StringObject>()
                            .map(|o| o.string().to_string())
                            .unwrap_or_default();
                        let row = gtk::Box::new(gtk::Orientation::Vertical, 2);
                        row.set_margin_top(3);
                        row.set_margin_bottom(3);
                        let l = gtk::Label::new(Some(&s));
                        l.set_xalign(0.0);
                        if i == dd.selected() {
                            l.add_css_class("heading");
                        }
                        row.append(&l);
                        let note = notes.borrow().get(i as usize).cloned().unwrap_or_default();
                        if !note.is_empty() && note != s {
                            let n = gtk::Label::new(Some(&note));
                            n.set_xalign(0.0);
                            n.set_wrap(true);
                            n.set_max_width_chars(34);
                            n.add_css_class("caption");
                            n.add_css_class("dim-label");
                            row.append(&n);
                        }
                        list.append(&row);
                    }
                }
            };
            let u = update.clone();
            version_dd.connect_selected_notify(move |dd| u(dd));
            version_dd.connect_model_notify(move |dd| update(dd));
        }
        title_box.append(&version_btn);
        header.pack_start(&title_box);
        header.set_title_widget(Some(&gtk::Box::new(gtk::Orientation::Horizontal, 0)));

        // labelled, so the study tools can be found without hovering
        let panel_toggle = gtk::ToggleButton::new();
        panel_toggle.set_child(Some(
            &adw::ButtonContent::builder()
                .icon_name("sidebar-show-right-symbolic")
                .label(crate::i18n::t("Study").as_str())
                .build(),
        ));
        panel_toggle.set_tooltip_text(Some(crate::i18n::t("Study panel").as_str()));
        // The word only shows while the panel is closed: open, the panel
        // names itself and the header needs the room.
        panel_toggle.connect_active_notify(|b| {
            if let Some(c) = b.child().and_downcast::<adw::ButtonContent>() {
                c.set_label(if b.is_active() { "" } else { crate::i18n::ts("Study") });
            }
        });
        panel_toggle.set_active(true);
        let par_toggle = gtk::ToggleButton::new();
        par_toggle.set_icon_name("view-dual-symbolic");
        par_toggle.set_tooltip_text(Some(crate::i18n::t("Side by side").as_str()));
        let par_dd = gtk::DropDown::from_strings(&[]);
        par_dd.set_visible(false);
        par_dd.set_enable_search(true);
        par_dd.set_tooltip_text(Some(crate::i18n::t("Second version").as_str()));
        let il_toggle = gtk::ToggleButton::new();
        // "Αω" (Alpha and Omega) in the scripture serif; the font list lives in CSS, since
        // Pango markup can't fall back through a list on Windows
        let il_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        let il_aa = gtk::Label::new(Some("Αω"));
        il_aa.add_css_class("il-label");
        il_box.append(&il_aa);
        il_box.append(&gtk::Label::new(Some(crate::i18n::t("Original").as_str())));
        il_toggle.set_child(Some(&il_box));
        il_toggle.set_tooltip_text(Some(crate::i18n::t("Interlinear: Greek and Hebrew word by word").as_str()));
        header.pack_end(&panel_toggle);
        header.pack_end(&par_dd);
        header.pack_end(&il_toggle);
        let listen_toggle = gtk::ToggleButton::new();
        listen_toggle.set_icon_name("audio-headphones-symbolic");
        listen_toggle.set_tooltip_text(Some(crate::i18n::t("Listen to this chapter").as_str()));
        let more = gtk::MenuButton::new();
        more.set_icon_name("view-more-symbolic");
        more.set_tooltip_text(Some(crate::i18n::t("More").as_str()));
        let more_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
        more_box.set_margin_top(6);
        more_box.set_margin_bottom(6);
        more_box.set_margin_start(6);
        more_box.set_margin_end(6);
        let menu_item = |label: &str, icon: &str| {
            let b = gtk::Button::new();
            let r = gtk::Box::new(gtk::Orientation::Horizontal, 10);
            r.append(&gtk::Image::from_icon_name(icon));
            let l = gtk::Label::new(Some(label));
            l.set_xalign(0.0);
            r.append(&l);
            b.set_child(Some(&r));
            b.add_css_class("flat");
            more_box.append(&b);
            b
        };
        // side by side lives in the menu: the header has no room to spare
        let side_b = menu_item("Side by side", "view-dual-symbolic");
        {
            let t = par_toggle.clone();
            side_b.connect_clicked(move |_| t.set_active(!t.is_active()));
            let b = side_b.clone();
            par_toggle.connect_sensitive_notify(move |t| b.set_sensitive(t.is_sensitive()));
        }
        let listen_b = menu_item("Listen to this chapter", "audio-headphones-symbolic");
        let present_b = menu_item("Present full screen", "view-fullscreen-symbolic");
        let arcs_b = menu_item("Cross-reference rainbow", "emblem-shared-symbolic");
        let export_b = menu_item("Export chapter as Markdown", "document-save-symbolic");
        let copy_ch_b = menu_item("Copy chapter", "edit-copy-symbolic");
        // text size lives here now, with its shortcuts shown so they can be learnt
        let with_key = |b: &gtk::Button, key: &str| {
            if let Some(r) = b.child().and_downcast::<gtk::Box>() {
                if let Some(l) = r.last_child().and_downcast::<gtk::Label>() {
                    l.set_hexpand(true);
                }
                let k = gtk::Label::new(Some(key));
                k.add_css_class("dim-label");
                k.add_css_class("caption");
                r.append(&k);
            }
        };
        let bigger_b = menu_item("Larger text", "zoom-in-symbolic");
        with_key(&bigger_b, "Ctrl +");
        let smaller_b = menu_item("Smaller text", "zoom-out-symbolic");
        with_key(&smaller_b, "Ctrl −");
        for (b, step) in [(&bigger_b, 1), (&smaller_b, -1)] {
            let c = ctx.clone();
            b.connect_clicked(move |_| c.zoom(step));
        }
        let more_pop = gtk::Popover::new();
        more_pop.set_child(Some(&more_box));
        more.set_popover(Some(&more_pop));
        {
            let p = more_pop.clone();
            side_b.connect_clicked(move |_| p.popdown());
        }
        header.pack_end(&more);
        let player = crate::audio::Player::new(ctx);

        // ----- passage picker -----
        let picker_stack = gtk::Stack::new();
        picker_stack.set_transition_type(gtk::StackTransitionType::SlideLeftRight);
        let go_entry = gtk::SearchEntry::new();
        go_entry.set_placeholder_text(Some(crate::i18n::t("Go to… John 3:16").as_str()));
        let book_list = gtk::ListBox::new();
        book_list.add_css_class("navigation-sidebar");
        let books_scroll = gtk::ScrolledWindow::builder()
            .child(&book_list)
            .min_content_height(380)
            .max_content_height(460)
            .propagate_natural_height(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build();
        let books_page = gtk::Box::new(gtk::Orientation::Vertical, 8);
        books_page.append(&go_entry);
        books_page.append(&books_scroll);
        let chapter_box = gtk::FlowBox::new();
        chapter_box.set_max_children_per_line(8);
        chapter_box.set_min_children_per_line(6);
        chapter_box.set_selection_mode(gtk::SelectionMode::None);
        chapter_box.set_homogeneous(true);
        chapter_box.set_row_spacing(4);
        chapter_box.set_column_spacing(4);
        let back = gtk::Button::from_icon_name("go-previous-symbolic");
        back.set_tooltip_text(Some(crate::i18n::t("Back to books").as_str()));
        back.add_css_class("flat");
        let picker_book_title = gtk::Label::new(None);
        picker_book_title.add_css_class("heading");
        let ch_head = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        ch_head.append(&back);
        ch_head.append(&picker_book_title);
        let ch_scroll = gtk::ScrolledWindow::builder()
            .child(&chapter_box)
            .min_content_height(200)
            .max_content_height(420)
            .propagate_natural_height(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build();
        let ch_page = gtk::Box::new(gtk::Orientation::Vertical, 8);
        ch_page.append(&ch_head);
        ch_page.append(&ch_scroll);
        picker_stack.add_named(&books_page, Some("books"));
        picker_stack.add_named(&ch_page, Some("chapters"));
        picker_stack.set_size_request(320, -1);
        picker_stack.set_vhomogeneous(false);
        picker_stack.set_interpolate_size(true);
        passage_popover.set_child(Some(&picker_stack));

        // ----- text -----
        let list = gtk::ListBox::new();
        list.add_css_class("verses");
        list.set_selection_mode(gtk::SelectionMode::None);
        let clamp = adw::Clamp::builder()
            .maximum_size(760)
            .tightening_threshold(600)
            .child(&list)
            .build();
        clamp.set_margin_top(18);
        clamp.set_margin_bottom(48);
        clamp.set_margin_start(24);
        clamp.set_margin_end(24);
        let tip_label = gtk::Label::new(None);
        tip_label.set_wrap(true);
        tip_label.set_xalign(0.0);
        tip_label.set_hexpand(true);
        let tip_btn = gtk::Button::with_label(crate::i18n::t("Open Downloads").as_str());
        tip_btn.add_css_class("suggested-action");
        let tip_close = gtk::Button::from_icon_name("window-close-symbolic");
        tip_close.add_css_class("flat");
        tip_close.add_css_class("circular");
        tip_close.set_tooltip_text(Some(crate::i18n::t("Dismiss").as_str()));
        let tip_row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        tip_row.add_css_class("tip-bar");
        tip_row.append(&gtk::Image::from_icon_name("dialog-information-symbolic"));
        tip_row.append(&tip_label);
        tip_row.append(&tip_btn);
        tip_row.append(&tip_close);
        let tip = gtk::Revealer::builder()
            .child(&tip_row)
            .transition_type(gtk::RevealerTransitionType::SlideDown)
            .build();
        let text_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        text_box.append(&tip);
        text_box.append(&player.tip);
        text_box.append(&clamp);
        let scroller = gtk::ScrolledWindow::builder()
            .child(&text_box)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .build();
        scroller.add_css_class("reader-page");
        let empty = adw::StatusPage::builder()
            .icon_name("bibleapp-book-symbolic")
            .title(crate::i18n::t("No Bibles yet").as_str())
            .description(crate::i18n::t("Download a Bible from Downloads to start reading. It suggests the one that turns on the most features, and everything works offline once it’s installed.").as_str())
            .build();
        let empty_btn = gtk::Button::with_label(crate::i18n::t("Open Downloads").as_str());
        empty_btn.add_css_class("pill");
        empty_btn.add_css_class("suggested-action");
        empty_btn.set_halign(gtk::Align::Center);
        empty.set_child(Some(&empty_btn));
        let content_stack = gtk::Stack::new();
        content_stack.set_transition_type(gtk::StackTransitionType::Crossfade);
        content_stack.add_named(&scroller, Some("text"));
        content_stack.add_named(&empty, Some("empty"));

        // ----- interlinear pane -----
        let il_title = gtk::Label::new(None);
        il_title.add_css_class("heading");
        il_title.set_xalign(0.0);
        let il_sub = gtk::Label::new(None);
        il_sub.add_css_class("caption");
        il_sub.add_css_class("dim-label");
        il_sub.set_xalign(0.0);
        let il_titles = gtk::Box::new(gtk::Orientation::Vertical, 2);
        il_titles.set_hexpand(true);
        il_titles.append(&il_title);
        il_titles.append(&il_sub);
        let il_prev = icon_button("go-up-symbolic", "Previous verse");
        il_prev.add_css_class("flat");
        let il_next = icon_button("go-down-symbolic", "Next verse");
        il_next.add_css_class("flat");
        let il_head = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        il_head.set_margin_start(16);
        il_head.set_margin_end(8);
        il_head.set_margin_top(10);
        il_head.set_margin_bottom(10);
        il_head.append(&il_titles);
        il_head.append(&il_prev);
        il_head.append(&il_next);
        let il_flow = gtk::FlowBox::new();
        il_flow.set_selection_mode(gtk::SelectionMode::None);
        il_flow.set_max_children_per_line(12);
        il_flow.set_min_children_per_line(2);
        il_flow.set_row_spacing(6);
        il_flow.set_column_spacing(6);
        il_flow.set_valign(gtk::Align::Start);
        il_flow.set_margin_start(12);
        il_flow.set_margin_end(12);
        il_flow.set_margin_top(12);
        il_flow.set_margin_bottom(24);
        let il_scroll = gtk::ScrolledWindow::builder()
            .child(&il_flow)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .build();
        let il_missing = adw::StatusPage::builder()
            .icon_name("bibleapp-words-symbolic")
            .title(crate::i18n::t("Interlinear not downloaded").as_str())
            .build();
        il_missing.add_css_class("compact");
        let il_dl_btn = gtk::Button::with_label(crate::i18n::t("Download").as_str());
        il_dl_btn.add_css_class("pill");
        il_dl_btn.add_css_class("suggested-action");
        il_dl_btn.set_halign(gtk::Align::Center);
        let il_progress = gtk::ProgressBar::new();
        il_progress.set_visible(false);
        let il_dl_box = gtk::Box::new(gtk::Orientation::Vertical, 12);
        il_dl_box.append(&il_dl_btn);
        il_dl_box.append(&il_progress);
        il_missing.set_child(Some(&il_dl_box));
        let il_loading = crate::loader::new(72);
        let il_stack = gtk::Stack::new();
        il_stack.set_transition_type(gtk::StackTransitionType::Crossfade);
        il_stack.add_named(&il_scroll, Some("words"));
        il_stack.add_named(&il_missing, Some("missing"));
        il_stack.add_named(&il_loading, Some("loading"));
        il_stack.set_vexpand(true);
        let il_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        il_box.add_css_class("il-pane");
        il_box.set_width_request(380);
        il_box.append(&il_head);
        il_box.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        il_box.append(&il_stack);
        let il_revealer = gtk::Revealer::builder()
            .child(&il_box)
            .transition_type(gtk::RevealerTransitionType::SlideLeft)
            .transition_duration(280)
            .reveal_child(false)
            .build();
        let main_area = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        content_stack.set_hexpand(true);
        main_area.append(&content_stack);
        main_area.append(&il_revealer);

        // ----- study panel -----
        let panel_stack = adw::ViewStack::new();
        let com_dd = gtk::DropDown::from_strings(&[]);
        com_dd.set_enable_search(true);
        let com_ref = gtk::Label::new(None);
        com_ref.add_css_class("dim-label");
        com_ref.add_css_class("caption-heading");
        com_ref.set_xalign(0.0);
        let com_text = gtk::Label::new(None);
        com_text.set_wrap(true);
        com_text.set_xalign(0.0);
        com_text.set_selectable(true);
        com_text.set_use_markup(true);
        com_text.add_css_class("commentary");
        let com_body = gtk::Box::new(gtk::Orientation::Vertical, 10);
        com_body.set_margin_start(16);
        com_body.set_margin_end(16);
        com_body.set_margin_top(8);
        com_body.set_margin_bottom(24);
        com_body.append(&com_dd);
        com_body.append(&com_ref);
        com_body.append(&com_text);
        let com_none = adw::StatusPage::builder()
            .icon_name("bibleapp-book-symbolic")
            .title(crate::i18n::t("No commentaries").as_str())
            .description(crate::i18n::t("Install Calvin, Matthew Henry, the Geneva notes and more from Downloads. Only download what you want.").as_str())
            .build();
        com_none.add_css_class("compact");
        let com_get = gtk::Button::with_label(crate::i18n::t("Get commentaries").as_str());
        com_get.add_css_class("pill");
        com_get.set_halign(gtk::Align::Center);
        com_none.set_child(Some(&com_get));
        let com_stack = gtk::Stack::new();
        com_stack.add_named(
            &gtk::ScrolledWindow::builder()
                .child(&com_body)
                .hscrollbar_policy(gtk::PolicyType::Never)
                .vexpand(true)
                .build(),
            Some("text"),
        );
        com_stack.add_named(&com_none, Some("none"));
        panel_stack.add_titled_with_icon(
            &com_stack,
            Some("commentary"),
            crate::i18n::ts("Commentary"),
            "bibleapp-book-symbolic",
        );

        let note_ref = gtk::Label::new(Some(crate::i18n::t("Select a verse to write a note").as_str()));
        note_ref.add_css_class("heading");
        note_ref.set_xalign(0.0);
        let hl_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        let note_view = gtk::TextView::new();
        note_view.set_wrap_mode(gtk::WrapMode::WordChar);
        note_view.add_css_class("note-editor");
        note_view.set_vexpand(true);
        note_view.set_top_margin(10);
        note_view.set_bottom_margin(10);
        note_view.set_left_margin(12);
        note_view.set_right_margin(12);
        let note_frame = gtk::Frame::new(None);
        note_frame.set_child(Some(&note_view));
        let note_status = gtk::Label::new(None);
        note_status.add_css_class("dim-label");
        note_status.add_css_class("caption");
        note_status.set_xalign(0.0);
        note_status.set_wrap(true);
        let notes_body = gtk::Box::new(gtk::Orientation::Vertical, 10);
        notes_body.set_margin_start(16);
        notes_body.set_margin_end(16);
        notes_body.set_margin_top(8);
        notes_body.set_margin_bottom(16);
        notes_body.append(&note_ref);
        notes_body.append(&hl_box);
        notes_body.append(&note_frame);
        notes_body.append(&note_status);
        panel_stack.add_titled_with_icon(
            &notes_body,
            Some("notes"),
            crate::i18n::ts("Notes"),
            "document-edit-symbolic",
        );

        let w_lemma = gtk::Label::new(None);
        w_lemma.add_css_class("orig-lemma");
        w_lemma.set_xalign(0.0);
        w_lemma.set_selectable(true);
        let w_sub = gtk::Label::new(None);
        w_sub.add_css_class("dim-label");
        w_sub.set_xalign(0.0);
        let w_form = gtk::Label::new(None);
        w_form.set_wrap(true);
        w_form.set_xalign(0.0);
        w_form.set_use_markup(true);
        let w_gloss = gtk::Label::new(None);
        w_gloss.add_css_class("title-4");
        w_gloss.set_wrap(true);
        w_gloss.set_xalign(0.0);
        let w_def = gtk::Label::new(None);
        w_def.set_wrap(true);
        w_def.set_xalign(0.0);
        w_def.set_selectable(true);
        w_def.set_use_markup(true);
        w_def.add_css_class("commentary");
        let w_dicts = gtk::Box::new(gtk::Orientation::Vertical, 14);
        let w_search = gtk::Button::with_label(crate::i18n::t("Word study").as_str());
        w_search.add_css_class("pill");
        w_search.set_tooltip_text(Some(
            "Every use of this word, how it is translated and where it appears",
        ));
        w_search.set_halign(gtk::Align::Start);
        let w_credit = gtk::Label::new(Some(bible_step::CREDIT));
        w_credit.add_css_class("caption");
        w_credit.add_css_class("dim-label");
        w_credit.set_wrap(true);
        w_credit.set_xalign(0.0);
        let w_body = gtk::Box::new(gtk::Orientation::Vertical, 10);
        w_body.set_margin_start(16);
        w_body.set_margin_end(16);
        w_body.set_margin_top(8);
        w_body.set_margin_bottom(24);
        for wdg in [
            w_lemma.upcast_ref::<gtk::Widget>(),
            w_sub.upcast_ref(),
            w_form.upcast_ref(),
            w_gloss.upcast_ref(),
            w_def.upcast_ref(),
            w_dicts.upcast_ref(),
            w_search.upcast_ref(),
            w_credit.upcast_ref(),
        ] {
            w_body.append(wdg);
        }
        let w_empty = adw::StatusPage::builder()
            .icon_name("bibleapp-words-symbolic")
            .title(crate::i18n::t("Pick a word").as_str())
            .description(crate::i18n::t("Tap a Greek or Hebrew word to see its meaning, grammar and lexicon entry. The lexicon comes with the Greek and Hebrew download (STEP Bible), so nothing else is needed.").as_str())
            .build();
        w_empty.add_css_class("compact");
        // turn it on from here, not only from the header
        {
            let b = gtk::Button::with_label(crate::i18n::t("Show Greek and Hebrew").as_str());
            b.add_css_class("pill");
            b.add_css_class("suggested-action");
            b.set_halign(gtk::Align::Center);
            let t = il_toggle.clone();
            b.connect_clicked(move |_| t.set_active(true));
            let b2 = b.clone();
            il_toggle.connect_active_notify(move |t| b2.set_visible(!t.is_active()));
            w_empty.set_child(Some(&b));
        }
        let w_stack = gtk::Stack::new();
        w_stack.add_named(
            &gtk::ScrolledWindow::builder()
                .child(&w_body)
                .hscrollbar_policy(gtk::PolicyType::Never)
                .vexpand(true)
                .build(),
            Some("info"),
        );
        w_stack.add_named(&w_empty, Some("empty"));
        w_stack.set_visible_child_name("empty");
        // word details live inside the Original pane, under the Greek and
        // Hebrew, and appear once a word is tapped
        w_stack.set_visible(false);
        il_box.remove(&il_stack);
        let il_paned = gtk::Paned::new(gtk::Orientation::Vertical);
        il_paned.set_start_child(Some(&il_stack));
        il_paned.set_end_child(Some(&w_stack));
        il_paned.set_shrink_start_child(false);
        il_paned.set_shrink_end_child(false);
        il_paned.set_position(420);
        il_paned.set_vexpand(true);
        il_box.append(&il_paned);
        {
            let ws = w_stack.clone();
            il_toggle.connect_active_notify(move |t| {
                if !t.is_active() {
                    ws.set_visible(false);
                }
            });
        }
        let map = crate::map::MapView::new(ctx);
        panel_stack.add_titled_with_icon(&map.root, Some("map"), crate::i18n::ts("Map"), "mark-location-symbolic");

        let panel_header = adw::HeaderBar::new();
        panel_header.set_show_start_title_buttons(false);
        let switcher = adw::ViewSwitcher::new();
        switcher.set_stack(Some(&panel_stack));
        switcher.set_policy(adw::ViewSwitcherPolicy::Narrow);
        panel_header.set_title_widget(Some(&switcher));
        let panel_tv = adw::ToolbarView::new();
        panel_tv.add_top_bar(&panel_header);
        panel_tv.set_content(Some(&panel_stack));

        let content_tv = adw::ToolbarView::new();
        content_tv.add_top_bar(&header);
        content_tv.set_content(Some(&main_area));
        content_tv.add_bottom_bar(&player.root);
        let split = adw::OverlaySplitView::builder()
            .content(&content_tv)
            .sidebar(&panel_tv)
            .sidebar_position(gtk::PackType::End)
            .show_sidebar(true)
            .min_sidebar_width(260.0)
            .max_sidebar_width(400.0)
            .sidebar_width_fraction(0.32)
            .build();
        split
            .bind_property("show-sidebar", &panel_toggle, "active")
            .bidirectional()
            .sync_create()
            .build();
        split
            .bind_property("show-sidebar", &header, "show-end-title-buttons")
            .invert_boolean()
            .sync_create()
            .build();

        let root = split.clone();

        let cfg = ctx.config.borrow();
        let state = State {
            book: cfg.book.clone(),
            chapter: cfg.chapter.max(1),
            sel: 0,
            notes: ChapterNotes::default(),
            verses: vec![],
        };
        drop(cfg);

        let r = Rc::new(Reader {
            ctx: ctx.clone(),
            link_at: Cell::new(None),
            root,
            split,
            passage_btn,
            verse_btn: verse_btn.clone(),
            verse_flow,
            nav_box: title_box.clone(),
            panel_toggle: panel_toggle.clone(),
            passage_popover,
            book_list,
            chapter_box,
            picker_stack,
            picker_book_title,
            version_dd,
            versions: RefCell::new(vec![]),
            version_notes,
            par_toggle,
            par_dd,
            clamp,
            list,
            scroller,
            content_stack,
            tip,
            panel_stack,
            map,
            intro_map: RefCell::new(None),
            com_dd,
            coms: RefCell::new(vec![]),
            com_ref,
            com_text,
            com_stack,
            note_ref,
            note_view,
            note_status,
            hl_box,
            il_toggle: il_toggle.clone(),
            listen_toggle: listen_toggle.clone(),
            player: player.clone(),
            il_revealer,
            il_title,
            il_sub,
            il_flow,
            il_stack,
            il_missing,
            il_dl_btn: il_dl_btn.clone(),
            il_progress,
            side_was: Cell::new(false),
            w_stack,
            w_lemma,
            w_sub,
            w_form,
            w_gloss,
            w_def,
            w_dicts,
            w_query: RefCell::new(String::new()),
            me: RefCell::new(Weak::new()),
            state: RefCell::new(state),
            loading_note: Cell::new(false),
            save_gen: Cell::new(0),
            filling: Cell::new(false),
        });

        // ----- signals -----
        *r.me.borrow_mut() = Rc::downgrade(&r);
        let w = Rc::downgrade(&r);
        il_toggle.connect_toggled(move |t| {
            let Some(r) = w.upgrade() else { return };
            if t.is_active() {
                // make room first, so the window never has to grow
                r.side_was.set(r.ctx.set_sidebar(false));
                r.il_revealer.set_visible(true);
                r.il_revealer.set_reveal_child(true);
                r.update_interlinear();
            } else {
                r.il_revealer.set_reveal_child(false);
                if r.side_was.get() {
                    r.ctx.set_sidebar(true);
                }
            }
        });
        // once the slide-out finishes, take the pane out of the layout so the text gets the room
        r.il_revealer.set_visible(false);
        r.il_revealer.connect_child_revealed_notify(|rv| {
            if !rv.is_child_revealed() && !rv.reveals_child() {
                rv.set_visible(false);
            }
        });
        let w = Rc::downgrade(&r);
        il_prev.connect_clicked(move |_| {
            if let Some(r) = w.upgrade() {
                r.step_verse(-1);
            }
        });
        let w = Rc::downgrade(&r);
        il_next.connect_clicked(move |_| {
            if let Some(r) = w.upgrade() {
                r.step_verse(1);
            }
        });
        let w = Rc::downgrade(&r);
        il_dl_btn.connect_clicked(move |_| {
            if let Some(r) = w.upgrade() {
                r.download_pack();
            }
        });
        let c = ctx.clone();
        w_search.connect_clicked({
            let w = Rc::downgrade(&r);
            move |_| {
                if let Some(r) = w.upgrade() {
                    let q = r.w_query.borrow().clone();
                    c.open_word_study(&q);
                }
            }
        });
        {
            let lbl = &r.w_def;
            let c = ctx.clone();
            lbl.connect_activate_link(move |_, uri| {
                if let Some(rf) = Ref::from_uri(uri) {
                    c.open_ref(&rf);
                }
                glib::Propagation::Stop
            });
        }
        let w = Rc::downgrade(&r);
        prev.connect_clicked(move |_| {
            if let Some(r) = w.upgrade() {
                r.step_chapter(-1)
            }
        });
        let w = Rc::downgrade(&r);
        next.connect_clicked(move |_| {
            if let Some(r) = w.upgrade() {
                r.step_chapter(1)
            }
        });
        let w = Rc::downgrade(&r);
        r.version_dd.connect_selected_notify(move |dd| {
            if let Some(r) = w.upgrade() {
                if r.filling.get() {
                    return;
                }
                if let Some(name) = r.versions.borrow().get(dd.selected() as usize) {
                    r.ctx.config.borrow_mut().bible = name.clone();
                }
                r.ctx.save_config();
                r.fill_books();
                r.render();
            }
        });
        let w = Rc::downgrade(&r);
        r.par_toggle.connect_toggled(move |t| {
            if let Some(r) = w.upgrade() {
                r.par_dd.set_visible(t.is_active());
                // two versions need the width, so the study panel steps aside
                if t.is_active() {
                    r.panel_toggle.set_active(false);
                }
                r.clamp
                    .set_maximum_size(if t.is_active() { 1200 } else { 760 });
                r.render();
            }
        });
        let w = Rc::downgrade(&r);
        r.par_dd.connect_selected_notify(move |dd| {
            if let Some(r) = w.upgrade() {
                if r.filling.get() {
                    return;
                }
                if let Some(name) = r.versions.borrow().get(dd.selected() as usize) {
                    r.ctx.config.borrow_mut().parallel = name.clone();
                }
                r.ctx.save_config();
                r.render();
            }
        });
        let w = Rc::downgrade(&r);
        r.com_dd.connect_selected_notify(move |dd| {
            if let Some(r) = w.upgrade() {
                if r.filling.get() {
                    return;
                }
                if let Some(name) = r.coms.borrow().get(dd.selected() as usize) {
                    r.ctx.config.borrow_mut().commentary = name.clone();
                }
                r.ctx.save_config();
                r.update_commentary();
            }
        });
        let w = Rc::downgrade(&r);
        r.list.connect_row_activated(move |_, row| {
            if let Some(r) = w.upgrade() {
                r.verse_clicked(row);
            }
        });
        let w = Rc::downgrade(&r);
        r.book_list.connect_row_activated(move |_, row| {
            if let Some(r) = w.upgrade() {
                let name = row.widget_name().to_string();
                r.show_chapters(&name);
            }
        });
        let ps = r.picker_stack.clone();
        back.connect_clicked(move |_| ps.set_visible_child_name("books"));
        let w = Rc::downgrade(&r);
        r.passage_popover.connect_show(move |_| {
            if let Some(r) = w.upgrade() {
                r.picker_stack.set_visible_child_name("books");
            }
        });
        let w = Rc::downgrade(&r);
        go_entry.connect_activate(move |e| {
            if let Some(r) = w.upgrade() {
                if let Some(rf) = bible_core::refs::parse(&e.text()) {
                    r.passage_popover.popdown();
                    e.set_text("");
                    r.open(&rf);
                }
            }
        });
        let c = ctx.clone();
        empty_btn.connect_clicked(move |_| c.go("library"));
        let c = ctx.clone();
        com_get.connect_clicked(move |_| c.go("library"));
        let c = ctx.clone();
        let tipr = r.tip.clone();
        tip_btn.connect_clicked(move |_| {
            tipr.set_reveal_child(false);
            c.config
                .borrow_mut()
                .dismissed_tips
                .push("commentary".into());
            c.save_config();
            c.go("library");
        });
        let c = ctx.clone();
        let tipr = r.tip.clone();
        tip_close.connect_clicked(move |_| {
            tipr.set_reveal_child(false);
            c.config
                .borrow_mut()
                .dismissed_tips
                .push("commentary".into());
            c.save_config();
        });
        tip_label.set_markup("<b>Commentaries can sit beside the text.</b> Download Calvin, Matthew Henry and others from Downloads when you want them.");
        let w = Rc::downgrade(&r);
        r.note_view.buffer().connect_changed(move |_| {
            if let Some(r) = w.upgrade() {
                r.schedule_note_save();
            }
        });
        let c = ctx.clone();
        r.com_text.connect_activate_link(move |_, uri| {
            if let Some(rf) = Ref::from_uri(uri) {
                c.open_ref(&rf);
            }
            glib::Propagation::Stop
        });

        let w = Rc::downgrade(&r);
        ctx.set_open_ref(move |rf| {
            if let Some(r) = w.upgrade() {
                r.open(rf);
            }
        });

        r.fill_versions();
        r.fill_books();
        r.render();
        {
            let pl = r.player.clone();
            r.listen_toggle.connect_toggled(move |t| {
                if t.is_active() {
                    pl.show();
                } else if pl.is_shown() {
                    pl.hide();
                }
            });
            let lt = r.listen_toggle.clone();
            r.player
                .root
                .connect_reveal_child_notify(move |rv| lt.set_active(rv.reveals_child()));
            let w = Rc::downgrade(&r);
            r.player.set_on_next(move || {
                if let Some(r) = w.upgrade() {
                    r.step_chapter(1);
                }
            });
            let w = Rc::downgrade(&r);
            r.player.set_on_prev(move || {
                if let Some(r) = w.upgrade() {
                    r.step_chapter(-1);
                }
            });
            let w = Rc::downgrade(&r);
            ctx.set_play_audio(move |book, chapter| {
                if let Some(r) = w.upgrade() {
                    r.open(&Ref::new(book, chapter, 0));
                    r.player.play_now();
                }
            });
        }
        {
            let c = r.ctx.clone();
            let p = more_pop.clone();
            arcs_b.connect_clicked(move |_| {
                p.popdown();
                c.go("crossrefs");
            });
            let lt = r.listen_toggle.clone();
            let p = more_pop.clone();
            listen_b.connect_clicked(move |_| {
                p.popdown();
                lt.set_active(true);
            });
            let w = Rc::downgrade(&r);
            let p = more_pop.clone();
            present_b.connect_clicked(move |_| {
                p.popdown();
                if let Some(r) = w.upgrade() {
                    if let Some(m) = r.primary() {
                        let st = r.state.borrow();
                        crate::present::present(&r.ctx, m, &st.book, st.chapter.max(1), st.sel);
                    }
                }
            });
            let w = Rc::downgrade(&r);
            let p = more_pop.clone();
            export_b.connect_clicked(move |_| {
                p.popdown();
                if let Some(r) = w.upgrade() {
                    if let Some(m) = r.primary() {
                        let (b, c) = {
                            let st = r.state.borrow();
                            (st.book.clone(), st.chapter.max(1))
                        };
                        crate::export::export_chapter(&r.ctx, &m, &b, c);
                    }
                }
            });
            let w = Rc::downgrade(&r);
            let p = more_pop.clone();
            copy_ch_b.connect_clicked(move |b| {
                p.popdown();
                if let Some(r) = w.upgrade() {
                    if let Some(m) = r.primary() {
                        let (bk, c) = {
                            let st = r.state.borrow();
                            (st.book.clone(), st.chapter.max(1))
                        };
                        let name = Ref::new(&bk, c, 0).book_name();
                        let mut text = format!("{name} {c} ({})\n\n", m.abbreviation());
                        for (v, _) in m.chapter(&bk, c) {
                            if let Some(t) = m.verse_plain(&bk, c, v) {
                                text.push_str(&format!("{v} {}\n", t.trim()));
                            }
                        }
                        b.clipboard().set_text(&text);
                        r.ctx.toast(crate::i18n::t("Chapter copied").as_str());
                    }
                }
            });
        }
        {
            let w = Rc::downgrade(&r);
            verse_pop.connect_show(move |_| {
                if let Some(r) = w.upgrade() {
                    r.fill_verses();
                }
            });
        }
        if std::env::var("BIBLE_APP_PRESENT").is_ok() {
            let rr = r.clone();
            glib::timeout_add_local_once(std::time::Duration::from_millis(800), move || {
                if let Some(m) = rr.primary() {
                    let st = rr.state.borrow();
                    crate::present::present(&rr.ctx, m, &st.book, st.chapter.max(1), st.sel);
                }
            });
        }
        if std::env::var("BIBLE_APP_AUDIO").is_ok() {
            r.player.show();
        }
        if std::env::var("BIBLE_APP_PARALLEL").is_ok() {
            r.par_toggle.set_active(true);
        }
        if std::env::var("BIBLE_APP_INTERLINEAR").is_ok() {
            r.il_toggle.set_active(true);
        }
        if let Ok(t) = std::env::var("BIBLE_APP_TAB") {
            r.panel_stack.set_visible_child_name(&t);
        }
        r
    }

    fn primary(&self) -> Option<Arc<Module>> {
        self.ctx.bible()
    }

    fn parallel(&self) -> Option<Arc<Module>> {
        if !self.par_toggle.is_active() {
            return None;
        }
        let name = self.ctx.config.borrow().parallel.clone();
        let lib = self.ctx.lib();
        lib.get(&name).filter(|m| m.is_readable())
    }

    fn commentary(&self) -> Option<Arc<Module>> {
        let name = self.ctx.config.borrow().commentary.clone();
        let lib = self.ctx.lib();
        lib.get(&name)
            .filter(|m| m.kind == Kind::Commentary && m.is_readable())
            .or_else(|| {
                lib.of_kind(Kind::Commentary)
                    .into_iter()
                    .find(|m| m.is_readable())
            })
    }

    fn fill_versions(&self) {
        self.filling.set(true);
        let lib = self.ctx.lib();
        let bibles: Vec<Arc<Module>> = lib
            .of_kind(Kind::Bible)
            .into_iter()
            .filter(|m| m.is_readable())
            .collect();
        let names: Vec<String> = bibles.iter().map(|m| m.name.clone()).collect();
        let labels: Vec<String> = bibles.iter().map(|m| m.abbreviation()).collect();
        let lab: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
        *self.version_notes.borrow_mut() = bibles.iter().map(|m| version_note(m)).collect();
        let model = gtk::StringList::new(&lab);
        self.version_dd.set_model(Some(&model));
        self.par_dd.set_model(Some(&gtk::StringList::new(&lab)));
        let cfg = self.ctx.config.borrow().clone();
        let cur = self.primary().map(|m| m.name.clone()).unwrap_or_default();
        if let Some(i) = names.iter().position(|n| *n == cur) {
            self.version_dd.set_selected(i as u32);
        }
        let par = if names.contains(&cfg.parallel) {
            cfg.parallel.clone()
        } else {
            names
                .iter()
                .find(|n| **n != cur)
                .cloned()
                .unwrap_or_default()
        };
        if let Some(i) = names.iter().position(|n| *n == par) {
            self.par_dd.set_selected(i as u32);
            self.ctx.config.borrow_mut().parallel = par;
        }
        self.par_toggle.set_sensitive(names.len() > 1);
        let have_bibles = !names.is_empty();
        *self.versions.borrow_mut() = names;

        let coms: Vec<Arc<Module>> = lib
            .of_kind(Kind::Commentary)
            .into_iter()
            .filter(|m| m.is_readable())
            .collect();
        let cl: Vec<String> = coms.iter().map(|m| m.description()).collect();
        let clr: Vec<&str> = cl.iter().map(|s| s.as_str()).collect();
        self.com_dd.set_model(Some(&gtk::StringList::new(&clr)));
        let curc = self
            .commentary()
            .map(|m| m.name.clone())
            .unwrap_or_default();
        let cnames: Vec<String> = coms.iter().map(|m| m.name.clone()).collect();
        if let Some(i) = cnames.iter().position(|n| *n == curc) {
            self.com_dd.set_selected(i as u32);
        }
        self.com_stack
            .set_visible_child_name(if cnames.is_empty() { "none" } else { "text" });
        let show_tip = cnames.is_empty()
            && cfg.feature_tips
            && !cfg.dismissed_tips.iter().any(|t| t == "commentary")
            && have_bibles;
        self.tip.set_reveal_child(show_tip);
        *self.coms.borrow_mut() = cnames;
        self.filling.set(false);
    }

    fn fill_books(&self) {
        while let Some(c) = self.book_list.first_child() {
            self.book_list.remove(&c);
        }
        let Some(m) = self.primary() else { return };
        let apoc = self.ctx.config.borrow().show_apocrypha;
        let sys = m.versification();
        let mut last_testament = 0;
        let books = m.books();
        for osis in &books {
            if !apoc && v11n::is_apocrypha(osis) {
                continue;
            }
            let idx = sys.book_index(osis).unwrap_or(0);
            let testament = if v11n::is_apocrypha(osis) {
                3
            } else if idx < sys.ot_books {
                1
            } else {
                2
            };
            if testament != last_testament {
                let h = gtk::Label::new(Some(match testament {
                    1 => "Old Testament",
                    2 => "New Testament",
                    _ => "Apocrypha",
                }));
                h.add_css_class("dim-label");
                h.add_css_class("caption-heading");
                h.set_xalign(0.0);
                h.set_margin_top(8);
                h.set_margin_start(6);
                let row = gtk::ListBoxRow::new();
                row.set_child(Some(&h));
                row.set_activatable(false);
                row.set_selectable(false);
                self.book_list.append(&row);
                last_testament = testament;
            }
            let name = sys
                .book(osis)
                .map(|b| b.name.clone())
                .unwrap_or_else(|| osis.clone());
            let l = gtk::Label::new(Some(&name));
            l.set_xalign(0.0);
            let row = gtk::ListBoxRow::new();
            row.set_widget_name(osis);
            row.set_child(Some(&l));
            self.book_list.append(&row);
        }
        // keep the current book valid for this module
        let cur = self.state.borrow().book.clone();
        if !books.contains(&cur) {
            if let Some(first) = books
                .iter()
                .find(|b| *b == "John")
                .or(books.iter().find(|b| *b == "Matt"))
                .or(books.first())
            {
                let mut st = self.state.borrow_mut();
                st.book = first.clone();
                st.chapter = 1;
            }
        }
    }

    fn show_chapters(&self, osis: &str) {
        let Some(m) = self.primary() else { return };
        let Some(b) = m.versification().book(osis) else {
            return;
        };
        while let Some(c) = self.chapter_box.first_child() {
            self.chapter_box.remove(&c);
        }
        self.picker_book_title.set_text(&b.name);
        let intro = gtk::Button::with_label(crate::i18n::t("Intro").as_str());
        intro.add_css_class("flat");
        intro.set_tooltip_text(Some(crate::i18n::t("Introduction to the book").as_str()));
        {
            let osis = osis.to_string();
            let pop = self.passage_popover.clone();
            let ctx = self.ctx.clone();
            intro.connect_clicked(move |_| {
                pop.popdown();
                ctx.open_ref(&Ref::new(&osis, 0, 0));
            });
        }
        self.chapter_box.insert(&intro, -1);
        for c in 1..=b.chapters() {
            let btn = gtk::Button::with_label(&c.to_string());
            btn.add_css_class("flat");
            let osis = osis.to_string();
            let pop = self.passage_popover.clone();
            let ctx = self.ctx.clone();
            btn.connect_clicked(move |_| {
                pop.popdown();
                ctx.open_ref(&Ref::new(&osis, c, 0));
            });
            self.chapter_box.insert(&btn, -1);
        }
        self.picker_stack.set_visible_child_name("chapters");
    }

    pub fn open(&self, r: &Ref) {
        {
            let mut st = self.state.borrow_mut();
            st.book = r.book.clone();
            st.chapter = r.chapter; // 0 = the book's introduction
            st.sel = r.verse;
        }
        {
            let mut c = self.ctx.config.borrow_mut();
            c.book = r.book.clone();
            c.chapter = r.chapter.max(1);
        }
        self.ctx.save_config();
        self.render();
        if r.verse > 0 {
            self.scroll_to_verse(r.verse);
        } else {
            self.scroller.vadjustment().set_value(0.0);
        }
    }

    fn step_chapter(&self, d: i32) {
        let Some(m) = self.primary() else { return };
        let sys = m.versification();
        let (book, ch) = {
            let st = self.state.borrow();
            (st.book.clone(), st.chapter)
        };
        let books = m.books();
        let apoc = self.ctx.config.borrow().show_apocrypha;
        let books: Vec<String> = books
            .into_iter()
            .filter(|b| apoc || !v11n::is_apocrypha(b))
            .collect();
        let Some(bi) = books.iter().position(|b| *b == book) else {
            return;
        };
        let max = sys.book(&book).map(|b| b.chapters()).unwrap_or(1);
        let target = if d > 0 && ch == 0 {
            Some(Ref::new(&book, 1, 0))
        } else if d > 0 {
            if ch < max {
                Some(Ref::new(&book, ch + 1, 0))
            } else {
                books.get(bi + 1).map(|b| Ref::new(b, 1, 0))
            }
        } else if ch > 1 {
            Some(Ref::new(&book, ch - 1, 0))
        } else if bi > 0 {
            let pb = &books[bi - 1];
            Some(Ref::new(
                pb,
                sys.book(pb).map(|b| b.chapters()).unwrap_or(1),
                0,
            ))
        } else {
            None
        };
        if let Some(t) = target {
            self.open(&t);
        }
    }

    fn dark(&self) -> bool {
        adw::StyleManager::default().is_dark()
    }

    /// Rebuild the verse list for the current chapter.
    pub fn render(&self) {
        while let Some(c) = self.list.first_child() {
            self.list.remove(&c);
        }
        let Some(m) = self.primary() else {
            self.content_stack.set_visible_child_name("empty");
            self.nav_box.set_visible(false);
            self.par_toggle.set_visible(false);
            self.par_dd.set_visible(false);
            self.panel_toggle.set_visible(false);
            self.split.set_show_sidebar(false);
            return;
        };
        if !self.nav_box.is_visible() {
            self.nav_box.set_visible(true);
            self.par_toggle.set_visible(true);
            self.panel_toggle.set_visible(true);
            self.split.set_show_sidebar(true);
        }
        self.par_dd.set_visible(self.par_toggle.is_active());
        self.content_stack.set_visible_child_name("text");
        let (book, chapter) = {
            let st = self.state.borrow();
            (st.book.clone(), st.chapter)
        };
        if chapter == 0 {
            self.render_intro(&m, &book);
            return;
        }
        let notes = self.ctx.notes().load(&book, chapter);
        let cfg = self.ctx.config.borrow().clone();
        let par = self.parallel();
        let book_name = m
            .versification()
            .book(&book)
            .map(|b| b.name.clone())
            .unwrap_or_else(|| book.clone());
        self.passage_btn
            .set_label(&format!("{book_name} {chapter}"));
        {
            let sel = self.state.borrow().sel;
            self.verse_btn.set_visible(true);
            self.verse_btn.set_label(&if sel > 0 {
                format!("v. {sel}")
            } else {
                "Verse".to_string()
            });
        }
        let chapters = m
            .versification()
            .book(&book)
            .map(|b| b.chapters())
            .unwrap_or(1);
        self.player
            .set_chapter(&m.name, &book, chapter, chapters, !self.tip.reveals_child());
        let dark = self.dark();

        let hbox = gtk::Box::new(gtk::Orientation::Vertical, 2);
        if cfg.vintage {
            // a printed page: running head between rules, "CHAP. I." and a fleuron
            let head = gtk::Label::new(Some(&book_name));
            head.add_css_class("running-head");
            hbox.append(&head);
            let chap = gtk::Label::new(Some(&format!("Chap. {}.", roman(chapter))));
            chap.add_css_class("chapter-heading");
            chap.add_css_class("vintage-chapter");
            hbox.append(&chap);
            let orn = gtk::Label::new(Some("❦"));
            orn.add_css_class("fleuron");
            hbox.append(&orn);
        } else {
            let heading = gtk::Label::new(Some(&format!("{book_name} {chapter}")));
            heading.add_css_class("chapter-heading");
            heading.set_xalign(0.0);
            hbox.append(&heading);
        }
        // where this passage sits in history, linked to the timeline
        let tl = bible_core::timeline::timeline();
        if let Some((start, end)) = tl.when(&book, chapter) {
            let era = tl
                .era_at(start)
                .map(|e| format!(" · {}", e.title))
                .unwrap_or_default();
            let when = bible_core::timeline::span(start, end, true);
            let btn = gtk::Button::new();
            let inner = gtk::Box::new(gtk::Orientation::Horizontal, 6);
            inner.append(&gtk::Image::from_icon_name("document-open-recent-symbolic"));
            let l = gtk::Label::new(Some(&format!("{when}{era}")));
            inner.append(&l);
            btn.set_child(Some(&inner));
            btn.add_css_class("flat");
            btn.add_css_class("when-chip");
            btn.set_halign(gtk::Align::Start);
            btn.set_tooltip_text(Some(crate::i18n::t("See where this is on the timeline").as_str()));
            let ctx = self.ctx.clone();
            let label = format!("{book_name} {chapter}");
            btn.connect_clicked(move |_| ctx.open_time(start, end, &label));
            hbox.append(&btn);
        }
        let hrow = gtk::ListBoxRow::new();
        hrow.set_activatable(false);
        hrow.set_focusable(false);
        hrow.set_child(Some(&hbox));
        self.list.append(&hrow);
        if let Some(p) = &par {
            let names = gtk::Box::new(gtk::Orientation::Horizontal, 28);
            names.set_homogeneous(true);
            names.set_margin_end(24);
            // a version without this book (the Septuagint has no Acts) says so
            let has_book = |mm: &Arc<Module>| {
                (1..=3).any(|v| {
                    mm.verse_spans(&book, chapter, v)
                        .is_some_and(|s| s.iter().any(|x| !x.text.trim().is_empty()))
                })
            };
            for mm in [&m, p] {
                let text = if Arc::ptr_eq(mm, p) && !has_book(mm) {
                    format!("{} doesn’t include {book_name}", mm.abbreviation())
                } else {
                    mm.description()
                };
                let l = gtk::Label::new(Some(&text));
                l.add_css_class("dim-label");
                l.add_css_class("caption-heading");
                l.set_xalign(0.0);
                l.set_hexpand(true);
                l.set_ellipsize(gtk::pango::EllipsizeMode::End);
                names.append(&l);
            }
            let r = gtk::ListBoxRow::new();
            r.set_activatable(false);
            r.set_child(Some(&names));
            self.list.append(&r);
        }

        let chapter_text = m.chapter(&book, chapter);
        let first_verse = chapter_text.first().map(|(v, _)| *v).unwrap_or(1);
        let mut verses = Vec::new();
        for (v, spans) in chapter_text {
            let hl = notes
                .highlights
                .get(&v)
                .and_then(|id| cfg.colour(id))
                .map(|c| c.hex.clone());
            // part-verse highlights follow their words, so they only show where those words occur
            let base_plain = spans_to_markup(
                &spans,
                &Style {
                    red_letters: false,
                    highlight: None,
                    dark,
                    ranges: &[],
                    module: &m.name,
                },
            )
            .2;
            let ranges: Vec<(usize, usize, String)> = notes
                .ranges_for(v)
                .filter_map(|r| {
                    let c = cfg.colour(&r.color)?;
                    let (f, t) = locate(&base_plain, r)?;
                    Some((f, t, c.hex.clone()))
                })
                .collect();
            let (body, titles, plain) = spans_to_markup(
                &spans,
                &Style {
                    red_letters: cfg.red_letters,
                    highlight: hl.as_deref(),
                    dark,
                    ranges: &ranges,
                    module: &m.name,
                },
            );
            for t in titles {
                let tl = gtk::Label::new(Some(&t));
                tl.add_css_class("verse-title");
                tl.set_xalign(0.0);
                tl.set_wrap(true);
                let tr = gtk::ListBoxRow::new();
                tr.set_activatable(false);
                tr.set_child(Some(&tl));
                self.list.append(&tr);
            }
            // classic typography opens each chapter with a large initial instead of "1"
            let split = if cfg.vintage && v == first_verse {
                crate::render::split_initial(&body)
            } else {
                None
            };
            let initial = split.is_some();
            let num = if cfg.verse_numbers && !initial {
                verse_number(v)
            } else {
                String::new()
            };
            let (initial_letter, body) = match split {
                Some((c, rest)) => (Some(c), rest),
                None => (None, body),
            };
            let row_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            let cols = gtk::Box::new(gtk::Orientation::Horizontal, 28);
            cols.set_homogeneous(true);
            cols.set_hexpand(true);
            row_box.append(&cols);
            let prefix: isize = if initial {
                -1
            } else if cfg.verse_numbers {
                v.to_string().chars().count() as isize + 2
            } else {
                0
            };
            let l = self.verse_label(&format!("{num}{body}"), v);
            self.make_interactive(&l, v, prefix, plain);
            if let Some(c) = initial_letter {
                let b = gtk::Box::new(gtk::Orientation::Horizontal, 10);
                let size = (cfg.text_size * 6.4).round() as i32;
                b.append(&crate::initial::ornate_initial(c, size, dark));
                b.append(&l);
                cols.append(&b);
            } else {
                cols.append(&l);
            }
            if let Some(p) = &par {
                let pm = p
                    .verse_spans(&book, chapter, v)
                    .map(|s| {
                        spans_to_markup(
                            &s,
                            &Style {
                                red_letters: cfg.red_letters,
                                highlight: None,
                                dark,
                                ranges: &[],
                                module: &p.name,
                            },
                        )
                        .0
                    })
                    .unwrap_or_default();
                let l2 = self.verse_label(&format!("{num}{pm}"), v);
                cols.append(&l2);
            }
            let has_note = notes
                .notes
                .get(&v)
                .map(|n| !n.trim().is_empty())
                .unwrap_or(false);
            if !has_note {
                // keep columns aligned whether or not a verse has a note
                let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
                spacer.set_size_request(16, -1);
                row_box.append(&spacer);
            }
            if has_note {
                let img = gtk::Image::from_icon_name("document-edit-symbolic");
                img.add_css_class("note-mark");
                img.set_valign(gtk::Align::Start);
                img.set_tooltip_text(Some(&notes.notes[&v]));
                row_box.append(&img);
            }
            let row = gtk::ListBoxRow::new();
            row.set_child(Some(&row_box));
            row.add_css_class("verse-row");
            row.set_widget_name(&v.to_string());
            row.set_activatable(false);
            self.list.append(&row);
            verses.push(v);
        }
        if verses.is_empty() {
            let l = gtk::Label::new(Some(crate::i18n::t("This version doesn’t include this chapter.").as_str()));
            l.add_css_class("dim-label");
            let r = gtk::ListBoxRow::new();
            r.set_activatable(false);
            r.set_child(Some(&l));
            self.list.append(&r);
        }
        {
            let mut st = self.state.borrow_mut();
            st.notes = notes;
            st.verses = verses;
            if st.sel > 0 && !st.verses.contains(&st.sel) {
                st.sel = 0;
            }
        }
        self.mark_selected();
        self.update_panel();
    }

    fn verse_label(&self, markup: &str, v: u16) -> gtk::Label {
        let l = gtk::Label::new(None);
        l.set_markup(markup);
        l.set_wrap(true);
        l.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        l.set_xalign(0.0);
        l.set_hexpand(true);
        l.set_valign(gtk::Align::Start);
        l.add_css_class("scripture");
        let ctx = self.ctx.clone();
        let w = self.me.borrow().clone();
        l.connect_activate_link(move |label, uri| {
            if let Some(me) = w.upgrade() {
                me.link_at.set(Some(std::time::Instant::now()));
            }
            if let Some(rest) = uri.strip_prefix("note:") {
                if let (Some(me), Some((module, i))) = (w.upgrade(), rest.rsplit_once(':')) {
                    if let Ok(i) = i.parse::<usize>() {
                        me.show_note_popover(label, module, v, i);
                    }
                }
            } else if let Some(rf) = Ref::from_uri(uri) {
                ctx.open_ref(&rf);
            }
            glib::Propagation::Stop
        });
        l
    }

    /// A footnote or cross-reference marker was clicked.
    fn show_note_popover(self: &Rc<Self>, label: &gtk::Label, module: &str, v: u16, i: usize) {
        let Some(m) = self.ctx.lib().get(module) else {
            return;
        };
        let (book, chapter) = {
            let st = self.state.borrow();
            (st.book.clone(), st.chapter)
        };
        let Some(note) = m
            .verse_spans(&book, chapter, v)
            .and_then(|sp| sp.into_iter().filter_map(|s| s.note).nth(i))
        else {
            return;
        };
        let pop = gtk::Popover::new();
        pop.set_parent(label);
        pop.set_position(gtk::PositionType::Bottom);
        let body = gtk::Box::new(gtk::Orientation::Vertical, 8);
        body.set_size_request(320, -1);
        let title = gtk::Label::new(Some(&format!(
            "{} · {}",
            if note.xref { crate::i18n::ts("Cross-references") } else { crate::i18n::ts("Footnote") },
            Ref::new(&book, chapter, v)
        )));
        title.add_css_class("caption-heading");
        title.add_css_class("dim-label");
        title.set_xalign(0.0);
        body.append(&title);
        if !note.xref && !note.text.is_empty() {
            let mut txt = String::new();
            let mut last = 0;
            for f in bible_core::refs::find_refs(&note.text) {
                txt.push_str(&glib::markup_escape_text(&note.text[last..f.start]));
                txt.push_str(&format!(
                    "<a href=\"{}\">{}</a>",
                    f.r.uri(),
                    glib::markup_escape_text(&note.text[f.start..f.end])
                ));
                last = f.end;
            }
            txt.push_str(&glib::markup_escape_text(&note.text[last..]));
            let l = gtk::Label::new(None);
            l.set_markup(&txt);
            l.set_wrap(true);
            l.set_xalign(0.0);
            l.set_max_width_chars(44);
            l.set_selectable(true);
            l.set_can_focus(false);
            let ctx = self.ctx.clone();
            let p = pop.clone();
            l.connect_activate_link(move |_, uri| {
                if let Some(r) = Ref::from_uri(uri) {
                    close_popover(&p);
                    ctx.open_ref(&r);
                }
                glib::Propagation::Stop
            });
            body.append(&l);
        }
        let primary = self.primary();
        for r in note.refs.iter().take(12) {
            let preview = primary
                .as_ref()
                .and_then(|pm| pm.verse_plain(&r.book, r.chapter, r.verse.max(1)))
                .unwrap_or_default();
            let short: String = preview.chars().take(90).collect();
            let row = gtk::Button::new();
            row.add_css_class("flat");
            let col = gtk::Box::new(gtk::Orientation::Vertical, 2);
            let t = gtk::Label::new(Some(&r.to_string()));
            t.add_css_class("heading");
            t.set_xalign(0.0);
            col.append(&t);
            if !short.is_empty() {
                let pl = gtk::Label::new(Some(&format!(
                    "{short}{}",
                    if preview.chars().count() > 90 {
                        "…"
                    } else {
                        ""
                    }
                )));
                pl.set_wrap(true);
                pl.set_xalign(0.0);
                pl.set_max_width_chars(44);
                pl.add_css_class("dim-label");
                col.append(&pl);
            }
            row.set_child(Some(&col));
            let ctx = self.ctx.clone();
            let p = pop.clone();
            let r = r.clone();
            row.connect_clicked(move |_| {
                close_popover(&p);
                let (ctx, r) = (ctx.clone(), r.clone());
                glib::idle_add_local_once(move || ctx.open_ref(&r));
            });
            body.append(&row);
        }
        pop.set_child(Some(&body));
        pop.connect_closed(|p| {
            let p = p.clone();
            glib::idle_add_local_once(move || {
                if p.parent().is_some() {
                    p.unparent();
                }
            });
        });
        pop.popup();
        // don't open with the note text already selected
        pop.set_focus_child(None::<&gtk::Widget>);
    }

    /// The verse picker's grid for the current chapter.
    fn fill_verses(self: &Rc<Self>) {
        while let Some(c) = self.verse_flow.first_child() {
            self.verse_flow.remove(&c);
        }
        let (book, chapter, sel) = {
            let st = self.state.borrow();
            (st.book.clone(), st.chapter, st.sel)
        };
        let Some(m) = self.primary() else { return };
        let n = m
            .versification()
            .book(&book)
            .map(|b| b.verse_count(chapter))
            .unwrap_or(0);
        // the book's introduction comes first
        let intro = gtk::Button::with_label(crate::i18n::t("Intro").as_str());
        intro.add_css_class("flat");
        intro.set_tooltip_text(Some(crate::i18n::t("Introduction to the book").as_str()));
        if chapter == 0 {
            intro.add_css_class("suggested-action");
            self.verse_btn.set_label(crate::i18n::t("Intro").as_str());
        }
        {
            let ctx = self.ctx.clone();
            let book = book.clone();
            intro.connect_clicked(move |b| {
                if let Some(pop) = b
                    .ancestor(gtk::Popover::static_type())
                    .and_downcast::<gtk::Popover>()
                {
                    pop.popdown();
                }
                ctx.open_ref(&Ref::new(&book, 0, 0));
            });
        }
        self.verse_flow.insert(&intro, -1);
        // at an introduction, the verses listed are chapter 1's
        let n = if chapter == 0 {
            m.versification().book(&book).map(|b| b.verse_count(1)).unwrap_or(0)
        } else {
            n
        };
        for v in 1..=n {
            let b = gtk::Button::with_label(&v.to_string());
            b.add_css_class("flat");
            if v == sel && chapter > 0 {
                b.add_css_class("suggested-action");
            }
            let w = Rc::downgrade(self);
            b.connect_clicked(move |b| {
                if let Some(pop) = b
                    .ancestor(gtk::Popover::static_type())
                    .and_downcast::<gtk::Popover>()
                {
                    pop.popdown();
                }
                if let Some(r) = w.upgrade() {
                    if chapter == 0 {
                        let book = r.state.borrow().book.clone();
                        r.ctx.open_ref(&Ref::new(&book, 1, v));
                    } else {
                        r.go_to_verse(v);
                    }
                }
            });
            self.verse_flow.insert(&b, -1);
        }
    }

    /// Select and scroll to a verse in the open chapter.
    fn go_to_verse(&self, v: u16) {
        self.state.borrow_mut().sel = v;
        self.mark_selected();
        self.scroll_to_verse(v);
        self.verse_btn.set_label(&format!("v. {v}"));
        self.update_panel();
    }

    fn mark_selected(&self) {
        let sel = self.state.borrow().sel;
        let mut child = self.list.first_child();
        while let Some(c) = child {
            if c.widget_name() == sel.to_string() && sel > 0 {
                c.add_css_class("verse-selected");
            } else {
                c.remove_css_class("verse-selected");
            }
            child = c.next_sibling();
        }
    }

    fn scroll_to_verse(&self, v: u16) {
        let list = self.list.clone();
        let scroller = self.scroller.clone();
        glib::idle_add_local_once(move || {
            let mut child = list.first_child();
            while let Some(c) = child {
                if c.widget_name() == v.to_string() {
                    if let Some(p) =
                        c.compute_point(&scroller, &gtk::graphene::Point::new(0.0, 0.0))
                    {
                        let adj = scroller.vadjustment();
                        adj.set_value((adj.value() + p.y() as f64 - 80.0).max(0.0));
                    }
                    break;
                }
                child = c.next_sibling();
            }
        });
    }

    /// Selecting words highlights part of the verse; a plain click opens the verse menu.
    fn make_interactive(&self, l: &gtk::Label, v: u16, prefix: isize, plain: String) {
        l.set_selectable(true);
        l.set_focus_on_click(false);
        // a legacy controller sees the release even though the label claims the click
        let g = gtk::EventControllerLegacy::new();
        g.set_propagation_phase(gtk::PropagationPhase::Capture);
        let clicks = Rc::new(Cell::new(0u32));
        let w = self.me.borrow().clone();
        let lw = l.downgrade();
        g.connect_event(move |_, ev| {
            if ev.event_type() != gdk::EventType::ButtonRelease {
                return glib::Propagation::Proceed;
            }
            if ev.downcast_ref::<gdk::ButtonEvent>().map(|b| b.button()) != Some(1) {
                return glib::Propagation::Proceed;
            }
            let (Some(me), Some(label)) = (w.upgrade(), lw.upgrade()) else {
                return glib::Propagation::Proceed;
            };
            // clicks on links (references, footnote markers) are handled by the link itself
            if label.current_uri().is_some() {
                clicks.set(clicks.get() + 1);
                return glib::Propagation::Proceed;
            }
            let plain = plain.clone();
            // wait past the double-click time so a double-click (select a word) isn't
            // mistaken for a single click (open the verse menu)
            let n = clicks.get() + 1;
            clicks.set(n);
            let clicks2 = clicks.clone();
            glib::timeout_add_local_once(std::time::Duration::from_millis(320), move || {
                if clicks2.get() != n {
                    return;
                }
                if label.current_uri().is_some() {
                    return;
                }
                if me
                    .link_at
                    .get()
                    .is_some_and(|t| t.elapsed() < std::time::Duration::from_millis(700))
                {
                    return;
                }
                match label.selection_bounds() {
                    Some((a, b)) if a != b => {
                        let n = plain.chars().count();
                        // prefix: label characters before the verse text (negative when the
                        // first letter is drawn as a separate initial)
                        let from = (a.min(b) as isize - prefix).clamp(0, n as isize) as usize;
                        let to = (a.max(b) as isize - prefix).clamp(0, n as isize) as usize;
                        // trim spaces at the edges of the selection
                        let chars: Vec<char> = plain.chars().collect();
                        let mut f = from;
                        let mut t = to;
                        while f < t && chars[f].is_whitespace() {
                            f += 1;
                        }
                        while t > f && chars[t - 1].is_whitespace() {
                            t -= 1;
                        }
                        if t > f {
                            let text: String = chars[f..t].iter().collect();
                            // point the menu at the start of the selection
                            let full = label.text().to_string();
                            let start_char = a.min(b) as usize;
                            let byte = full
                                .char_indices()
                                .nth(start_char)
                                .map(|(i, _)| i)
                                .unwrap_or(0) as i32;
                            let (ox, oy) = label.layout_offsets();
                            let pos = label.layout().index_to_pos(byte);
                            let sc = gtk::pango::SCALE;
                            let rect = gdk::Rectangle::new(
                                ox + pos.x() / sc,
                                oy + pos.y() / sc,
                                (pos.width() / sc).max(1),
                                pos.height() / sc,
                            );
                            me.show_range_popover(&label, v, f, t, text, Some(rect));
                        }
                    }
                    _ => me.verse_clicked_v(v),
                }
            });
            glib::Propagation::Proceed
        });
        l.add_controller(g);
    }

    fn row_for(&self, v: u16) -> Option<gtk::ListBoxRow> {
        let mut child = self.list.first_child();
        while let Some(c) = child {
            if c.widget_name() == v.to_string() {
                return c.downcast::<gtk::ListBoxRow>().ok();
            }
            child = c.next_sibling();
        }
        None
    }

    fn verse_clicked_v(self: &Rc<Self>, v: u16) {
        if let Some(row) = self.row_for(v) {
            self.verse_clicked(&row);
        }
    }

    fn show_range_popover(
        self: &Rc<Self>,
        label: &gtk::Label,
        v: u16,
        from: usize,
        to: usize,
        text: String,
        at: Option<gdk::Rectangle>,
    ) {
        self.state.borrow_mut().sel = v;
        self.mark_selected();
        self.update_panel();
        let pop = gtk::Popover::new();
        pop.set_parent(label);
        pop.set_position(gtk::PositionType::Bottom);
        if let Some(r) = at {
            pop.set_pointing_to(Some(&r));
        }
        let body = gtk::Box::new(gtk::Orientation::Vertical, 10);
        let short: String = text.chars().take(48).collect();
        let title = gtk::Label::new(Some(&format!(
            "“{short}{}”",
            if text.chars().count() > 48 { "…" } else { "" }
        )));
        title.add_css_class("caption-heading");
        title.add_css_class("dim-label");
        title.set_xalign(0.0);
        title.set_wrap(true);
        title.set_max_width_chars(36);
        let colours = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        let palette = self.ctx.config.borrow().palette.clone();
        for c in palette {
            let b = gtk::Button::new();
            b.add_css_class("swatch");
            b.add_css_class(&format!("swatch-{}", c.id));
            b.set_size_request(28, 28);
            b.set_valign(gtk::Align::Center);
            b.set_tooltip_text(Some(&c.name));
            let me = self.clone();
            let p = pop.clone();
            let id = c.id.clone();
            let text = text.clone();
            b.connect_clicked(move |_| {
                close_popover(&p);
                let (me, id, text) = (me.clone(), id.clone(), text.clone());
                glib::idle_add_local_once(move || {
                    me.edit_ranges(|n| {
                        n.add_range(bible_core::notes::RangeHighlight {
                            verse: v,
                            from,
                            to,
                            color: id,
                            text,
                        })
                    })
                });
            });
            colours.append(&b);
        }
        let overlaps = self
            .state
            .borrow()
            .notes
            .ranges_for(v)
            .any(|r| r.from < to && from < r.to);
        if overlaps {
            let clear = gtk::Button::from_icon_name("edit-clear-symbolic");
            clear.add_css_class("flat");
            clear.add_css_class("circular");
            clear.set_tooltip_text(Some(crate::i18n::t("Remove highlight").as_str()));
            let me = self.clone();
            let p = pop.clone();
            clear.connect_clicked(move |_| {
                close_popover(&p);
                let me = me.clone();
                glib::idle_add_local_once(move || me.edit_ranges(|n| n.remove_range(v, from, to)));
            });
            colours.append(&clear);
        }
        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        let copy_b = gtk::Button::with_label(crate::i18n::t("Copy").as_str());
        let note_b = gtk::Button::with_label(crate::i18n::t("Add note").as_str());
        let (bk, ch) = {
            let st = self.state.borrow();
            (st.book.clone(), st.chapter)
        };
        actions.append(&self.bookmark_button(&pop, &bk, ch, v));
        actions.append(&note_b);
        actions.append(&copy_b);
        let p = pop.clone();
        let t2 = text.clone();
        let me = self.clone();
        copy_b.connect_clicked(move |b| {
            p.popdown();
            b.clipboard().set_text(&t2);
            me.ctx.toast(crate::i18n::t("Copied").as_str());
        });
        let p = pop.clone();
        let me = self.clone();
        note_b.connect_clicked(move |_| {
            p.popdown();
            me.split.set_show_sidebar(true);
            me.panel_stack.set_visible_child_name("notes");
            let buf = me.note_view.buffer();
            let mut end = buf.end_iter();
            let quote = format!(
                "{}“{}” — ",
                if buf.char_count() > 0 { "\n" } else { "" },
                text
            );
            buf.insert(&mut end, &quote);
            me.note_view.grab_focus();
        });
        body.append(&title);
        body.append(&colours);
        body.append(&actions);
        pop.set_child(Some(&body));
        pop.connect_closed(|p| {
            let p = p.clone();
            glib::idle_add_local_once(move || {
                if p.parent().is_some() {
                    p.unparent();
                }
            });
        });
        pop.popup();
    }

    /// Change part-verse highlights, save the chapter file and redraw in place.
    fn edit_ranges(&self, f: impl FnOnce(&mut ChapterNotes)) {
        let (book, chapter, notes) = {
            let mut st = self.state.borrow_mut();
            f(&mut st.notes);
            (st.book.clone(), st.chapter, st.notes.clone())
        };
        if let Err(e) = self.ctx.notes().save(&book, chapter, &notes) {
            self.ctx.toast(&format!("Couldn’t save highlight: {e}"));
        }
        let adj = self.scroller.vadjustment().value();
        self.render();
        let s = self.scroller.clone();
        glib::idle_add_local_once(move || s.vadjustment().set_value(adj));
    }

    /// A star that bookmarks the verse (and closes the menu).
    fn bookmark_button(&self, pop: &gtk::Popover, book: &str, chapter: u16, v: u16) -> gtk::Button {
        let osis = format!("{book}.{chapter}.{v}");
        let marked = crate::bookmarks::is_bookmarked(&self.ctx, &osis);
        let b = gtk::Button::from_icon_name(if marked {
            "starred-symbolic"
        } else {
            "non-starred-symbolic"
        });
        b.set_tooltip_text(Some(if marked { crate::i18n::ts("Remove bookmark") } else { crate::i18n::ts("Bookmark") }));
        if marked {
            b.add_css_class("accent");
        }
        let ctx = self.ctx.clone();
        let p = pop.clone();
        b.connect_clicked(move |_| {
            let now = crate::bookmarks::toggle(&ctx, &osis);
            p.popdown();
            let c2 = ctx.clone();
            if now {
                ctx.toast_action("Bookmarked", "Bookmarks", move || c2.go("bookmarks"));
            } else {
                ctx.toast(crate::i18n::t("Bookmark removed").as_str());
            }
            ctx.refresh_except("bible");
        });
        b
    }

    fn verse_clicked(self: &Rc<Self>, row: &gtk::ListBoxRow) {
        let Ok(v) = row.widget_name().parse::<u16>() else {
            return;
        };
        self.state.borrow_mut().sel = v;
        self.mark_selected();
        self.update_panel();
        self.show_verse_popover(row, v);
    }

    fn show_verse_popover(self: &Rc<Self>, row: &gtk::ListBoxRow, v: u16) {
        let pop = gtk::Popover::new();
        pop.set_parent(row);
        pop.set_position(gtk::PositionType::Bottom);
        pop.set_has_arrow(true);
        let rect = gdk::Rectangle::new(40, row.height() - 4, 1, 1);
        pop.set_pointing_to(Some(&rect));
        let body = gtk::Box::new(gtk::Orientation::Vertical, 10);
        let (book, chapter, current) = {
            let st = self.state.borrow();
            (
                st.book.clone(),
                st.chapter,
                st.notes.highlights.get(&v).cloned(),
            )
        };
        let r = Ref::new(&book, chapter, v);
        let title = gtk::Label::new(Some(&format!(
            "{}{}",
            r,
            current
                .as_ref()
                .and_then(|c| self
                    .ctx
                    .config
                    .borrow()
                    .colour(c)
                    .map(|x| format!(" · {}", x.name)))
                .unwrap_or_default()
        )));
        title.add_css_class("caption-heading");
        title.add_css_class("dim-label");
        title.set_xalign(0.0);
        let colours = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        let palette = self.ctx.config.borrow().palette.clone();
        for c in palette {
            let b = gtk::Button::new();
            b.add_css_class("swatch");
            b.set_size_request(28, 28);
            b.set_valign(gtk::Align::Center);
            b.add_css_class(&format!("swatch-{}", c.id));
            if current.as_deref() == Some(c.id.as_str()) {
                b.add_css_class("swatch-current");
            }
            b.set_tooltip_text(Some(&c.name));
            let me = self.clone();
            let p = pop.clone();
            let id = c.id.clone();
            b.connect_clicked(move |_| {
                close_popover(&p);
                let (me, id) = (me.clone(), id.clone());
                glib::idle_add_local_once(move || me.set_highlight(v, &id));
            });
            colours.append(&b);
        }
        let clear = gtk::Button::from_icon_name("edit-clear-symbolic");
        clear.add_css_class("flat");
        clear.add_css_class("circular");
        clear.set_tooltip_text(Some(crate::i18n::t("Remove highlight").as_str()));
        let me = self.clone();
        let p = pop.clone();
        clear.connect_clicked(move |_| {
            close_popover(&p);
            let me = me.clone();
            glib::idle_add_local_once(move || me.set_highlight(v, ""));
        });
        colours.append(&clear);
        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        let note_b = gtk::Button::with_label(crate::i18n::t("Add note").as_str());
        let com_b = gtk::Button::with_label(crate::i18n::t("Commentary").as_str());
        let copy_b = gtk::Button::with_label(crate::i18n::t("Copy").as_str());
        actions.append(&self.bookmark_button(&pop, &book, chapter, v));
        for b in [&note_b, &com_b, &copy_b] {
            actions.append(b);
        }
        let me = self.clone();
        let p = pop.clone();
        note_b.connect_clicked(move |_| {
            p.popdown();
            me.split.set_show_sidebar(true);
            me.panel_stack.set_visible_child_name("notes");
            me.note_view.grab_focus();
        });
        let me = self.clone();
        let p = pop.clone();
        com_b.connect_clicked(move |_| {
            p.popdown();
            me.split.set_show_sidebar(true);
            me.panel_stack.set_visible_child_name("commentary");
        });
        let me = self.clone();
        let p = pop.clone();
        copy_b.connect_clicked(move |b| {
            p.popdown();
            if let Some(m) = me.primary() {
                let text = m
                    .verse_plain(&r.book, r.chapter, r.verse)
                    .unwrap_or_default();
                b.clipboard()
                    .set_text(&format!("“{text}” — {r} ({})", m.abbreviation()));
                me.ctx.toast(crate::i18n::t("Verse copied").as_str());
            }
        });
        body.append(&title);
        body.append(&colours);
        body.append(&actions);
        pop.set_child(Some(&body));
        pop.connect_closed(|p| {
            let p = p.clone();
            glib::idle_add_local_once(move || {
                if p.parent().is_some() {
                    p.unparent();
                }
            });
        });
        pop.popup();
    }

    fn set_highlight(&self, v: u16, id: &str) {
        let (book, chapter) = {
            let mut st = self.state.borrow_mut();
            if id.is_empty() {
                st.notes.highlights.remove(&v);
                st.notes.ranges.retain(|r| r.verse != v);
            } else {
                st.notes.highlights.insert(v, id.to_string());
            }
            (st.book.clone(), st.chapter)
        };
        let notes = self.state.borrow().notes.clone();
        if let Err(e) = self.ctx.notes().save(&book, chapter, &notes) {
            self.ctx.toast(&format!("Couldn’t save highlight: {e}"));
        }
        let adj = self.scroller.vadjustment().value();
        self.render();
        let s = self.scroller.clone();
        glib::idle_add_local_once(move || s.vadjustment().set_value(adj));
    }

    fn update_panel(&self) {
        self.update_commentary();
        {
            let st = self.state.borrow();
            if st.chapter == 0 {
                self.map.set_book(&st.book);
            } else {
                self.map.set_chapter(&st.book, st.chapter, st.sel);
            }
        }
        if let Some(me) = self.me.borrow().upgrade() {
            me.update_interlinear();
        }
        let (book, chapter, sel, note, hl) = {
            let st = self.state.borrow();
            (
                st.book.clone(),
                st.chapter,
                st.sel,
                st.notes.notes.get(&st.sel).cloned().unwrap_or_default(),
                st.notes.highlights.get(&st.sel).cloned(),
            )
        };
        self.loading_note.set(true);
        if sel > 0 {
            self.note_ref
                .set_text(&Ref::new(&book, chapter, sel).to_string());
            self.note_view.set_sensitive(true);
            self.note_view.buffer().set_text(&note);
            let path = self.ctx.notes().chapter_path(&book, chapter);
            let shown = path
                .strip_prefix(dirs::home_dir().unwrap_or_default())
                .map(|p| format!("~/{}", p.display()))
                .unwrap_or_else(|_| path.display().to_string());
            self.note_status
                .set_text(&format!("Saved as Markdown in {shown}"));
        } else {
            self.note_ref.set_text("Select a verse to write a note");
            self.note_view.buffer().set_text("");
            self.note_view.set_sensitive(false);
            self.note_status.set_text("Notes are plain Markdown files, one per chapter. Point Obsidian at the folder and they appear there too.");
        }
        while let Some(c) = self.hl_box.first_child() {
            self.hl_box.remove(&c);
        }
        if let Some(id) = hl {
            if let Some(c) = self.ctx.config.borrow().colour(&id) {
                let dot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
                dot.add_css_class("swatch-dot");
                dot.add_css_class(&format!("swatch-{}", c.id));
                let l = gtk::Label::new(Some(&format!("Highlighted · {}", c.name)));
                l.add_css_class("caption");
                self.hl_box.append(&dot);
                self.hl_box.append(&l);
            }
        }
        self.loading_note.set(false);
    }

    fn update_commentary(&self) {
        let Some(c) = self.commentary() else { return };
        let (book, chapter, sel) = {
            let st = self.state.borrow();
            (st.book.clone(), st.chapter, st.sel)
        };
        if chapter == 0 {
            self.com_ref.set_text(&c.abbreviation());
            self.com_text.set_markup("<i>The book’s introduction is shown in the main view. Open a chapter to see the commentary for each verse.</i>");
            return;
        }
        let v = sel.max(1);
        match c.commentary_for(&book, chapter, v) {
            Some((at, spans)) => {
                self.com_ref.set_text(&format!(
                    "{} · {}",
                    c.abbreviation(),
                    Ref::new(&book, chapter, at)
                ));
                self.com_text.set_markup(&long_markup(&spans, self.dark()));
            }
            None => {
                self.com_ref.set_text(&format!(
                    "{} · {}",
                    c.abbreviation(),
                    Ref::new(&book, chapter, v)
                ));
                self.com_text
                    .set_markup("<i>Nothing on this verse in this commentary.</i>");
            }
        }
    }

    fn schedule_note_save(self: &Rc<Self>) {
        if self.loading_note.get() {
            return;
        }
        let gen = self.save_gen.get() + 1;
        self.save_gen.set(gen);
        self.note_status.set_text("Editing…");
        let w = Rc::downgrade(self);
        glib::timeout_add_local_once(std::time::Duration::from_millis(700), move || {
            let Some(r) = w.upgrade() else { return };
            if r.save_gen.get() != gen {
                return;
            }
            let buf = r.note_view.buffer();
            let text = buf
                .text(&buf.start_iter(), &buf.end_iter(), false)
                .to_string();
            let (book, chapter, notes) = {
                let mut st = r.state.borrow_mut();
                let sel = st.sel;
                if sel == 0 {
                    return;
                }
                if text.trim().is_empty() {
                    st.notes.notes.remove(&sel);
                } else {
                    st.notes.notes.insert(sel, text);
                }
                (st.book.clone(), st.chapter, st.notes.clone())
            };
            match r.ctx.notes().save(&book, chapter, &notes) {
                Ok(_) => r.note_status.set_text("Saved"),
                Err(e) => r.note_status.set_text(&format!("Couldn’t save: {e}")),
            }
            // refresh the note marker without losing the scroll position
            let adj = r.scroller.vadjustment().value();
            let focus = r.note_view.has_focus();
            r.loading_note.set(true);
            r.render();
            r.loading_note.set(false);
            if focus {
                r.note_view.grab_focus();
            }
            let s = r.scroller.clone();
            glib::idle_add_local_once(move || s.vadjustment().set_value(adj));
        });
    }
}

impl Section for Reader {
    fn id(&self) -> &'static str {
        "bible"
    }
    fn title(&self) -> &'static str { crate::i18n::ts("Bible") }
    fn icon(&self) -> &'static str {
        "bibleapp-book-symbolic"
    }
    fn widget(&self) -> gtk::Widget {
        self.root.clone().upcast()
    }
    fn refresh(&self) {
        self.fill_versions();
        self.fill_books();
        self.render();
    }
    /// The reading Bible may have been changed elsewhere (the cross-reference
    /// views' version menu): catch up when shown.
    fn shown(&self) {
        let want = self.ctx.config.borrow().bible.clone();
        let showing = self.versions.borrow().get(self.version_dd.selected() as usize).cloned();
        if showing.as_deref() != Some(want.as_str()) {
            self.refresh();
        }
    }
}

/// Roman numerals for printed chapter headings.
fn roman(mut n: u16) -> String {
    const T: [(u16, &str); 13] = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut s = String::new();
    for (v, r) in T {
        while n >= v {
            s.push_str(r);
            n -= v;
        }
    }
    s
}

/// "King James Version" or "Septuagint in American English 2012 · Old
/// Testament only": what a version is, under its short name in the menu.
fn version_note(m: &Module) -> String {
    let ot = m.raw_entry("Gen", 1, 1).is_some();
    let nt = m.raw_entry("Matt", 1, 1).is_some();
    let scope = match (ot, nt) {
        (true, false) => " · Old Testament only",
        (false, true) => " · New Testament only",
        _ => "",
    };
    format!("{}{scope}", m.description())
}

#[cfg(test)]
mod roman_tests {
    #[test]
    fn numerals() {
        assert_eq!(super::roman(1), "I");
        assert_eq!(super::roman(14), "XIV");
        assert_eq!(super::roman(150), "CL");
    }
}
