<script lang="ts">
  import { untrack } from "svelte";
  import { Dialog } from "bits-ui";
  import type { ContainerView } from "$shared/ipc/bindings";
  import { tabs } from "$domain/tabs";
  import * as m from "$shared/i18n/messages";
  import Button from "$shared/ui/Button";
  import Field from "$shared/ui/Field";
  import { CONTAINER_COLORS, containerHue } from "$shared/lib/container-hue";

  let {
    open = $bindable(false),
    container = null,
    color = 0,
  }: {
    open?: boolean;
    /** The container edited; none creates one. */
    container?: ContainerView | null;
    /** A new container's first colour. */
    color?: number;
  } = $props();

  let name = $state("");
  let hue = $state(0);
  let step = $state<"edit" | "delete">("edit");
  let busy = $state(false);
  let failed = $state(false);
  let opener: HTMLElement | null = null;
  const valid = $derived(name.trim().length > 0);

  // Each opening starts from the container as it is; a projection arriving
  // while the sheet is open never overwrites what is being typed.
  $effect(() => {
    if (!open) return;
    untrack(() => {
      opener ??= document.activeElement instanceof HTMLElement ? document.activeElement : null;
      step = "edit";
      failed = false;
      name = container?.name ?? "";
      hue = container?.color ?? color;
    });
  });

  const settled = (outcome: string) =>
    outcome === "applied" || outcome === "deferred" || outcome === "no_op";

  async function submit() {
    if (busy || !valid) return;
    busy = true;
    failed = false;
    const result = container
      ? await tabs.updateContainer(container.id, name.trim(), hue, container.symbol)
      : await tabs.createContainer(name.trim(), hue, 0);
    busy = false;
    if (settled(result.outcome)) open = false;
    else failed = true;
  }

  async function remove() {
    if (!container || busy) return;
    busy = true;
    failed = false;
    const result = await tabs.deleteContainer(container.id);
    busy = false;
    if (settled(result.outcome)) open = false;
    else failed = true;
  }
</script>

<Dialog.Root bind:open>
  <Dialog.Portal>
    <Dialog.Overlay class="settings-dialog-overlay" />
    <Dialog.Content
      class="settings-dialog container-sheet"
      onCloseAutoFocus={(event) => {
        if (opener?.isConnected) {
          event.preventDefault();
          opener.focus();
        }
        opener = null;
      }}
    >
      {#if step === "delete" && container}
        <Dialog.Title class="settings-dialog-title"
          >{m.containers_delete_title({ name: container.name })}</Dialog.Title
        >
        <Dialog.Description class="settings-dialog-description"
          >{container.tabs > 0
            ? m.containers_delete_body_tabs({ count: container.tabs })
            : m.containers_delete_body()}</Dialog.Description
        >
        {#if failed}<p class="failed" role="alert">{m.containers_failed()}</p>{/if}
        <footer>
          <Button onclick={() => (step = "edit")} disabled={busy}>{m.action_cancel()}</Button>
          <Button variant="danger" pending={busy} onclick={() => void remove()}
            >{m.containers_delete_confirm()}</Button
          >
        </footer>
      {:else}
        <Dialog.Title class="settings-dialog-title"
          >{container ? m.containers_edit_title() : m.containers_create_title()}</Dialog.Title
        >
        {#if !container}<Dialog.Description class="settings-dialog-description"
            >{m.containers_create_body()}</Dialog.Description
          >{/if}
        <form
          class="sheet-body"
          onsubmit={(event) => {
            event.preventDefault();
            void submit();
          }}
        >
          <Field
            label={m.containers_name()}
            placeholder={m.containers_name_placeholder()}
            maxlength={64}
            autocomplete="off"
            spellcheck="false"
            bind:value={name}
          />
          <fieldset class="colors">
            <legend>{m.containers_color()}</legend>
            {#each { length: CONTAINER_COLORS } as _, index (index)}
              <label class="swatch" style:--container-hue={containerHue(index)}>
                <input
                  type="radio"
                  name="container-color"
                  value={index}
                  checked={hue === index}
                  aria-label={m.containers_color_option({ number: index + 1 })}
                  onchange={() => (hue = index)}
                />
              </label>
            {/each}
          </fieldset>
          {#if failed}<p class="failed" role="alert">{m.containers_failed()}</p>{/if}
          <footer>
            {#if container}<div class="start">
                <Button
                  variant="ghost"
                  disabled={busy}
                  onclick={() => {
                    failed = false;
                    step = "delete";
                  }}>{m.containers_delete_open()}</Button
                >
              </div>{/if}
            <Button onclick={() => (open = false)} disabled={busy}>{m.action_cancel()}</Button>
            <Button type="submit" variant="primary" pending={busy} disabled={!valid}
              >{container ? m.containers_save() : m.containers_create_action()}</Button
            >
          </footer>
        </form>
      {/if}
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<style>
  :global(.settings-dialog.container-sheet) {
    width: min(420px, calc(100vw - 48px));
  }

  .sheet-body {
    display: grid;
    gap: 22px;
    margin-block-start: 20px;
  }

  .sheet-body footer {
    margin-top: 6px;
  }

  .start {
    margin-inline: -10px auto;
  }

  .colors {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
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

  .swatch {
    display: grid;
    place-items: center;
    inline-size: 28px;
    block-size: 28px;
    border-radius: 50%;
    background: var(--container-hue);
    cursor: default;
  }

  .swatch:has(input:checked) {
    box-shadow:
      0 0 0 2px var(--color-surface),
      0 0 0 4px var(--container-hue);
  }

  .swatch:has(input:focus-visible) {
    outline: 2px solid var(--color-accent);
    outline-offset: 4px;
  }

  .swatch input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }

  .failed {
    margin: 0;
    color: var(--color-danger);
    font-size: var(--text-label);
    line-height: 1.5;
  }

  @media (forced-colors: active) {
    .swatch:has(input:checked) {
      outline: 2px solid CanvasText;
    }
  }
</style>
