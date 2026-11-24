# Changelog

## 0.9.17

- **The rainbow uses your Bible's own cross-references whenever it has any**,
  even an incomplete set (ESV, for example, brings over 85,000). The built-in
  OpenBible.info set is used only for Bibles with none of their own, and the
  footer says which you're seeing.

## 0.9.16

- **Tags on Bibles in Downloads.** Up to three short tags say what a Bible
  can do: Cross-references (used by the rainbow), Word study (Strong's),
  Footnotes, Red letters or Headings. They come straight from each module's
  own description on the download server.

## 0.9.15

- **Bibles with their own cross-references are marked** in Downloads with an
  "Own cross-references" tag; the rainbow and the other views use those links.
- Flatpak: your downloads stay when you update. Every release now checks
  this, along with the window opening with the splash screen on. (The
  re-downloads in the last few versions came from moving Flatpak downloads
  into the app's own folder in 0.9.11, and the Quran's new pronunciation in
  0.9.13; neither repeats.)

## 0.9.14

- In the Reading room, the Quran has its own section at the bottom instead
  of appearing under the church fathers.
- The Quran's pronunciation shows long and emphatic letters underlined, as
  Tanzil marks them, instead of showing raw tags.

## 0.9.13

- **Contents as a panel.** In the Reading room, a book's contents (each surah
  of the Quran, each section of the church fathers) now sit in a panel beside
  the text, with the part you're reading highlighted. The Contents button
  shows or hides it.
- **How the Arabic sounds.** Each verse of the Quran now shows its
  pronunciation in Latin letters under the Arabic (Tanzil's transliteration),
  as the Greek and Hebrew do. Download the Quran again to get it.
- **Islamic history in the timeline**, as its own lane: from the Prophet's
  life and the Hijra through the caliphates, Córdoba, the House of Wisdom,
  the Ottomans and the Mughals.

## 0.9.12

- **Cross-references from your Bible.** When the selected Bible comes with its
  own full set of cross-references, the rainbow and the other views now use
  them. Otherwise the footer says plainly that the links are OpenBible.info's
  set, which is built into the app (so it works offline too).

## 0.9.11

- **The Quran, for study and comparison.** Downloads, Start here, has the
  Quran at the very bottom: the Arabic text (Tanzil) with Marmaduke
  Pickthall's English translation (1930), verse by verse, read in the Reading
  room. Story links with the Bible and the cross-reference views come later.
- **Ready for Flathub.** In the Flatpak, downloads now go into the app's own
  folder, so it needs no access to your home folder; the update banner is
  gone there, since GNOME Software updates Flatpaks; and the app's store
  listing has screenshots and every version's notes.
- The language list in Downloads no longer jumps around as you scroll.

## 0.9.10

- **Word details live inside Original.** Tap a Greek or Hebrew word and its
  meaning, grammar and lexicon entry open right under the interlinear, instead
  of in a separate Words tab. The lexicon comes with the Greek and Hebrew
  download, so nothing else is needed.
- **Church history on Today** has a button on each event to read about it on
  Wikipedia.
- Every release now checks that the files Bible offers to download (Bible
  sources, Greek and Hebrew data, church fathers, audio, Wikipedia) are still
  reachable.

## 0.9.9

- **Personal copies.** Many modern translations are still under copyright, so
  they can't be offered for download. If you already have one as a SWORD
  module (.zip), add it from Downloads, Start here, Personal copies. It stays
  on your computer.
- Plans for reading the Quran and seeing its story parallels with the Bible
  (in docs/quran.md); not in the app yet.

## 0.9.8

- **Downloads, a category at a time.** Browse now has a list on the left:
  Start here, Bibles, Commentaries, Books, Devotionals, and Dictionaries and
  lexicons, each with a count. Search still looks through everything.
- **Languages by name.** The language menu shows every language's name
  instead of codes such as "ckb" or "tpi", in alphabetical order, with how
  many works each has, and you can type to find one.
- A little more space under the search bar.

## 0.9.7

- **Mac app (untested):** the macOS download is now a real Bible.app with
  everything inside it, GTK included, so nothing else needs installing
  (Apple Silicon). Unzip, drag it to Applications, and allow it once in
  System Settings, Privacy & Security, since it isn't notarized by Apple.
- **Windows build (untested):** text uses Windows' own fonts (Georgia or
  Cambria for scripture, Cascadia Mono or Consolas for code) instead of falling
  back to a plain sans font, and the app's own icons are included. No more
  console window beside the app, and no "win32 session" warning. The zip's
  README explains the SmartScreen warning: the app isn't code-signed.
