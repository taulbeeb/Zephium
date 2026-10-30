# Web panels

Status: phases 1 to 3 built on the `feat/web-panels` branch of this fork,
on top of [containers](containers.md). Not an upstream feature.

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
| Overlay mode (slide over the page) | `Axis::Over`: a floating leaf the stage draws above the tree | 3 |
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

## Phase 1 as built

- **Add a panel** from the strip's +, from Add Web Panel… on the sidebar
  menu or View menu, or from a tab's menu with Add Page to Panels (the tab
  keeps its container in the panel).
- **The strip** at the trailing edge: click an icon to show or close that
  panel, drag icons to reorder, right-click to edit or remove one. Back,
  forward, reload and close for the open panel sit at the strip's foot.
- **Resize** by dragging the gap between page and panel, or with the arrow
  keys on it. Widths are 280 to 960 points, kept per panel.
- **Toggle Web Panel** is Cmd/Ctrl+Shift+L (Zen Sidebar's Ctrl+Shift+E is
  Zephium's Notes), and can be rebound in Keyboard settings.
- Links a panel opens arrive as tabs of the current space, in the panel's
  container. Panels and the open one come back after a restart.

Gaps closed after phase 1:

- **Hidden sidebar:** while the sidebar auto-hides, the open panel stays
  docked and only the strip goes; chrome raised over the page never spans
  the window. The strip returns with the sidebar.
- **Names:** a panel can be named in its sheet.
- **Zoom:** zoom in, out and back to actual size from the strip. Zoom
  follows the site's level, as it does in tabs.
- **Page requests:** permission prompts from the open panel are asked in
  the sidebar, as a tab's are. Its blocked pop-ups, app links and sign-ins
  show the tab request card there.

## Phase 2 as built

- **Per-panel settings** in the panel sheet: ask for the mobile site, load
  when Zephium starts, unload when closed, unload after being closed for a
  set time, and reload every so often. They persist with the panel.
- **Mobile site:** the engine builds the panel's views with a phone user
  agent (Safari on iPhone on macOS); changing it rebuilds the page.
- **Unread badges** on strip icons, read from counts in page titles such as
  "(3) Inbox".
- **Show Web Panel 1-9** (Ctrl+Option+digit on macOS, unbound on Windows)
  in the keyboard registry.
- **Move Tab to Panels** on the tab menu: the page becomes a panel and the
  tab closes.

## Phase 3 as built

- **Slide over the page:** a panel can float over the page's trailing edge
  instead of narrowing it, set in its sheet or with the pin button at the
  strip's foot. The split engine has a third axis, `Axis::Over`, composed
  at layout time and never stored: the page keeps its whole region and the
  panel's rect overlaps it. The macOS stage sorts a floating page and its
  cover above the other pages in place; Windows raises its container.
  There is no divider at a floating panel's edge, so it is resized while
  docked. It closes from its icon, the close button or the shortcut, not by
  clicking the page: the engine reports no page clicks to the shell.
- **Show only:** a CSS selector per panel. The shell builds a style that
  hides everything but what the selector finds and fills the panel with it,
  and hands it to the engine with `set_item_style`, which applies it with
  the page styles. Selectors that could leave their rule (braces,
  semicolons, comments, at-rules, escapes, unbalanced quotes) are refused.
- **Custom icon:** a symbol or emoji, chosen as a space's is, drawn on the
  strip instead of the site's icon.
- **Open Link in Web Panel** in the page's own menu. On macOS, wry gains
  `willOpenMenu:withEvent:`, where WebKit shows its real page menu; the item
  runs WebKit's Open Link in New Window and marks the new-window request it
  makes, which becomes a panel in the page's container instead of a tab.
  On Windows it is a WebView2 custom item for http(s) links.
- **Panel icons: Show at the edge** (Settings, Tabs, Web Panels). The strip
  waits as the window's own 8 point edge, kept as chrome, and comes out
  when the pointer reaches it. The page narrows while it is out; nothing of
  the chrome is drawn over the page.

- **Panel sheet over pages:** the interface draws the panel sheet, and on
  macOS native pages stand above the interface. While the sheet is open the
  shell takes the pages off the stage (`PanelCommand::Sheet`), as it does
  for a permission prompt, so the sheet is seen and its backdrop never
  catches clicks unseen.
- **Floating width:** a width grip in the strip's controls while the panel
  floats; drag toward the page to widen, or use the arrow keys.

Not done:

- **Search in Web Panel:** WebKit gives the browser no hook for the
  selected text in its menu. Type the search in the panel's address field.
- **Padding and animation preferences:** the strip follows Zephium's own
  density and Reduce Motion settings.
- The menu item's title is English only.

## Decisions

- **Scope:** panels are shared by every space of a profile, as in Zen
  Sidebar. The profile's first space anchors them.
- **Icons:** a strip at the window's right edge, beside the open panel. The
  chrome is one webview anchored at the window's left, so phase 1 gives it a
  second, right-hand region. The content region shrinks by the strip's width,
  and the page views never cover it.
- **Overlay:** phase 1 docks the panel and narrows the page. Slide-over waits
  for phase 3.
