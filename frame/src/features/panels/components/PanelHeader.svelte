<script lang="ts">
  import * as m from "$shared/i18n/messages";
  import {
    ArrowLeft01Icon,
    ArrowRight01Icon,
    ArrowReloadHorizontalIcon,
    Cancel01Icon,
    MinusSignIcon,
    PinOffIcon,
    PlusSignIcon,
  } from "@hugeicons/core-free-icons";
  import type { WebPanelView } from "$shared/ipc/bindings";
  import type { WebPanelPlace } from "$domain/layout";
  import { tabs } from "$domain/tabs";
  import { preferences } from "$domain/preferences";
  import IconButton from "$shared/ui/IconButton";

  let { place, panel }: { place: WebPanelPlace; panel: WebPanelView } = $props();

  const shows = (group: "navigation" | "reload" | "zoom" | "layout" | "close") =>
    preferences.value(`panels.toolbar-${group}`) !== "false";
  const id = $derived(panel.tab.id);
</script>

<!-- The docked panel's page starts below this band, which native leaves to
     the interface; the header stands in it, across the panel's top. -->
<div
  class="header"
  role="toolbar"
  aria-label={m.panels_controls_label()}
  style:left={`${place.x}px`}
  style:top={`${place.y}px`}
  style:width={`${place.width}px`}
  style:height={`${place.header}px`}
>
  {#if shows("navigation")}<IconButton
      icon={ArrowLeft01Icon}
      label={m.panels_back()}
      disabled={!panel.tab.can_go_back}
      onclick={() => void tabs.navigatePanel(id, "back")}
    /><IconButton
      icon={ArrowRight01Icon}
      label={m.panels_forward()}
      disabled={!panel.tab.can_go_forward}
      onclick={() => void tabs.navigatePanel(id, "forward")}
    />{/if}
  {#if shows("reload")}<IconButton
      icon={ArrowReloadHorizontalIcon}
      label={m.panels_reload()}
      onclick={() => void tabs.navigatePanel(id, "reload")}
    />{/if}
  <span class="spacer"></span>
  {#if shows("zoom")}<IconButton
      icon={MinusSignIcon}
      label={m.panels_zoom_out()}
      onclick={() => void tabs.zoomPanel(id, "out")}
    /><button
      type="button"
      class="zoom"
      title={m.panels_zoom_reset()}
      aria-label={m.panels_zoom_level({ percent: panel.tab.zoom ?? 100 })}
      onclick={() => void tabs.zoomPanel(id, "reset")}>{panel.tab.zoom ?? 100}%</button
    ><IconButton
      icon={PlusSignIcon}
      label={m.panels_zoom_in()}
      onclick={() => void tabs.zoomPanel(id, "in")}
    />{/if}
  {#if shows("layout")}<IconButton
      icon={PinOffIcon}
      label={m.panels_float()}
      onclick={() => void tabs.configurePanel(id, { ...panel.settings, floating: true })}
    />{/if}
  {#if shows("close")}<IconButton
      icon={Cancel01Icon}
      label={m.panels_close()}
      onclick={() => void tabs.openPanelBeside(null)}
    />{/if}
</div>

<style>
  .header {
    position: fixed;
    display: flex;
    align-items: center;
    gap: 2px;
    box-sizing: border-box;
    padding-inline: 4px;
    padding-block-end: 4px;
  }

  .spacer {
    flex: 1;
  }

  .zoom {
    min-inline-size: 40px;
    block-size: 28px;
    padding: 0 4px;
    border: 0;
    border-radius: var(--radius-inset);
    background: transparent;
    color: var(--color-muted);
    font: inherit;
    font-size: var(--text-label);
    font-variant-numeric: tabular-nums;
  }

  .zoom:hover,
  .zoom:focus-visible {
    outline: none;
    background: var(--row-hover);
    color: var(--color-text);
  }
</style>
