<script lang="ts">
  import type { WebPanelView } from "$shared/ipc/bindings";
  import { tabs } from "$domain/tabs";
  import { layout } from "$domain/layout";
  import { uiCommands as ui } from "$domain/ui-commands";
  import WebPanelStrip from "./WebPanelStrip.svelte";
  import PanelResizeHandle from "./PanelResizeHandle.svelte";

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

  async function sheet(panel: WebPanelView | null) {
    editing = panel;
    PanelSheet ??= (await import("./PanelSheet.svelte")).default;
    sheetOpen = true;
  }

  let handledCommand = 0;
  $effect(() => {
    const command = ui.uiCommand();
    if (command.seq === 0 || command.seq === handledCommand) return;
    handledCommand = command.seq;
    if (command.id === "panels.new" && tabs.profile()?.kind !== "incognito")
      queueMicrotask(() => void sheet(null));
  });
</script>

{#if place}
  <WebPanelStrip
    width={place.strip}
    onadd={() => void sheet(null)}
    onedit={(panel) => void sheet(panel)}
  />
  {#if open && place.width > 0}<PanelResizeHandle {place} panel={open.tab.id} />{/if}
{/if}
{#if PanelSheet}<PanelSheet bind:open={sheetOpen} panel={editing} />{/if}
