# Release process

## Commit conventions

This project uses [Conventional Commits](https://www.conventionalcommits.org/) so that
`git-cliff` can generate the changelog automatically.

| Prefix | Changelog section | Use for |
|---|---|---|
| `feat:` | Added | New rules, CLI flags, features |
| `fix:` | Fixed | Bug fixes |
| `perf:` | Performance | Measurable performance improvements |
| `refactor:` | Changed | Internal restructuring, no behaviour change |
| `docs:` | Documentation | README, CHANGELOG, `docs/` |
| `test:` | Testing | New or updated tests only |
| `ci:` | CI | GitHub Actions workflow changes |
| `chore:` | Miscellaneous | Dependency bumps, tooling, housekeeping |

Add a scope in parentheses for clarity: `feat(RF02): add wildcard column rule`.

Breaking changes: append `!` after the type, or add `BREAKING CHANGE:` in the footer.

## Cutting a release

### 1. Update the version

```bash
# Edit Cargo.toml
version = "0.2.0"

# Regenerate Cargo.lock
cargo build
```

### 2. Update CHANGELOG.md

**Option A — automated (requires `cargo install git-cliff`):**
```bash
git cliff --unreleased --tag v0.2.0 --prepend CHANGELOG.md
```

**Option B — manual:**
- Rename `[Unreleased]` to `[0.2.0] - YYYY-MM-DD`
- Add a new empty `[Unreleased]` section at the top
- Update the comparison link at the bottom of `CHANGELOG.md`

### 3. Update pyproject.toml version

The `pyproject.toml` version is separate from `Cargo.toml`. Keep them in sync:

```toml
# pyproject.toml
[project]
version = "0.2.0"
```

Also update `__version__` in `python/squint/__init__.py`.

### 4. Commit and tag

```bash
git add Cargo.toml Cargo.lock CHANGELOG.md pyproject.toml python/squint/__init__.py
git commit -m "chore: release v0.2.0"
git tag -a v0.2.0 -m "v0.2.0"
git push && git push --tags
```

Pushing the tag triggers two workflows:
- **`release.yml`** — cargo-dist builds pre-compiled binaries and attaches them to the GitHub Release.
- **`pypi.yml`** — fires when the GitHub Release is _published_, builds platform-specific Python wheels with maturin, and publishes them to PyPI.

### 5. Publish to crates.io

```bash
cargo publish
```

Requires `CARGO_REGISTRY_TOKEN` set in your environment or `~/.cargo/credentials`.

## GitHub secrets and environments

| Secret / Permission | Used by | Purpose |
|---|---|---|
| `GITHUB_TOKEN` | `release.yml` | Create GitHub Release, upload assets |
| `CARGO_REGISTRY_TOKEN` | `release.yml` (publish job) | Publish to crates.io |
| PyPI Trusted Publishing | `pypi.yml` | Publish to PyPI (no API token needed) |

### Setting up PyPI Trusted Publishing

`pypi.yml` uses OIDC Trusted Publishing — no API token is required.

1. Go to **PyPI → Your Projects → squint → Publishing → Add a new publisher**.
2. Fill in:
   - **Owner**: `IlllIIIlllIlIlIIllllIIIlI`
   - **Repository**: `squint`
   - **Workflow name**: `pypi.yml`
   - **Environment name**: `pypi`
3. In the GitHub repo, create an environment named **`pypi`** (Settings → Environments).
4. That's it — the workflow will authenticate automatically on the next release.

## Versioning policy

- **Patch** (`0.1.x`) — bug fixes, rule accuracy improvements, performance
- **Minor** (`0.x.0`) — new rules, new CLI flags, new config options (backwards-compatible)
- **Major** (`x.0.0`) — breaking changes to config format, CLI flags, or rule IDs
