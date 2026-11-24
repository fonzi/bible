# Paid versions: notes

Status: notes only, nothing built. Bible stays free and offline first; this is
about letting people use translations they've paid for (NIV, ESV, NASB, NKJV…)
the same way.

## What we already have

- SWORD has **locked modules**: the text is downloaded encrypted, and a
  `CipherKey=` line in the module's `.conf` unlocks it. Bible already notices
  a `CipherKey` entry (`Module::is_locked`) and shows the module as locked in
  Downloads.
- Some publishers distribute their translations this way through SWORD
  sources: you download the locked module for free and buy the unlock key from
  the publisher or a seller they name.

## Option 1: unlock keys (smallest step)

1. In Downloads, a locked module gets an **Unlock…** button next to Download.
2. A dialog asks for the key, says where it can be bought (from the module's
   `.conf`: `UnlockURL=` when present), and writes `CipherKey=` into the
   user's copy of the `.conf` in `~/.sword/mods.d`.
3. Check the key by decrypting a known verse (Gen 1:1 or Matt 1:1); if the
   text comes out as garbage, say the key didn't work and leave the `.conf`
   as it was.
4. Needs the SWORD cipher (Sapphire II) in `bible-sword` to read the text.
5. The key never leaves the computer and is never logged.

No money passes through us, nothing to host, works offline after unlocking.

## Option 2: licensing directly from publishers

Publishers license their text to apps, often for a fee per user or per year,
with rules about offline copies, how much may be quoted or exported, and
attribution. That means contracts, payments and probably an account system,
which is against what Bible is for. Only worth doing if a publisher offers
free or flat terms for offline, open-source apps. Read each one's current
terms before planning anything; they change.

## Option 3: online APIs

Some translations are available through publisher APIs. These usually limit
how much text may be stored offline, which doesn't fit "offline first". Could
be an opt-in extra later, clearly marked as needing the internet.

## Open questions

- Which publishers currently sell SWORD unlock keys, and for which
  translations? (Check CrossWire's list of locked modules.)
- Do we show paid translations in Browse at all, or only once unlocked?
  Suggest: show them in their category with a "Paid: unlock key needed"
  label, never above the free ones.
- Flatpak and the Windows and macOS builds keep `.conf` files in the same
  places, so unlocking works the same everywhere.

## Suggested order

Option 1 first (Unlock button, key check, cipher). Revisit 2 and 3 only if
people ask for a specific translation that isn't sold as a SWORD key.
