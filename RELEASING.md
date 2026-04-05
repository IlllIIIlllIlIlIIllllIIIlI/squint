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

Bump in all three places and regenerate the lock file:

```bash
# Cargo.toml
version = "X.Y.Z"

# pyproject.toml
version = "X.Y.Z"

# editors/vscode/package.json
"version": "X.Y.Z"

# python/squint/__init__.py
__version__ = "X.Y.Z"

cargo build   # regenerates Cargo.lock
```

### 2. Update CHANGELOG.md

**Option A — automated (requires `cargo install git-cliff`):**
```bash
git cliff --unreleased --tag vX.Y.Z --prepend CHANGELOG.md
```

**Option B — manual:**
- Rename `[Unreleased]` to `[X.Y.Z] - YYYY-MM-DD`
- Add a new empty `[Unreleased]` section at the top
- Update the comparison links at the bottom of `CHANGELOG.md`

### 3. Commit and tag

```bash
git add Cargo.toml Cargo.lock CHANGELOG.md pyproject.toml python/squint/__init__.py editors/vscode/package.json
git commit -m "chore: release vX.Y.Z"
git tag vX.Y.Z
git push origin main
git push origin vX.Y.Z   # triggers release.yml and (after release publishes) pypi.yml
```

**Note:** version tags are protected — only repo admins with bypass permission can push
them. See repo Settings → Rules → "Protect version tags".

Pushing the tag triggers two workflows automatically:
- **`release.yml`** — cargo-dist builds cross-platform binaries, creates the GitHub
  Release, and publishes to crates.io (uses `CARGO_REGISTRY_TOKEN` repo secret).
- **`pypi.yml`** — fires when the GitHub Release is published; builds platform-specific
  Python wheels with maturin and publishes to PyPI as `pysquint` via OIDC Trusted
  Publishing (no API token needed).

## GitHub secrets and environments

| Secret / Permission | Used by | Purpose |
|---|---|---|
| `GITHUB_TOKEN` | `release.yml` | Create GitHub Release, upload assets |
| `CARGO_REGISTRY_TOKEN` | `release.yml` (publish job) | Publish to crates.io |
| PyPI Trusted Publishing | `pypi.yml` | Publish to PyPI (no API token needed) |

### Setting up PyPI Trusted Publishing

Already configured. If ever re-configuring (e.g. after a repo rename):

`pypi.yml` uses OIDC Trusted Publishing — no API token is required.

1. Go to **pypi.org → Account → Publishing → Add a new pending publisher**.
2. Fill in:
   - **PyPI project name**: `pysquint`
   - **Owner**: `IlllIIIlllIlIlIIllllIIIlI`
   - **Repository**: `squint`
   - **Workflow name**: `pypi.yml`
   - **Environment name**: `pypi`
3. In the GitHub repo, create an environment named **`pypi`** (Settings → Environments).
4. That's it — the workflow authenticates automatically via OIDC on the next release.

## Versioning policy

- **Patch** (`0.1.x`) — bug fixes, rule accuracy improvements, performance
- **Minor** (`0.x.0`) — new rules, new CLI flags, new config options (backwards-compatible)
- **Major** (`x.0.0`) — breaking changes to config format, CLI flags, or rule IDs
