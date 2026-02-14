## Git Commit Style Preferences

**NEVER commit unless explicitly asked by the user.**

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

## Tone and Communication

- ASCII only. No em dashes, smart quotes, or other unicode punctuation. Use "--" only in code contexts, not as prose punctuation.
- No wasted words. No fluff. Each word should add value to the reader.
- Human readable and clear.
- Calm, matter-of-fact technical tone.

## Code Quality

Always run `./scripts/check.sh` before committing. Use `cargo fmt` to fix formatting issues.

## Release Process

Use `/release [version]` command to execute the automated release workflow. See `.claude/commands/release.md` for details.
