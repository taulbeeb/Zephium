<script lang="ts">
  import * as m from "$shared/i18n/messages";
  import {
    Add01Icon,
    ArrowLeft01Icon,
    ArrowRight01Icon,
    Cancel01Icon,
    Globe02Icon,
    ArrowReloadHorizontalIcon,
  } from "@hugeicons/core-free-icons";
  import type { WebPanelView } from "$shared/ipc/bindings";
  import { tabs } from "$domain/tabs";
  import { favicons } from "$domain/favicons";
  import FavIcon from "$shared/ui/FavIcon";
  import IconButton from "$shared/ui/IconButton";
  import { containerHue } from "$shared/lib/container-hue";

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
  const open = $derived(tabs.openPanel());
  let dragged = $state<string | null>(null);
  let over = $state<string | null>(null);

  // Not a presentation row: a panel is never the active tab, so its icon
  // carries no tab sentinels for native to check against.
  const nameOf = (panel: WebPanelView) => {
    const container = tabs.containerOf(panel.tab);
    const title = panel.tab.title || m.untitled_tab();
    return container ? m.tab_in_container({ title, container: container.name }) : title;
  };

  function choose(panel: WebPanelView) {
    void tabs.openPanelBeside(panel.open ? null : panel.tab.id);
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

<nav class="strip" style:width={`${width}px`} aria-label={m.panels_strip_label()}>
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
          <FavIcon
            image={favicons.image(panel.tab.icon)}
            tone={favicons.tone(panel.tab.icon)}
            loading={panel.tab.loading}
            size={18}
            lit
            fallback={Globe02Icon}
            notched={!!panel.tab.audio}
          />
          {#if panel.tab.audio}<span class="audio" aria-hidden="true"></span>{/if}
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

  {#if open}
    <div class="controls" role="toolbar" aria-label={m.panels_controls_label()}>
      <IconButton
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
      />
      <IconButton
        icon={ArrowReloadHorizontalIcon}
        label={m.panels_reload()}
        onclick={() => void tabs.navigatePanel(open.tab.id, "reload")}
      />
      <IconButton
        icon={Cancel01Icon}
        label={m.panels_close()}
        onclick={() => void tabs.openPanelBeside(null)}
      />
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

  .controls {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
  }

  @media (forced-colors: active) {
    .site[aria-pressed="true"],
    .site[data-container] {
      outline: 2px solid CanvasText;
    }

    .audio {
      background: CanvasText;
    }
  }
</style>
