<script lang="ts">
  import * as m from "$shared/i18n/messages";
  import {
    Add01Icon,
    ArrowLeft01Icon,
    ArrowRight01Icon,
    Cancel01Icon,
    Globe02Icon,
    ArrowReloadHorizontalIcon,
    MinusSignIcon,
    PlusSignIcon,
    PinIcon,
    PinOffIcon,
  } from "@hugeicons/core-free-icons";
  import type { WebPanelView } from "$shared/ipc/bindings";
  import { tabs } from "$domain/tabs";
  import { preferences } from "$domain/preferences";
  import { favicons } from "$domain/favicons";
  import FavIcon from "$shared/ui/FavIcon";
  import IconButton from "$shared/ui/IconButton";
  import { containerHue } from "$shared/lib/container-hue";
  import { SpaceGlyph } from "$features/tabs";
  import { badgeText, unreadCount } from "../lib/badge";
  import PanelWidthGrip from "./PanelWidthGrip.svelte";

  let {
    width,
    onadd,
    onedit,
  }: {
    width: number;
    onadd: () => void;
    /** Opens the sheet for one panel. */
    onedit: (panel: WebPanelView) => void;
  } = $props();

  const panels = $derived(tabs.webPanels());
  // Where the open panel's buttons stand, and which of them a person kept.
  const toolbar = $derived(preferences.value("panels.toolbar"));
  const shows = (group: "navigation" | "reload" | "zoom" | "layout" | "close") =>
    preferences.value(`panels.toolbar-${group}`) !== "false";
  const open = $derived(tabs.openPanel());
  let dragged = $state<string | null>(null);
  let over = $state<string | null>(null);

  // Not a presentation row: a panel is never the active tab, so its icon
  // carries no tab sentinels for native to check against.
  const nameOf = (panel: WebPanelView) => {
    const container = tabs.containerOf(panel.tab);
    const title = panel.settings.name || panel.tab.title || m.untitled_tab();
    const named = container ? m.tab_in_container({ title, container: container.name }) : title;
    const unread = unreadCount(panel.tab.title);
    return unread > 0 ? m.panels_unread({ name: named, count: unread }) : named;
  };

  function choose(panel: WebPanelView) {
    void tabs.openPanelBeside(panel.open ? null : panel.tab.id);
  }

  // Pinned beside the page, or sliding over it.
  function pin(panel: WebPanelView) {
    void tabs.configurePanel(panel.tab.id, {
      ...panel.settings,
      floating: !panel.settings.floating,
    });
  }

  function edit(event: MouseEvent, panel: WebPanelView) {
    event.preventDefault();
    onedit(panel);
  }

  function drop(target: string | null) {
    const id = dragged;
    dragged = null;
    over = null;
    if (id === null || id === target) return;
    void tabs.movePanel(id, target);
  }
</script>

<nav
  class="strip"
  data-toolbar={toolbar}
  style:width={`${width}px`}
  aria-label={m.panels_strip_label()}
