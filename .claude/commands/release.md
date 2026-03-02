---
allowed-tools: Bash, Edit, Read, Glob, WebFetch
argument-hint: [version] (e.g., 0.4.5)
description: Automated release process - version bump, tag, publish
---

# Automated Release Process

Execute the complete release workflow for cross-stream/xs project.

## Pre-flight Checks

Current repository status: !`git status`

Current branch: !`git branch --show-current`

Last few releases: !`git tag --sort=-version:refname | grep -v dev | head -5`

## Release Steps

### 1. Pre-Release Information Gathering

**Ask the user for the following before starting:**
- Cargo registry token (if not already set in environment)
- Confirm the version number: $ARGUMENTS

### 2. Version Management

- Update version in Cargo.toml to $ARGUMENTS
- Run `cargo check` to update Cargo.lock
- Generate changelog from commits since last stable release

### 3. Review Release Notes

**⚠️ REVIEW REQUIRED**: The release notes have been generated in
`changes/$ARGUMENTS.md`. Please review them carefully:

- Check that all important changes are highlighted appropriately
- Edit the highlights section to focus on user-facing improvements
- Ensure the changelog is accurate and complete
- **No soft line breaks** -- paragraphs should be single long lines, not wrapped at 80 columns. GitHub renders markdown with soft wraps, so hard breaks mid-paragraph show up as unwanted newlines in the release notes.

**Do not proceed to the next step until you are satisfied with the release
notes.**

### 4. Git Operations

- Commit changes with message: `chore: release $ARGUMENTS`
- Create and push git tag `v$ARGUMENTS`
- This triggers GitHub workflow to build cross-platform binaries
- **Output Release Binaries workflow watch command:**
  ```
  gh run watch <run-id> --repo cablehead/xs --exit-status
  ```

### 5. Parallel Prep (While CI Runs)

- Clone `../homebrew-tap` if not present: `git clone https://github.com/cablehead/homebrew-tap.git`
- Clone `../www.cross.stream` if not present: `git clone https://github.com/cablehead/www.cross.stream.git`
- Pre-stage website update in `../www.cross.stream/www/index.html`:
  - Update `version` attribute to `v$ARGUMENTS`
  - Update `release-url` to `https://github.com/cablehead/xs/releases/tag/v$ARGUMENTS`
  - Leave `release-date` for now -- it needs the full ISO timestamp from `publishedAt`, which isn't available until the GitHub release is created
  - **Do not commit or push yet**

### 6. Wait for CI Completion

- Get the latest workflow run ID: `gh run list --limit 1`
- Monitor build with: `gh run watch <run-id> --exit-status`
- Wait for all three builds to complete (linux-amd64, linux-arm64, macos-arm64)
- Verify GitHub release: `gh release view v$ARGUMENTS`
- Ensure all artifacts are uploaded (macOS, Linux AMD64, Linux ARM64 tarballs)
- **Important**: Verify release notes are set correctly with `gh release view v$ARGUMENTS --json body`
- If release body is just the commit message, update it: `gh release edit v$ARGUMENTS --notes-file changes/v$ARGUMENTS.md`

### 7. Homebrew Formula Update

- First check available assets: `gh release view v$ARGUMENTS`
- Download macOS tarball and calculate SHA256:
  ```bash
  cd /tmp
  curl -sL https://github.com/cablehead/xs/releases/download/v$ARGUMENTS/cross-stream-v$ARGUMENTS-macos.tar.gz -o cross-stream-v$ARGUMENTS-macos.tar.gz
  sha256sum cross-stream-v$ARGUMENTS-macos.tar.gz
  ```
- Update `../homebrew-tap/Formula/cross-stream.rb` with new version, URL, and SHA256 checksum
- Commit and push homebrew formula changes

### 8. Manual Verification Required

**⚠️ CRITICAL: macOS Verification BEFORE Publishing to Crates.io**

After homebrew formula is updated, **PAUSE** and ask a macOS user to test:

```bash
brew uninstall cross-stream  # if previously installed
brew install cablehead/tap/cross-stream
xs --version  # should show v$ARGUMENTS
```

**STOP HERE if verification fails.** Publishing to crates.io is irreversible.

### 9. Cargo Registry Publication

**Only proceed after macOS verification passes.**

- Use the cargo token provided in step 1: `export CARGO_REGISTRY_TOKEN="..."`
- Run `cargo publish` to publish to crates.io
- **Warning**: This step cannot be undone - you cannot unpublish from crates.io

### 10. Website Update (Final Step)

**Only after cargo publish succeeds:**

- Update `release-date` with the full ISO timestamp: `gh release view v$ARGUMENTS --json publishedAt --jq '.publishedAt'` -- must be the full timestamp (e.g. `2026-03-02T19:15:14Z`), not just a date, or the relative time display will be wrong
- Commit and push website changes to make the release publicly visible

### 11. Bump to Dev Version

- Update version in Cargo.toml to the next patch with `-dev` suffix (e.g. `0.11.0` -> `0.11.1-dev`)
- Run `cargo check` to update Cargo.lock
- Commit with message: `chore: bump to <version>-dev`

## Release Complete

The release is now public! Summary:
- ✅ GitHub release: https://github.com/cablehead/xs/releases/tag/v$ARGUMENTS
- ✅ Homebrew: `brew install cablehead/tap/cross-stream`
- ✅ Crates.io: `cargo install cross-stream`
- ✅ Website updated: https://cross.stream

## Rollback Plan

If verification fails **before cargo publish**:

1. Delete the git tag:
   `git tag -d v$ARGUMENTS && git push --delete origin v$ARGUMENTS`
2. Delete the GitHub release: `gh release delete v$ARGUMENTS`
3. Revert homebrew formula changes
4. Investigate and fix issues before retry

**Note**: If cargo publish has already completed, you cannot unpublish from crates.io.
You would need to publish a new patch version with the fix instead.

---

**Ready to execute release for version $ARGUMENTS?**
