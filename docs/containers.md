# Containers

Status: in development on the `feat/containers` branch of this fork. Not an
upstream feature.

A container is a separate cookie jar inside a profile. Tabs in the same
profile share history, bookmarks, extensions, settings and spaces, but a tab
opened in a container signs in, stores cookies and keeps site storage apart
from the profile's own tabs and from every other container. This is the same
idea as Firefox's Multi-Account Containers, and it is the isolation the web
panel sidebar will reuse for per-panel sign-ins.

Profiles remain the heavier boundary: a profile is a whole separate browser.
A container is the light one.

## Model

- `ContainerId` is a ULID like every other identity in `zephium-core::ids`.
- A container belongs to exactly one persistent profile (`Default` or
  `Named`). Private profiles have no containers: their store is already
  ephemeral and shared by design.
- A container has a bounded name, a colour index and a symbol index into
  curated sets, the same scheme profile avatars use, so a stored container
  never names a colour or glyph the interface cannot draw.
- A tab's container is fixed when the tab is created and never changes. A tab
  "moved" to another container is a new tab with a new native view, which is
  what keeps one view bound to one store for its whole life. Duplicating,
  reopening a closed tab and opening a link in a new tab keep the source
  tab's container.
- A tab with no container uses the profile's own store, exactly as before.

## Persistence

`SessionState` gains `containers: Vec<PersistedContainer>` and
`PersistedKind::Tab` and `PersistedClosedTab` gain an optional `container`.
All are `serde(default)` and skipped when empty, so snapshots written without
containers restore unchanged. Restore drops a container whose profile is
missing or private, and a tab whose container is missing or belongs to
another profile loses the tab rather than silently moving it into the
profile's own store: a page must never run with credentials it was not opened
with.

## Engine storage

`Partition` gains `Container(ProfileId, ContainerId)`. `Partition::profile()`
still answers the owning profile, so extensions, the blocker, permissions,
downloads, userscripts and profile retirement keep their profile scope.
Only the website data store differs.

The store identity is derived, never stored:

```
SHA-256("zephium.container-website-data.v1\0" || profile || container)[..16]
```

with UUID version 8 bits set, the same construction `work_website_data`
uses for Work's store. It is stable across launches, distinct per container
and per profile, and never equal to the profile's own identifier or Work's.

- **macOS**: the view is built from the profile's extension-bearing
  configuration and bound to `WKWebsiteDataStore(forIdentifier:)` with the
  derived identifier. Storage attestation checks the exact identifier.
- **Windows**: a WebView2 profile named `container-<id>` inside the
  profile's own environment (`ICoreWebView2ControllerOptions::ProfileName`),
  so the profile's data folder stays the single deletion root. After a view
  is built its `ProfileName` is the postcondition: a container's page must
  be in exactly its own profile, and no other page in any container's.
  Data calls reach a container through one of its pages, or a hidden one
  opened for the call. A deleted container's profile is found by name under
  `EBWebView` and cleared of everything; the empty folder goes with the
  Zephium profile.
- **Linux**: not a supported platform; container tabs are refused.

## Erasure

- Deleting a container closes its tabs, then removes its store
  (`removeDataStoreForIdentifier` on macOS).
- Deleting a profile removes every container store the profile owns along
  with the profile's own store and Work's.
- "Clear Data for This Site" on a container tab clears that container's
  store, because the shell passes the tab's partition.

## Interface

- Settings › Profile › Containers: create, rename, recolour, delete.
- Tab menu: "Open in Container ▸" (No Container, each container).
- Link menu: "Open Link in Container ▸".
- New tab: "New Container Tab ▸" in the File menu.
- Container tabs carry a coloured mark in the tab list and address field.

## Status and known gaps

Built on this branch: phases 1 to 5 below, on macOS and Windows.

- **Linux** refuses container tabs and hides the menus; no container store
  is ever opened there.
- **Windows extensions** are installed per WebView2 profile, so they do not
  run in container tabs on Windows.
- **Windows has not been run.** It type-checks and follows the pattern Work
  mode already uses for its own WebView2 profiles.
- **Settings › Privacy › Site data** lists only the profile's own store.
  Clearing data, all of it or one site's, reaches every container too.
- **Extensions** run in container tabs with the profile's extension runtime.
  An extension's own cookie access still reads the profile's store. This
  needs checking with real extensions in the running app.
- **Link menu**: pages use WebKit's context menu, so "Open Link in
  Container" is not there yet. Open the link, then use the tab menu's
  "Open in Container".

## Trying it

Build the development app on an Apple Silicon Mac (it runs as "Zephium
Dev" with its own data, apart from an installed Zephium):

```sh
pnpm install --frozen-lockfile
pnpm dev
```

Then Settings › Profile › Containers › New Container, and "New Tab" on the
container's row, or File › New Container Tab. Sign in to the same site in a
container tab and an ordinary tab to see two separate sessions.

## Phases

1. Core model, persistence and restore, with tests.
2. Engine partition and macOS storage binding, attestation and erasure.
3. Shell intents and projections, IPC bindings.
4. Interface.
5. Windows storage binding and qualification.
6. Web panel sidebar on top of containers.
