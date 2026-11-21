# To do

**Vision:** an offline Bible first; then everything in Christendom, as far
as each user wants to take it. Everything optional is downloaded from a
source that will last (Wikimedia Commons, Wikipedia, CrossWire, public
archives), even if the user has to click Download.

**Done in 0.6.0:** apocrypha intros (18 books, built in, plus their full
Wikipedia articles), gentler Wikipedia downloads, and a large centred
spinner on the cross-reference map.

- **Chant school.** Follow-along only; see docs/NEXT-chant-school.md.
- **Check Wikipedia downloads.** Confirm on a real internet connection that
  the pacing and retries in 0.5.3 are enough; if not, pace further (one
  book every 3 s).
- **Small windows.** At about 1280 px wide, the Bible view's header is a few
  pixels too wide.
- **Passage wallpapers.** Generate desktop wallpapers from passages.
- **Art gallery.** One place for public-domain biblical artwork: paintings,
  engravings (Doré), icons, illuminated manuscripts. Each piece is linked to
  its passages, so it shows up in the reader and on Today. Sources:
  Wikimedia Commons, the Met and Art Institute of Chicago open access (CC0),
  Rijksmuseum. Download on demand and credit every piece.
- **Writings outside the canon (on hold until asked).** 1 Enoch (R. H.
  Charles, 1917, public domain), Jubilees, the Odes of Solomon, the
  Didache, the Shepherd of Hermas and others: public-domain translations,
  with Wikipedia intros, in their own "Other ancient writings" shelf. Mark
  them clearly as outside the canon, like the "Other traditions" group in
  the Library.
- **Chant school sources (on hold).** GABC scores from GregoBase
  (gregobase.selapa.net), public-domain Liber Usualis and Graduale scans,
  and freely licensed recordings on Wikimedia Commons; downloaded when the
  user clicks.
- **Church fathers (next up).** A download option for patristic writings:
  the public-domain Ante-Nicene, Nicene and Post-Nicene Fathers (Schaff,
  1885–1900; CrossWire has some as SWORD books, CCEL has the full set),
  shown in Books, with each father's verse comments linked from the reader.
- **Translations.** Spanish is started (po/es.po): the sidebar, page titles,
  Today and Preferences. Still to do: the rest of the interface, book names
  and feast names (from the Bible module's language where possible), dates
  in the local format, and the website. To add a language, copy po/es.po to
  po/xx.po and add it to CATALOGS in app/src/i18n.rs.

## Cross-references over time (later)

An animation of the cross-reference views through time: books appear in the
order they were written, and their links light up as each one arrives.
Suggested 2026-11-21.
