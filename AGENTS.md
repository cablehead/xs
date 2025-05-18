## Testing

Before pushing or creating a PR:
- For Rust changes run `./scripts/check.sh`
- For docs changes run `cd ./docs && npm run build`

All changes must pass these checks.

## Commits

Use **conventional commit messages** for all commits and PR titles (e.g.
`feat(nu): add new parser`, `fix(engine): resolve job deadlock`).
