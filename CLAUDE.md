# Working on Bible

## Releases are bundled, and only when the owner says so

The release workflow (`.github/workflows/release.yml`) builds and tags a
release when `version` in the workspace `Cargo.toml` changes. Don't release
per change: collect changes and release them together when asked.

While working:

1. Push changes without bumping `version`; CI tests every push, and nothing
   ships.
2. Add each user-facing change, in plain words, under `## Unreleased` at the
   top of `CHANGELOG.md`.

When the owner asks for a release:

1. Summarize what's under `## Unreleased` and agree what goes in.
2. Bump `version` in `Cargo.toml` (patch for fixes and polish, minor for new
   features), run `cargo build` so `Cargo.lock` follows, rename
   `## Unreleased` to `## <version>`, and update `pkgver` in
   `packaging/arch/PKGBUILD`.
3. Run `python3 scripts/metainfo-releases.py`, so the store listing (GNOME
   Software, Flathub) has the same notes. Commit and push.
4. Confirm the release exists:
   `gh api repos/fonzi/bible/releases --jq '.[0].tag_name'`
   should show the new `v<version>` with its files attached.

## Commits

- Author: `Alfonso Vazquez <626330+fonzi@users.noreply.github.com>`.
- End with `Co-Authored-By: Claude …`, and no session link line.
- No keys, tokens or secrets in the repo.

## Before committing

- `cargo build` (clippy alone doesn't rebuild the binary), then
  `cargo test --workspace`.
- UI changes: check the window at 1280 px for `exceeds AdwApplicationWindow
  width` warnings in the log.
- No mobile/phone layout work.
