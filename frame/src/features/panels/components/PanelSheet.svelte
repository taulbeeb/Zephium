<script lang="ts">
  import { untrack } from "svelte";
  import { Dialog } from "bits-ui";
  import type { PanelSettingsInputV1, WebPanelView } from "$shared/ipc/bindings";
  import { tabs } from "$domain/tabs";
  import * as m from "$shared/i18n/messages";
  import Button from "$shared/ui/Button";
  import Field from "$shared/ui/Field";
  import Checkbox from "$shared/ui/Checkbox";
  import Select from "$shared/ui/Select";
  import { containerHue } from "$shared/lib/container-hue";
  import Icon from "$shared/ui/Icon";
  import { UnavailableIcon } from "@hugeicons/core-free-icons";
  import type { SpaceIconView } from "$shared/ipc/bindings";
  import { SPACE_SYMBOLS, emojiOf } from "$features/tabs";

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
  let name = $state("");
  let mobile = $state(false);
  let loadOnStartup = $state(false);
  let unloadOnClose = $state(false);
  let unloadAfter = $state("never");
  let reloadEvery = $state("never");
  let floating = $state(false);
  let selector = $state("");
  let icon = $state<SpaceIconView | null>(null);
  let emoji = $state("");

  function typeEmoji(text: string) {
    emoji = text;
    const found = emojiOf(text);
    if (found) icon = { kind: "emoji", text: found };
  }

  const unloadOptions = [
    { value: "never", label: m.panels_timer_never() },
    { value: "5", label: m.panels_minutes({ count: 5 }) },
    { value: "15", label: m.panels_minutes({ count: 15 }) },
    { value: "30", label: m.panels_minutes({ count: 30 }) },
    { value: "60", label: m.panels_minutes({ count: 60 }) },
  ];
  const reloadOptions = [
    { value: "never", label: m.panels_timer_never() },
    { value: "30", label: m.panels_seconds({ count: 30 }) },
    { value: "60", label: m.panels_minutes({ count: 1 }) },
    { value: "300", label: m.panels_minutes({ count: 5 }) },
    { value: "900", label: m.panels_minutes({ count: 15 }) },
    { value: "3600", label: m.panels_minutes({ count: 60 }) },
  ];
  // A value kept from before, not in the list, still shows as itself.
  const withValue = (
    options: { value: string; label: string }[],
    value: string,
    label: (count: number) => string,
  ) =>
    options.some((option) => option.value === value)
      ? options
      : [...options, { value, label: label(Number(value)) }];
  const timer = (value: number | null) => (value === null ? "never" : String(value));
  const parsed = (value: string) => (value === "never" ? null : Number(value));
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
      const settings = panel?.settings;
      name = settings?.name ?? "";
      mobile = settings?.mobile ?? false;
      loadOnStartup = settings?.load_on_startup ?? false;
      unloadOnClose = settings?.unload_on_close ?? false;
      unloadAfter = timer(settings?.unload_after_minutes ?? null);
      reloadEvery = timer(settings?.reload_every_seconds ?? null);
      floating = settings?.floating ?? false;
      selector = settings?.selector ?? "";
      icon = settings?.icon ?? null;
      emoji = icon?.kind === "emoji" ? icon.text : "";
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
      const settings: PanelSettingsInputV1 = {
        name: name.trim() || null,
        mobile,
        load_on_startup: loadOnStartup,
        unload_on_close: unloadOnClose,
        unload_after_minutes: parsed(unloadAfter),
        reload_every_seconds: parsed(reloadEvery),
        floating,
        selector: selector.trim() || null,
        icon,
      };
      if (ok) ok = settled((await tabs.configurePanel(panel.tab.id, settings)).outcome);
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
        {#if panel}<Field
            label={m.panels_name()}
            placeholder={panel.tab.title || m.untitled_tab()}
            autocomplete="off"
            maxlength={64}
            bind:value={name}
          />{/if}
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
        {#if panel}
          <div class="icons" role="radiogroup" aria-labelledby="panel-icon-label">
            <div class="icons-head">
              <span id="panel-icon-label">{m.panels_icon()}</span>
              <input
                class="emoji-field"
                value={emoji}
                oninput={(event) => typeEmoji(event.currentTarget.value)}
                placeholder={m.space_emoji_short()}
                aria-label={m.space_emoji_placeholder()}
                data-on={icon?.kind === "emoji" || undefined}
                maxlength={16}
              />
            </div>
            <div class="symbols">
              <button
                type="button"
                role="radio"
                class="symbol symbol-none"
                aria-checked={icon === null}
                aria-label={m.panels_icon_site()}
                title={m.panels_icon_site()}
                onclick={() => (icon = null)}
                ><Icon icon={UnavailableIcon} size={15} strokeWidth={1.6} /></button
              >
              {#each SPACE_SYMBOLS as symbol, index (index)}
                <button
                  type="button"
                  role="radio"
                  class="symbol"
                  aria-checked={icon?.kind === "symbol" && icon.index === index}
                  aria-label={m.space_symbol({ index: String(index + 1) })}
                  onclick={() => (icon = { kind: "symbol", index })}
                  ><Icon icon={symbol} size={15} strokeWidth={1.8} /></button
                >
              {/each}
            </div>
          </div>
          <div class="options">
            <Checkbox
              label={m.panels_floating()}
              description={m.panels_floating_note()}
              checked={floating}
              onchange={(value) => (floating = value)}
            />
            <Field
              label={m.panels_selector()}
              placeholder={m.panels_selector_placeholder()}
              autocomplete="off"
              spellcheck="false"
              maxlength={256}
              bind:value={selector}
            />
            <p class="hint">{m.panels_selector_note()}</p>
            <Checkbox
              label={m.panels_mobile()}
              description={m.panels_mobile_note()}
              checked={mobile}
              onchange={(value) => (mobile = value)}
            />
            <Checkbox
              label={m.panels_load_on_startup()}
              checked={loadOnStartup}
              onchange={(value) => (loadOnStartup = value)}
            />
            <Checkbox
              label={m.panels_unload_on_close()}
              checked={unloadOnClose}
              onchange={(value) => (unloadOnClose = value)}
            />
            <div class="timers">
              <Select
                label={m.panels_unload_after()}
                options={withValue(unloadOptions, unloadAfter, (count) =>
                  m.panels_minutes({ count }),
                )}
                bind:value={unloadAfter}
                disabled={unloadOnClose}
              />
              <Select
                label={m.panels_reload_every()}
                options={withValue(reloadOptions, reloadEvery, (count) =>
                  m.panels_seconds({ count }),
                )}
                bind:value={reloadEvery}
              />
            </div>
          </div>
        {/if}
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

  .options {
    display: grid;
    gap: 12px;
  }

  .hint {
    margin: -6px 0 0;
    color: var(--color-label-secondary);
    font-size: var(--text-label);
  }

  .icons {
    display: grid;
    gap: 6px;
  }

  .icons-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    color: var(--color-label-secondary);
    font-size: var(--text-label);
    font-weight: 500;
  }

  .emoji-field {
    width: 64px;
    height: 20px;
    padding-inline: 8px;
    border: 0;
    border-radius: var(--radius-capsule);
    background: var(--row-hover);
    color: var(--color-text);
    font: inherit;
    font-size: var(--text-label);
    text-align: center;
    outline: none;
  }

  .emoji-field::placeholder {
    color: var(--color-faint);
  }

  .emoji-field:focus,
  .emoji-field[data-on] {
    background: var(--row-active);
  }

  .emoji-field[data-on] {
    box-shadow: inset 0 0 0 1px var(--color-accent);
  }

  .symbols {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(24px, 1fr));
    gap: 1px;
  }

  .symbol {
    display: grid;
    height: 26px;
    padding: 0;
    border: 0;
    border-radius: var(--radius-inset);
    background: transparent;
    color: var(--color-muted);
    place-items: center;
    transition:
      background-color var(--motion-instant) var(--ease-out),
      color var(--motion-instant) var(--ease-out);
  }

  .symbol-none {
    color: var(--color-faint);
  }

  .symbol:hover,
  .symbol:focus-visible {
    outline: none;
    background: var(--row-hover);
    color: var(--color-text);
  }

  .symbol[aria-checked="true"] {
    background: var(--row-active);
    color: var(--color-text);
  }

  .timers {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
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