>
  <ul class="sites">
    {#each panels as panel (panel.tab.id)}
      {@const container = tabs.containerOf(panel.tab)}
      <li
        data-over={over === panel.tab.id || undefined}
        ondragover={(event) => {
          if (dragged === null) return;
          event.preventDefault();
          over = panel.tab.id;
        }}
        ondragleave={() => over === panel.tab.id && (over = null)}
        ondrop={(event) => {
          event.preventDefault();
          drop(panel.tab.id);
        }}
      >
        <button
          type="button"
          class="site"
          draggable="true"
          aria-pressed={panel.open}
          aria-label={nameOf(panel)}
          title={nameOf(panel)}
          data-container={container ? "" : undefined}
          style:--container-hue={container ? containerHue(container.color) : undefined}
          onclick={() => choose(panel)}
          oncontextmenu={(event) => edit(event, panel)}
          ondragstart={(event) => {
            dragged = panel.tab.id;
            event.dataTransfer?.setData("text/plain", panel.tab.id);
          }}
          ondragend={() => {
            dragged = null;
            over = null;
          }}
        >
          {#if panel.settings.icon}<SpaceGlyph
              icon={panel.settings.icon}
              color={0}
              size={18}
            />{:else}<FavIcon
              image={favicons.image(panel.tab.icon)}
              tone={favicons.tone(panel.tab.icon)}
              loading={panel.tab.loading}
              size={18}
              lit
              fallback={Globe02Icon}
              notched={!!panel.tab.audio}
            />{/if}
          {#if panel.tab.audio}<span class="audio" aria-hidden="true"></span>{/if}
          {#if unreadCount(panel.tab.title) > 0}<span class="badge" aria-hidden="true"
              >{badgeText(unreadCount(panel.tab.title))}</span
            >{/if}
        </button>
      </li>
    {/each}
    <li
      class="add-slot"
      ondragover={(event) => dragged !== null && event.preventDefault()}
      ondrop={(event) => {
        event.preventDefault();
        drop(null);
      }}
    >
      <IconButton icon={Add01Icon} label={m.panels_add()} onclick={onadd} buttonSize={36} />
    </li>
  </ul>

  {#if open && toolbar !== "hidden"}
    <div class="controls" role="toolbar" aria-label={m.panels_controls_label()}>
      {#if shows("navigation")}<IconButton
          icon={ArrowLeft01Icon}
          label={m.panels_back()}
          disabled={!open.tab.can_go_back}
          onclick={() => void tabs.navigatePanel(open.tab.id, "back")}
        />
        <IconButton
          icon={ArrowRight01Icon}
          label={m.panels_forward()}
          disabled={!open.tab.can_go_forward}
          onclick={() => void tabs.navigatePanel(open.tab.id, "forward")}
        />{/if}
      {#if shows("reload")}<IconButton
          icon={ArrowReloadHorizontalIcon}
          label={m.panels_reload()}
          onclick={() => void tabs.navigatePanel(open.tab.id, "reload")}
        />{/if}
      {#if shows("zoom")}<IconButton
          icon={PlusSignIcon}
          label={m.panels_zoom_in()}
          onclick={() => void tabs.zoomPanel(open.tab.id, "in")}
        />
        <button
          type="button"
          class="zoom"
          title={m.panels_zoom_reset()}
          aria-label={m.panels_zoom_level({ percent: open.tab.zoom ?? 100 })}
          onclick={() => void tabs.zoomPanel(open.tab.id, "reset")}>{open.tab.zoom ?? 100}%</button
        >
        <IconButton
          icon={MinusSignIcon}
          label={m.panels_zoom_out()}
          onclick={() => void tabs.zoomPanel(open.tab.id, "out")}
        />{/if}
      <!-- A floating panel's only edge the pointer can reach is this grip,
           so it stays while any buttons do. -->
      {#if open.settings.floating}<PanelWidthGrip panel={open.tab.id} width={open.width} />{/if}
      {#if shows("layout")}<IconButton
          icon={open.settings.floating ? PinIcon : PinOffIcon}
          label={open.settings.floating ? m.panels_dock() : m.panels_float()}
          onclick={() => pin(open)}
        />{/if}
      {#if shows("close")}<IconButton
          icon={Cancel01Icon}
          label={m.panels_close()}
          onclick={() => void tabs.openPanelBeside(null)}
        />{/if}
    </div>
  {/if}
</nav>

<style>
  /* The strip stands at the window's trailing edge, beside the page, on the
     chrome that spans the window beneath the page while panels exist. */
  .strip {
    position: fixed;
    inset-block: 0;
    inset-inline-end: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: space-between;
    box-sizing: border-box;
    padding-block: 10px;
  }

  .sites {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .sites > li {
    position: relative;
  }

  .sites > li[data-over]::before {
    position: absolute;
    inset-block-start: -4px;
    inset-inline: 6px;
    block-size: 2px;
    border-radius: var(--radius-capsule);
    background: var(--color-accent);
    content: "";
  }

  .site {
    position: relative;
    display: grid;
    place-items: center;
    inline-size: 36px;
    block-size: 36px;
    border: 0;
    border-radius: var(--radius-card);
    background: var(--color-tile);
    box-shadow: var(--shine), var(--shadow-tile);
    cursor: default;
    transition:
      background-color var(--motion-fast) var(--ease-out),
      box-shadow var(--motion-fast) var(--ease-out),
      scale var(--motion-slow) var(--ease-spring);
  }

  .site:hover {
    background: var(--color-tile-hover);
  }

  .site:active {
    scale: 0.94;
    transition-duration: var(--motion-instant);
  }

  .site[aria-pressed="true"] {
    background: var(--color-fill-active);
    box-shadow: var(--row-rim);
  }

  .site[data-container] {
    background: color-mix(in oklab, var(--container-hue) 20%, var(--color-tile));
    box-shadow:
      inset 0 0 0 1.5px var(--container-hue),
      var(--shadow-tile);
  }

  .site:focus-visible {
    outline-offset: 2px;
  }

  .audio {
    position: absolute;
    inset-block-end: 3px;
    inset-inline-end: 3px;
    inline-size: 6px;
    block-size: 6px;
    border-radius: var(--radius-capsule);
    background: var(--color-accent);
  }

  .badge {
    position: absolute;
    inset-block-start: -4px;
    inset-inline-end: -5px;
    min-inline-size: 16px;
    block-size: 16px;
    padding-inline: 4px;
    box-sizing: border-box;
    border-radius: var(--radius-capsule);
    background: var(--color-danger);
    color: var(--color-on-accent);
    font-size: 10px;
    font-weight: 600;
    line-height: 16px;
    text-align: center;
  }

  .zoom {
    inline-size: 40px;
    block-size: 20px;
    padding: 0;
    border: 0;
    border-radius: var(--radius-inset);
    background: transparent;
    color: var(--color-muted);
    font: inherit;
    font-size: var(--text-caption);
    font-variant-numeric: tabular-nums;
    cursor: default;
  }

  .zoom:hover {
    background: var(--row-hover);
    color: var(--color-text);
  }

  .controls {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
  }

  /* Buttons at the top come first, set off from the icons below them. */
  .strip[data-toolbar="top"] {
    justify-content: flex-start;
    gap: 10px;
  }

  .strip[data-toolbar="top"] .controls {
    order: -1;
    padding-block-end: 10px;
    border-block-end: 1px solid var(--color-border);
  }

  @media (forced-colors: active) {
    .site[aria-pressed="true"],
    .site[data-container] {
      outline: 2px solid CanvasText;
    }

    .audio,
    .badge {
      background: CanvasText;
      color: Canvas;
    }
  }
</style>
