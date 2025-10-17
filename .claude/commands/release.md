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

### 1. Version Management

- Update version in Cargo.toml to $ARGUMENTS
- Run `cargo check` to update Cargo.lock
- Generate changelog from commits since last stable release

### 2. Review Release Notes

**⚠️ REVIEW REQUIRED**: The release notes have been generated in
`changes/$ARGUMENTS.md`. Please review them carefully:

- Check that all important changes are highlighted appropriately
- Edit the highlights section to focus on user-facing improvements
- Ensure the changelog is accurate and complete

**Do not proceed to the next step until you are satisfied with the release
notes.**

### 3. Git Operations

- Commit changes with message: `chore: release $ARGUMENTS`
- Create and push git tag `v$ARGUMENTS`
- This triggers GitHub workflow to build cross-platform binaries

### 4. Wait for CI Completion

- Monitor GitHub release creation
- Ensure all artifacts are uploaded (macOS, Linux AMD64, Linux ARM64 tarballs)
- **Important**: Verify release notes are set correctly with `gh release view v$ARGUMENTS --json body`
- If release body is just the commit message, update it: `gh release edit v$ARGUMENTS --notes-file changes/$ARGUMENTS.md`

### 5. Homebrew Formula Update

- First check available assets: `gh release view v$ARGUMENTS`
- Download correct macOS tarball from GitHub release (check actual asset names)
- Calculate SHA256 checksum for the correct asset
- Update `../homebrew-tap/Formula/cross-stream.rb` with new version, URL, and
  checksum
- Commit homebrew formula changes

### 6. Cargo Registry Publication

- Run `cargo publish` to publish to crates.io

### 7. Manual Verification Required

**⚠️ macOS Verification Needed**

After homebrew formula is updated, please ask a macOS user to test:

```bash
brew uninstall cross-stream  # if previously installed
brew install cablehead/tap/cross-stream
xs --version  # should show v$ARGUMENTS
```

Confirm the installation works before proceeding to website update.

### 8. Website Update (Final Step)

**Only after verification passes:**

- Update `../www.cross.stream/www/index.html` release badge:
  - Update `version` attribute to `v$ARGUMENTS`
  - Update `release-date` attribute to current UTC timestamp (ISO 8601 format)
  - Update `release-url` attribute to `https://github.com/cablehead/xs/releases/tag/v$ARGUMENTS`
- Commit and push website changes to make the release publicly visible

## Rollback Plan

If verification fails:

1. Delete the git tag:
   `git tag -d v$ARGUMENTS && git push --delete origin v$ARGUMENTS`
2. Delete the GitHub release
3. Revert homebrew formula changes
4. Investigate and fix issues before retry

---

**Ready to execute release for version $ARGUMENTS?**
