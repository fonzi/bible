# Quran: exploration

Status: explored, not built. Goal: read the Quran offline and see where its
stories meet the Bible's, including in the rainbow view.

## Where it goes

- **Downloads:** last on Start here, below Other traditions (Jehovah's
  Witnesses, Latter-day Saints). Personal copies sit above Other traditions.
- **Rainbow and the other cross-reference views:** a band at the very bottom,
  after the Bible's books and below the Other traditions band, with its 114
  surahs in their usual order. Off until the Quran is downloaded, and it can be
  hidden.

## Text (all downloadable once, then offline)

- **Arabic:** Tanzil's Uthmani text. Its licence allows copying with the text
  unchanged, attribution and a link back to tanzil.net. Check the current
  terms before shipping.
- **English, public domain:** Rodwell (1861) and Palmer (1880). Pickthall
  (1930) entered the US public domain in 2026; check other countries.
- Stored like the church fathers: parsed once into JSON in the app's data
  folder, read in the Reading room, surah by surah, Arabic beside English.

## Word by word, like Original

The Quran gets the same treatment as the Greek and Hebrew: an Original pane
with each Arabic word, its transliteration, a gloss and its root, and the word
details underneath when a word is tapped. The Quranic Arabic Corpus
(corpus.quran.com) has word-by-word morphology and glosses; check its licence
(it has been GPL) before using it.

## Links: story parallels, not verse cross-references

The Quran retells many Biblical stories in its own way, so verse-to-verse
links would claim more than is true. Links are between **stories**:

| Story | Bible | Quran |
|---|---|---|
| Creation and Adam | Gen 1–3 | 2:30–39; 7:11–25 |
| Cain and Abel | Gen 4 | 5:27–32 |
| Noah | Gen 6–9 | 11:25–49; 71 |
| Abraham and the idols, the guests, the sacrifice | Gen 12–22 | 6:74–83; 11:69–76; 37:99–111 |
| Joseph | Gen 37–50 | 12 (whole surah) |
| Moses, Pharaoh, the Exodus | Exod 1–15 | 20:9–98; 28:3–46 |
| David and Solomon | 1–2 Sam; 1 Kgs 1–11 | 27:15–44; 38:17–40 |
| Jonah | Jonah | 37:139–148; 21:87–88 |
| Zechariah, John, Mary, the birth of Jesus | Luke 1–2 | 3:35–47; 19:1–36 |
| Jesus | Gospels | 3:45–59; 5:110–118 |

About 40 to 60 hand-checked rows would cover the main narratives. Each row
carries a one-line note where the accounts differ (for example, the
sacrificed son is not named in 37:99–111).

## In the rainbow

- Arcs drawn from the Bible passage to the surah band, in a muted colour and
  dashed, so they never look like the Bible's own cross-references.
- Hovering an arc shows the story's name and both passages; clicking opens
  them side by side.
- A legend line: "Story parallels with the Quran (shared figures and events,
  not quotations)".

## Tone

Neutral and descriptive, like Other traditions: listed for study and
comparison, with no claim about which account is right.

## Work, in order

1. Download and parse Tanzil Arabic plus one English translation; read it in
   the Reading room.
2. The story-parallel table (data file plus a test that every reference
   exists).
3. The rainbow band and dashed arcs; then the circle, matrix and 3D views.
