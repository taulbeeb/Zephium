<script lang="ts">
  import { untrack } from "svelte";
  import { Dialog } from "bits-ui";
  import type { WebPanelView } from "$shared/ipc/bindings";
  import { tabs } from "$domain/tabs";
  import * as m from "$shared/i18n/messages";
  import Button from "$shared/ui/Button";
  import Field from "$shared/ui/Field";
  import { containerHue } from "$shared/lib/container-hue";

  let {
    open = $bindable(false),
    panel = null,
  }: {
    open?: boolean;
    /** The panel edited; none adds one. */
    panel?: WebPanelView | null;
  } = $props();

  let address = $state("");
  let container = $state<string | null>(null);
  let busy = $state(false);
  let failed = $state(false);
  let opener: HTMLElement | null = null;
  const containers = $derived(tabs.containers());
  const valid = $derived(address.trim().length > 0);

  // Each opening starts from the panel as it is; a projection arriving
  // while the sheet is open never overwrites what is being typed.
  $effect(() => {
    if (!open) return;
    untrack(() => {
      opener ??= document.activeElement instanceof HTMLElement ? document.activeElement : null;
      failed = false;
      address = panel?.tab.url ?? "";
      container = panel?.tab.container ?? null;
    });
  });

  const settled = (outcome: string) =>
    outcome === "applied" || outcome === "deferred" || outcome === "no_op";

  async function submit() {
    if (busy || !valid) return;
    busy = true;
    failed = false;
    let ok = true;
    if (!panel) {
      ok = settled((await tabs.addPanel(address.trim(), container)).outcome);
    } else {
      if (address.trim() !== (panel.tab.url ?? ""))
        ok = settled((await tabs.loadPanel(panel.tab.id, address.trim())).outcome);
      if (ok && container !== (panel.tab.container ?? null))
        ok = settled((await tabs.setPanelContainer(panel.tab.id, container)).outcome);
    }
    busy = false;
    if (ok) open = false;
    else failed = true;
  }

  async function remove() {
    if (!panel || busy) return;
    busy = true;
    failed = false;
    const ok = settled((await tabs.removePanel(panel.tab.id)).outcome);
    busy = false;
    if (ok) open = false;
    else failed = true;
  }
</script>

<Dialog.Root bind:open>
  <Dialog.Portal>
    <Dialog.Overlay class="settings-dialog-overlay" />
    <Dialog.Content
      class="settings-dialog panel-sheet"
      onCloseAutoFocus={(event) => {
        if (opener?.isConnected) {
          event.preventDefault();
          opener.focus();
        }
        opener = null;
      }}
    >
      <Dialog.Title class="settings-dialog-title"
        >{panel ? m.panels_edit_title() : m.panels_add_title()}</Dialog.Title
      >
      {#if !panel}<Dialog.Description class="settings-dialog-description"
          >{m.panels_add_body()}</Dialog.Description
        >{/if}
      <form
        class="sheet-body"
        onsubmit={(event) => {
          event.preventDefault();
          void submit();
        }}
      >
        <Field
          label={m.panels_address()}
          placeholder={m.panels_address_placeholder()}
          autocomplete="off"
          spellcheck="false"
          maxlength={2048}
          bind:value={address}
        />
        {#if containers.length > 0}
          <fieldset class="containers">
            <legend>{m.panels_container()}</legend>
            <label class="chip">
              <input
                type="radio"
                name="panel-container"
                checked={container === null}
                onchange={() => (container = null)}
              />{m.space_container_none()}
            </label>
            {#each containers as entry (entry.id)}
              <label class="chip" style:--container-hue={containerHue(entry.color)}>
                <input
                  type="radio"
                  name="panel-container"
                  checked={container === entry.id}
                  onchange={() => (container = entry.id)}
                /><span class="dot"></span>{entry.name}
              </label>
            {/each}
          </fieldset>
        {/if}
        {#if panel && container !== (panel.tab.container ?? null)}<p class="note">
            {m.panels_container_reload()}
          </p>{/if}
        {#if failed}<p class="failed" role="alert">{m.panels_failed()}</p>{/if}
        <footer>
          {#if panel}<div class="start">
              <Button variant="ghost" disabled={busy} onclick={() => void remove()}
                >{m.panels_remove()}</Button
              >
            </div>{/if}
          <Button onclick={() => (open = false)} disabled={busy}>{m.action_cancel()}</Button>
          <Button type="submit" variant="primary" pending={busy} disabled={!valid}
            >{panel ? m.panels_save() : m.panels_add_action()}</Button
          >
        </footer>
      </form>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<style>
  :global(.settings-dialog.panel-sheet) {
    width: min(440px, calc(100vw - 48px));
  }

  .sheet-body {
    display: grid;
    gap: 20px;
    margin-block-start: 20px;
  }

  .sheet-body footer {
    margin-top: 6px;
  }

  .start {
    margin-inline: -10px auto;
  }

  .containers {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin: 0;
    padding: 0;
    border: 0;
  }

  legend {
    margin-block-end: 8px;
    padding: 0;
    color: var(--color-label-secondary);
    font-size: var(--text-label);
    font-weight: 500;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-block-size: 26px;
    padding-inline: 10px;
    border-radius: var(--radius-capsule);
    background: var(--row-hover);
    color: var(--color-label-secondary);
    font-size: var(--text-label);
    cursor: default;
  }

  .chip:has(input:checked) {
    color: var(--color-text);
    box-shadow: inset 0 0 0 1.5px var(--container-hue, var(--color-accent));
  }

  .chip:has(input:focus-visible) {
    outline: 2px solid var(--color-accent);
    outline-offset: 2px;
  }

  .chip input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }

  .dot {
    inline-size: 8px;
    block-size: 8px;
    border-radius: var(--radius-capsule);
    background: var(--container-hue);
  }

  .note,
  .failed {
    margin: 0;
    font-size: var(--text-label);
    line-height: 1.5;
  }

  .note {
    color: var(--color-muted);
  }

  .failed {
    color: var(--color-danger);
  }
</style>