- Bible is made for GNOME first: Windows and macOS builds are offered as they
  are, without support. Bug reports now use a short form, and reports from
  Windows or macOS are closed automatically with a note.
- The original-language button reads **Αω** (Alpha and Omega).

## 0.9.6

- The README says plainly that Windows and macOS builds are untested; Bible is
  made for GNOME first. (This release has no Windows build: its font check
  failed, and it was fixed in 0.9.7.)

## 0.9.5

- The version menu now shows each version's full name under its short one,
  and says when a version has only the Old or only the New Testament. LXX2012,
  for example, is the Septuagint (the Greek Old Testament) in English, so it
  shows "Old Testament only".

## 0.9.4

- **Windows and macOS test builds.** Each release now includes a Windows zip
  and a macOS zip, named `_untested`: they build and pass every test
  automatically, but no one has run them on a real PC or Mac yet. The macOS
  build needs GTK from Homebrew (see the README inside the zip).
- Releases come out faster: the tests and the Linux package share one build,
  and Flatpak, Windows and macOS build at the same time.

## 0.9.3

- **Flatpak.** Each release now also comes as a `.flatpak`, which brings its
  own GNOME libraries and so runs on older systems such as Pop!_OS and
  Ubuntu 22.04 and Debian 12, and on most other distributions.
- Inside the Flatpak, the update banner opens the release page instead of
  installing a .deb.
- Groundwork for Windows and macOS: modules and data go in the usual place
  on each system (%APPDATA%\Sword on Windows), and every push is now built
  and tested on Windows, macOS and as a Flatpak.

## 0.9.2

- The loading animations follow the accent colour.
- Dictionaries and glossaries stored like Bibles no longer appear in the
  version lists.
- Side by side: a version without the current book says so (the Septuagint
  has no Acts) instead of showing empty verses.

## 0.9.1

- Church fathers: paragraphs no longer break at footnotes; volumes you
  already downloaded are read again with the fix.
- The Contents list is easier to scan (one line per section, full title on
  hover) and opens at the section you're reading.
- Downloading or removing a volume no longer closes the list, so you can
  start several downloads at once.
- Splash style and loading animation are picked from live previews in
  Preferences; click a splash to see it full size.
- An Arch Linux PKGBUILD, and clear system requirements in the README.

## 0.9.0

- **Möbius strip (3D).** A cross-reference joins two places in the Bible.
  With the Bible closed into a loop (Revelation hands back to Genesis), every
  such pair is exactly one point on a Möbius strip, so nothing is shown twice.
  Neighbouring passages line the strip's single edge, passages half the Bible
  apart run along its centre line, and Genesis meets Revelation at the seam.
- The ordering menu (Bible order, chronological, by writer…) now rearranges
  the cube, globe and Möbius strip too.

## 0.8.2

- The Words panel has a "Show Greek and Hebrew" button, so the interlinear
  can be turned on from where you look for it.

## 0.8.1

- The globe is round: each Testament spirals over its own half, Genesis by
  the North Pole and Revelation by the South Pole.
- The cube fills its space: depth is now how far back in time a reference
  reaches (late books citing early ones at the back).
- Splash style has a preview button, and changing it plays the new style
  straight away.

## 0.8.0

- **Church fathers.** The Ante-Nicene and Nicene and Post-Nicene Fathers,
  37 volumes, download one at a time from the Christian Classics Ethereal
  Library and read offline in the Reading room, opening where you left off.
- **Cross-references in 3D.** A cube and a globe you turn by hand, alongside
  the rainbow, circle and matrix.
- **Accent colour.** Follows the church year by default (violet, gold, green,
  red), or your desktop's accent, or a colour you pick.
- **Preferences in four pages:** General, Reading, Today, Downloads.
- Clearer names: Reading room (was Books and devotionals) and Downloads (was
  Library). The sidebar is grouped.
- Today leads with your reading; the legal notice shows once; the
  cross-reference view explains itself on your first visit.
- A new install opens on a Bible in your language.
- About, the README and the website say why the app exists.
- Every icon-only button has a name for screen readers.

## 0.7.0

- The reader's header is labelled: Original (Greek and Hebrew) and Study.
  Text size moved into the ⋮ menu.
