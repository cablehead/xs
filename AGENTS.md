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

Prefer calm, matter-of-fact technical tone like "docs: improve release process with lessons from 0.6.4".

## Code Quality

Always run `./scripts/check.sh` before committing. Use `cargo fmt` to fix formatting issues.

## Naming Conventions & Consistency

xs follows a consistent naming schema across CLI, API, and internal components.
Refer to [Naming Schema Documentation](docs/naming-schema/NAMING_SCHEMA.md) when:
- Adding new CLI commands or flags
- Designing new API endpoints
- Naming internal functions and structures
- Reviewing code for consistency

Key principles:
- Use `from-latest` and `from-id` for stream positioning (not `tail` or `last-id`)
- Use `head` for "most recent frame" (aligns with Git conventions)
- Use `cat`, `append`, `follow`, `remove` for core operations
- Maintain consistency between CLI flags, API routes, and internal naming

## Release Process

Use `/release [version]` command to execute the automated release workflow. See `.claude/commands/release.md` for details.
