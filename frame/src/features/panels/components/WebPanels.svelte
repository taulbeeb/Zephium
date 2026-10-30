<script lang="ts">
  import type { WebPanelView } from "$shared/ipc/bindings";
  import { tabs } from "$domain/tabs";
  import { layout } from "$domain/layout";
  import { uiCommands as ui } from "$domain/ui-commands";
  import { preferences } from "$domain/preferences";
  import { PANEL_STRIP_WIDTH } from "../lib/strip";
  import WebPanelStrip from "./WebPanelStrip.svelte";
  import PanelResizeHandle from "./PanelResizeHandle.svelte";
  import PanelHeader from "./PanelHeader.svelte";

  type Sheet = typeof import("./PanelSheet.svelte").default;

  // The strip, the gutter handle and the sheet that adds or edits a panel.
  // The sheet lives here so "Add Web Panel…" works before any strip exists.
  const place = $derived(layout.webPanel());
  const open = $derived(tabs.openPanel());
  let sheetOpen = $state(false);
  let editing = $state.raw<WebPanelView | null>(null);
  // The sheet and its dialog load the first time one is asked for, so the
  // browser's first paint does not carry them.
  let PanelSheet = $state.raw<Sheet | null>(null);

  // Native pages stand above the interface, which draws the sheet; they
  // step aside before it opens and come back once it closes.
  async function sheet(panel: WebPanelView | null) {
    editing = panel;
    PanelSheet ??= (await import("./PanelSheet.svelte")).default;
    await tabs.panelSheet(true);
    sheetOpen = true;
  }
  let sheetShown = false;
  $effect(() => {
    if (sheetOpen) {
      sheetShown = true;
    } else if (sheetShown) {
      sheetShown = false;
      void tabs.panelSheet(false);
    }
  });

  // A strip set to wait keeps only the window's edge. The page beside it
  // takes the pointer, so native watches that edge and says when the
  // pointer reaches it; the strip comes out, and goes back a moment after
  // the pointer leaves it, or when the pointer never came onto it.
  const waiting = $derived(!!place && place.strip < PANEL_STRIP_WIDTH);
  const auto = $derived(preferences.value("panels.strip") === "auto");
  const watched = $derived(auto && !!place);
  let timer: ReturnType<typeof setTimeout> | undefined;
  let pointerIn = false;
  const after = (ms: number, peek: boolean) => {
    clearTimeout(timer);
    timer = setTimeout(() => void tabs.peekPanelStrip(peek), ms);
  };
  const stay = () => clearTimeout(timer);
  $effect(() => {
    if (!auto || waiting) return;
    if (!pointerIn) after(1200, false);
    return stay;
  });
  $effect(() => {
    if (!watched) return;
    void tabs.watchPanelEdge(true);
    const edge = (event: Event) => {
      if (!(event instanceof CustomEvent) || typeof event.detail?.near !== "boolean") return;
      if (event.detail.near) after(0, true);
      else if (!pointerIn) after(450, false);
    };
    window.addEventListener("zephium:panel-edge", edge);
    return () => {
      window.removeEventListener("zephium:panel-edge", edge);
      void tabs.watchPanelEdge(false);
    };
  });

  let handledCommand = 0;
  $effect(() => {
    const command = ui.uiCommand();
    if (command.seq === 0 || command.seq === handledCommand) return;
    handledCommand = command.seq;
    if (command.id === "panels.new" && tabs.profile()?.kind !== "incognito")
      queueMicrotask(() => void sheet(null));
  });
</script>

{#if place && waiting}
  <div
    class="edge"
    style:inline-size={`${place.strip}px`}
    aria-hidden="true"
    onpointerenter={() => after(90, true)}
    onpointerleave={stay}
  ></div>
{:else if place}
  <div
    class="strip-host"
    role="presentation"
    onpointerenter={auto
      ? () => {
          pointerIn = true;
          stay();
        }
      : undefined}
    onpointerleave={auto
      ? () => {
          pointerIn = false;
          after(450, false);
        }
      : undefined}
  >
    <WebPanelStrip
      width={place.strip}
      header={place.header > 0}
      onadd={() => void sheet(null)}
      onedit={(panel) => {
        stay();
        void sheet(panel);
      }}
    />
  </div>
  {#if open && place.width > 0 && place.header > 0}<PanelHeader {place} panel={open} />{/if}
  {#if open && place.width > 0 && !open.settings.floating}<PanelResizeHandle
      {place}
      panel={open.tab.id}
    />{/if}
{/if}
{#if PanelSheet}<PanelSheet bind:open={sheetOpen} panel={editing} />{/if}

<style>
  /* The window's own edge, kept as chrome so the pointer can reach it. */
  .edge {
    position: fixed;
    inset-block: 0;
    inset-inline-end: 0;
  }

  .strip-host {
    display: contents;
  }
</style>
