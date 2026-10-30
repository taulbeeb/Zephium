# Zephium Plus (taulbeeb fork)

This fork is [Zephium](https://github.com/zephium-browser/Zephium) with
two features added on top:

- **Containers:** Firefox-style separate sign-ins inside one profile. See
  [docs/containers.md](docs/containers.md).
- **Web panels:** sites docked or floating beside the page, after the
  Zen Sidebar script. See [docs/web-panels.md](docs/web-panels.md).

`zephium-plus` is the branch that holds both and the fork's default.
`feat/containers` and `feat/web-panels` are the history they were built on.

## Taking Zephium's updates

The fork's `upstream` remote is Zephium itself. Merge rather than rebase,
so the branch never rewrites history already pushed.

```bash
git checkout zephium-plus
git fetch upstream
git merge upstream/main
```

Then run the checks before pushing:

```bash
cargo test -p zephium-core -p zephium-app -p zephium-store
cd frame && pnpm install --frozen-lockfile && pnpm run check && pnpm run build
```

Conflicts are most likely where the features touch Zephium's own code:
the window layout (`crates/zephium-app/src/shell/window_layout.rs`),
session persistence (`crates/zephium-core/src/session.rs`,
`crates/zephium-store/src/hub/session.rs`), the macOS stage
(`crates/zephium-engine/src/platform/macos/stage.rs`) and the IPC bindings
(`frame/src/shared/ipc/bindings.ts`, which mirrors the desktop commands).

## Fork-only changes

These exist for this fork's CI and must be reverted before offering any of
this to Zephium as a pull request:

- `.github/workflows/ci.yml`: pushes to `zephium-plus` and `feat/**` run
  CI; the macOS job uses Intel runners with a 90 minute limit; the
  blocker seed probe runs only on Apple Silicon.

The dev build runs as "Zephium Dev" (`app.zephium.dev`) with its own data,
so it never touches an installed Zephium.
