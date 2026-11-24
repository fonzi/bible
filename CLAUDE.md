# Working on Bible

## Every push that changes the app ships a release

The release workflow (`.github/workflows/release.yml`) only builds and tags a
release when `version` in the workspace `Cargo.toml` changes. A push without a
bump runs CI and nothing else, and users never see the change.

So, before every push that touches the app, data or packaging:

1. Bump `version` in `Cargo.toml` (patch for fixes and polish, minor for new
   features), then `cargo build` so `Cargo.lock` follows.
2. Add a `## <version>` section at the top of `CHANGELOG.md`, in plain words
   for users; the release workflow puts it at the top of the release notes.
   Then run `python3 scripts/metainfo-releases.py`, so the store listing
   (GNOME Software, Flathub) has the same notes.
   Commit it with the bump and the change.
3. After pushing, confirm the release exists:
   `gh api repos/fonzi/bible/releases --jq '.[0].tag_name'`
   should show the new `v<version>` with the `.deb` attached.

Docs-only or website-only pushes don't need a bump.

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
