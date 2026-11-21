# Next feature: Chant school (learn to sing chants)

Plan agreed in chat on 2026-11-18, not started yet. Current version is 0.5.2.

## Scope (user, 2026-11-18): keep it simple
Mostly just follow along, nothing too complex. Build steps 1 to 4 and 7.
Leave out the microphone pitch feedback (5) and the lessons (6) unless asked.

## What it is
A new sidebar section, "Chants". It holds a library of short chants, each with
a follow-along view so you can learn to sing it.

## Content (public domain or freely licensed only)
- Gregorian chant from the Liber Usualis (1961 and earlier editions, public
  domain), as GABC notation. Fine to bundle: Kyrie XI Orbis Factor, Kyrie
  Lux et origo, Gloria VIII, Sanctus XVIII, Agnus Dei XVIII, Pater noster,
  Salve Regina (simple tone), Ave Maria, Veni Creator Spiritus, Te lucis,
  Ubi caritas, Adoro te devote, Victimae paschali laudes, Dies irae
  (excerpt), Puer natus est, Magnificat tone VIII, the psalm tones I to VIII.
- Simple English chant (Anglican chant, metrical psalms, Doxology).
- Orthodox: Byzantine "Lord have mercy", Trisagion. Check the licences.
- GregoBase (gregobase.selapa.net) has GABC files for most of these. Check
  each file's licence; the melodies themselves are public domain.

## Follow along
1. Render square-note (neume) notation from GABC with cairo. Keep it simple:
   punctum, virga, podatus, clivis, torculus, porrectus, quilisma, and the
   C and F clefs on a four-line staff. Latin text sits under the notes, with
   an English translation line.
2. Synthesise the melody, as in tools/mksounds.py, but at runtime: a
   voice-like tone made with formant filtering, or an organ. Play it through
   the existing GStreamer player.
3. Highlight the current syllable and neume as it plays, karaoke-style,
   using timing taken from the GABC note lengths.
4. Practice controls:
   - tempo slider
   - transpose up or down, to fit the user's voice
   - loop one phrase
   - "sing back" mode: a drone or the starting note only
   - "listen first, then sing"
5. Optional pitch feedback: listen on the microphone through GStreamer
   (autoaudiosrc), estimate the pitch (YIN), and draw the user's pitch line
   over the staff in real time, with green when on pitch and amber when off.
   Microphone access is opt-in.
6. Lessons: how to read neumes, the modes, psalm tones. Short pages with
   playable examples.
7. Recordings: where a freely licensed human recording exists, like the
   Paterm Kyrie (CC BY-SA 3.0), offer "Hear it sung" alongside the
   synthesised guide.

## Tech notes
- New crate or module: `chant` with a GABC parser (the format is documented
  at gregorio-project.github.io/gabc) and a notation layout.
- Store chants in app/data/chants/*.gabc, plus a small JSON index with the
  title, use, mode, source and licence.
- Credits go on the About page.
