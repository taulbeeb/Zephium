# Web panels

Status: plan, on the `feat/web-panels` branch of this fork, built on
[containers](containers.md). Not an upstream feature.

A web panel is a site kept docked beside the page, such as chat, music or
mail. It stays open while you switch tabs and spaces, and it has its own icon
on a strip of panel icons. It is the native successor to the
[Zen Sidebar](https://github.com/taulbeeb/Zen-Sidebar) userChrome script, and
it aims at that script's feature set.

## How it fits Zephium

Three things Zephium already has do most of the work.

**A panel is a tab item that never shows in the tab list.** Tabs already have
everything a panel needs: a native view, title and favicon tracking, audio
and mute, zoom, sleeping and waking (hibernation), crash recovery, a
container, and persistence. The plan adds one placement, `Placement::Panel`,
so panels reuse that whole machinery instead of duplicating it. The tab list,
Essentials, split view, archiving and Recently Closed all skip that placement.

**The panel is laid out by the split engine.** Each relayout wraps the
window's content tree in one more branch:

```
Branch { axis: Row, ratio, a: <the tabs' tree>, b: Leaf(panel) }
```

The ratio is computed from the panel's width in points every time the layout
runs, so the panel keeps its width when the window resizes. The engine
already lays out, reveals and hides the leaves of a tree. A docked panel
therefore needs no new native code on macOS or Windows, and the gutter
between page and panel is the existing split divider. Dragging it resizes
the panel.

**Containers give each panel its own sign-in.** A panel can be opened in any
container of its profile, which covers Zen Sidebar's per-panel
Multi-Account Containers.

## Rules it must keep

- **Privileged chrome never covers page pixels.** The panel icon strip is
  chrome, so it gets its own strip of the window, never an overlay on a page.
- **One native view is bound to one store.** A panel's container is chosen
  when the panel is made. Changing it rebuilds the panel, as it does for tabs.
- **Panels belong to a profile.** Private windows have none. Deleting a
  profile deletes its panels along with its data.
- **Nothing reaches the network before it is asked for.** An unloaded panel
  makes no view until it is opened, as a restored tab does today.

## Feature map

| Zen Sidebar feature | Zephium plan | Phase |
|---|---|---|
| Web panels, persistent, add/remove/reorder | `Placement::Panel` items, saved with the session | 1 |
| Right-side panel | Docked branch at the content's trailing edge | 1 |
| Toggle shortcut (Ctrl+Shift+E) | `panels.toggle` in the keymap registry | 1 |
| Drag to resize, width per panel | Split divider; width stored per panel | 1 |
| Containers per panel | `TabState::container` | 1 |
| Settings dialog: URL, title, icon | Panel sheet in the frame | 1 |
| Audio badge, mute | Existing `TabAudio` | 1 |
| Per-panel zoom | Existing zoom state | 1 |
| Load on startup, unload on close, unload timer | Existing hibernation, with per-panel policy | 2 |
| Auto-reload timer | Shell timer that issues a reload | 2 |
| Mobile user agent | Per-view user agent at view creation | 2 |
| Notification badge from the page title | Count parsed from the title the shell already holds | 2 |
| Per-panel shortcuts | Keymap entries generated per panel | 2 |
| Move Tab to Panel | Tab menu item; the tab is rebuilt as a panel | 2 |
| Overlay mode (slide over the page) | A floating leaf the engine stacks over the tree, as the Work pane does | 3 |
| Auto-hide strip, reveal on hover | The hidden-sidebar pattern, on the right | 3 |
| Open Link / Search in Panel from the page menu | Page context-menu hook on macOS (`willOpenMenu`) and the WebView2 menu filter | 3 |
| Show only one element (CSS selector) | A page-assist style for the panel's site | 3 |
| Tooltip, padding and animation preferences | Frame preferences | 3 |

## Phase 1: docked panels

1. **Core.** `Placement::Panel { space }`. If panels are shared by the whole
   profile, the profile's first space anchors them and they show in every
   space of the profile. `PanelState` (width, ordering) is kept beside the
   items, the way containers keep their tabs. Session
   persistence and restore get the same dangling-reference checks containers
   have. Tests.
2. **Shell.** Panel commands: add, edit, remove, reorder, show, hide. Choosing
   the active panel. The panel branch is composed into `relayout` and into
   focus and presentation scope. The tab list, Essentials, split, archive and
   Recently Closed skip panels. The panel tab follows its own lifecycle rules
   rather than the tab LRU. Projections and IPC.
3. **Desktop.** Tauri commands, the `panels.toggle` key, and a tab menu entry
   for opening the current page as a panel.
4. **Frame.** The panel icon strip with badges, audio dots and drag to
   reorder. A panel sheet for URL, title, container and width. A small panel
   toolbar for back, forward, reload, zoom and close.
5. **Tests**, then CI on macOS and Windows, then trying it on your Mac.

## Open decisions

- **Scope:** panels shared by every space of a profile (Zen Sidebar's
  behaviour), or each space keeping its own.
- **Where the icons live:** a strip at the window's right edge (the Zen
  Sidebar and Vivaldi shape), or a row in Zephium's own left sidebar. The
  right edge needs the chrome to draw a second strip. The left sidebar needs
  no layout change.
- **Overlay mode:** needed in phase 1, or fine in phase 3.
