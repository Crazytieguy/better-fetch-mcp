# Development Guidelines

## Quality Gates
- Run `cargo fmt && cargo clippy -- -D warnings && cargo test` before commits
- Launch review agents in parallel for complex changes, iterate until positive reviews
- Build debug version after changes (`cargo build`), then ask user to restart for testing

## Testing & Security
- Add tests if they provide value (no "not critical for this version" mentality)
- Take security issues seriously - fix vulnerabilities regardless of perceived risk

## Dependencies & Documentation
- Add dependencies with `cargo add <crate>` to get latest versions
- Check generated docs in `target/doc-md` for dependency APIs (regenerate with `cargo doc-md`)

## Releases
- Update version in Cargo.toml and User-Agent strings
- Commit features separately from version bumps
- Create and push annotated tag (`git tag -a v0.x.y -m "Release v0.x.y" && git push origin v0.x.y`)
- The automated workflow will create the GitHub release and publish to npm (don't create the release manually)
