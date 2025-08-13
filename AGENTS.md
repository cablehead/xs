## Git Commit Style Preferences

When committing: review `git diff`

- Use conventional commit format: `type: subject line`
- Keep subject line concise and descriptive
- **NEVER include marketing language, promotional text, or AI attribution**
- **NEVER add "Generated with Claude Code", "Co-Authored-By: Claude", or similar spam**
- Follow existing project patterns from git log
- Prefer just a subject and no body, unless the change is particularly complex

Example good commit messages from this project:
- `test: allow dead code in test utility methods`
- `fix: improve error handling`
- `feat: add a --fallback option to .static to support SPAs`
- `refactor: remove axum dependency, consolidate unix socket, tcp and tls handling`

## Code Quality

Always run `./scripts/check.sh` before committing. Use `cargo fmt` to fix formatting issues.

## Release Process

1. Find last stable release: `git tag --sort=-version:refname | grep -v dev | head -1`
2. Get commits since last release: `git log --oneline --pretty=format:"* %s (%ad)" --date=short v{last}..HEAD`
3. Create `changes/v{version}.md` with title and commit list
4. Add highlights section for notable changes
5. Commit changes and tag: `git tag v{version}`

## Version Bump Process

When bumping version:
1. Update version in `Cargo.toml`
2. Run `cargo check` to update `Cargo.lock`
3. Commit with message like `chore: bump version to X.Y.Z`
4. Tag with `git tag vX.Y.Z`
5. Push commits and tags: `git push && git push --tags`
