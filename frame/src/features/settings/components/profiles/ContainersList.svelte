<script lang="ts">
  import { Add01Icon } from "@hugeicons/core-free-icons";
  import type { ContainerView } from "$shared/ipc/bindings";
  import { tabs } from "$domain/tabs";
  import * as m from "$shared/i18n/messages";
  import Button from "$shared/ui/Button";
  import Icon from "$shared/ui/Icon";
  import { CONTAINER_COLORS, containerHue } from "$shared/lib/container-hue";
  import ContainerSheet from "./ContainerSheet.svelte";

  const list = $derived(tabs.containers());
  const privateWindow = $derived(tabs.profile()?.kind === "incognito");
  let sheetOpen = $state(false);
  let editing = $state.raw<ContainerView | null>(null);
  /** A new container starts on the first colour no other container wears. */
  const nextColor = $derived(
    Array.from({ length: CONTAINER_COLORS }, (_, index) => index).find(
      (color) => !list.some((container) => container.color === color),
    ) ?? 0,
  );

  function create() {
    editing = null;
    sheetOpen = true;
  }

  function edit(container: ContainerView) {
    editing = container;
    sheetOpen = true;
  }
</script>

{#if !privateWindow}
  <section class="containers" aria-label={m.containers_list_label()}>
    <h2>{m.containers_heading()}</h2>
    <ul class="list">
      {#each list as container (container.id)}
        <li class="row">
          <span class="mark" style:--container-hue={containerHue(container.color)}></span>
          <div class="copy">
            <strong>{container.name}</strong>
            <small>{m.containers_tab_count({ count: container.tabs })}</small>
          </div>
          <div class="actions">
            <Button
              size="compact"
              aria-label={m.containers_open_tab_label({ name: container.name })}
              onclick={() => void tabs.openContainerTab(container.id)}
              >{m.containers_open_tab()}</Button
            >
            <Button
              size="compact"
              variant="ghost"
              aria-label={m.containers_edit_label({ name: container.name })}
              onclick={() => edit(container)}>{m.containers_edit()}</Button
            >
          </div>
        </li>
      {/each}
      <li>
        <button type="button" class="add" onclick={create}>
          <span class="add-mark"><Icon icon={Add01Icon} size={16} strokeWidth={1.8} /></span>
          <span>{m.containers_new()}</span>
        </button>
      </li>
    </ul>
    <p class="note">{m.containers_list_note()}</p>
  </section>

  <ContainerSheet bind:open={sheetOpen} container={editing} color={nextColor} />
{/if}

<style>
  .containers {
    margin-block-end: 34px;
  }

  h2 {
    margin: 0 0 10px;
    padding-inline: 4px;
    color: var(--color-label-secondary);
    font-size: var(--text-label);
    font-weight: 600;
  }

  .list {
    overflow: hidden;
    margin: 0;
    padding: 0;
    border-radius: var(--radius-card);
    background: var(--color-surface);
    box-shadow: var(--shadow-card);
    list-style: none;
  }

  .row,
  .add {
    position: relative;
    display: flex;
    align-items: center;
    gap: 14px;
    box-sizing: border-box;
    min-block-size: 56px;
    padding: 10px 14px 10px 16px;
  }

  li + li > .add::before,
  li + .row::before {
    position: absolute;
    top: 0;
    block-size: 1px;
    background: var(--color-border);
    content: "";
    inset-inline: 46px 0;
  }

  .mark {
    flex: none;
    inline-size: 16px;
    block-size: 16px;
    border-radius: 50%;
    background: var(--container-hue);
  }

  .copy {
    display: grid;
    flex: 1;
    gap: 2px;
    min-inline-size: 0;
  }

  strong,
  small {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  strong {
    color: var(--color-text);
    font-size: var(--text-page-title);
    font-weight: 500;
    line-height: 19px;
  }

  small {
    color: var(--color-muted);
    font-size: var(--text-label);
    line-height: 1.5;
  }

  .actions {
    display: flex;
    flex: none;
    align-items: center;
    gap: 6px;
  }

  .add {
    inline-size: 100%;
    border: 0;
    background: transparent;
    color: var(--color-text);
    font: inherit;
    font-size: var(--text-page-title);
    text-align: start;
    transition: background-color var(--motion-fast) var(--ease-out);
  }

  .add:hover {
    background: var(--row-hover);
  }

  .add:focus-visible {
    outline-offset: -2px;
  }

  .add-mark {
    display: grid;
    place-items: center;
    inline-size: 16px;
    color: var(--color-label-secondary);
  }

  .note {
    margin: 10px 4px 0;
    color: var(--color-muted);
    font-size: var(--text-label);
    line-height: 1.5;
  }

  @media (forced-colors: active) {
    .mark {
      border: 1px solid CanvasText;
    }
  }
</style>
