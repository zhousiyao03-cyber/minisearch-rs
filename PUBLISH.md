# Publishing to crates.io

This file is for the maintainer (you). End users don't need it.

## One-time setup

1. Sign up at https://crates.io/ — you'll be prompted to "Sign in with
   GitHub" using `zhousiyao03-cyber`. The crates.io account is just a
   GitHub-linked profile, no separate password.
2. Once signed in, go to https://crates.io/me — there's a section
   called "API Tokens".
3. Click "New Token", give it any name (e.g. `mac-laptop-2026`), and
   leave the default scope (`publish-new` + `publish-update`). Save the
   token shown — crates.io won't show it again.
4. Run `cargo login <token>` locally. The token is stored in
   `~/.cargo/credentials.toml`. Don't commit that file anywhere.

## Publishing v0.1.0

The first publish is also the most error-prone, so we dry-run first.

```bash
# 1. Sanity check: does cargo think the package is publishable?
cargo publish --dry-run

# This will:
#   - run `cargo package` to gather files according to the `include`
#     list in Cargo.toml,
#   - verify metadata (license, repo URL, description, keywords),
#   - produce a `target/package/minisearch-rs-0.1.0.crate` tarball,
#   - print warnings if anything is fishy.
# It will NOT upload anything.

# 2. If the dry-run is clean, do the real upload:
cargo publish
```

The first publish uploads the `.crate` tarball to crates.io and kicks
off the docs.rs build. Within a minute or two:

- `https://crates.io/crates/minisearch-rs` shows the crate page
- `https://docs.rs/minisearch-rs` builds the rustdoc site

Verify both before tagging the GitHub release.

## Future versions

For 0.1.x patch releases (no API breaks):

1. Bump `version = "0.1.1"` in `Cargo.toml`.
2. Add a new section to `CHANGELOG.md`.
3. Commit, then `cargo publish` (no dry-run needed once the workflow
   is settled — `cargo publish` itself will still verify locally).
4. Tag `v0.1.1` and push tags.

For 0.2.0 (new minor — API additions, no breaks):
- Same as above, just bump the minor.

For 1.0.0:
- Read https://doc.rust-lang.org/cargo/reference/semver.html first.
- We'll pick a stable-API moment to do this. Not yet.

## Troubleshooting

**"crate name minisearch-rs is taken"**
Someone got there first. Check https://crates.io/search?q=minisearch
to see what's free. We can rename to e.g. `bm25-mini` or
`minisearch-bm25` and update Cargo.toml accordingly.

**"missing or empty metadata fields"**
The `include` list in Cargo.toml requires LICENSE, CHANGELOG.md and
README.md to exist at the repo root. Make sure none have been
renamed or removed.

**"unsupported lock file version"**
Your Rust toolchain is older than the one that produced
`Cargo.lock`. `rustup update stable` and try again.
