<script lang="ts">
  import * as m from "$shared/i18n/messages";
  import type { WebPanelPlace } from "$domain/layout";
  import { tabs } from "$domain/tabs";

  let { place, panel }: { place: WebPanelPlace; panel: string } = $props();

  /** Match `GAP` in core layout and the panel width bounds in core panels. */
  const GAP = 8;
  const MIN_WIDTH = 280;
  const MAX_WIDTH = 960;

  let frame = 0;
  let pending: number | null = null;
  let pointer: number | null = null;

  // The panel's trailing edge stays put; the handle moves its leading edge.
  const widthAt = (x: number) => place.x + place.width - x - GAP / 2;

  function send() {
    frame = 0;
    if (pending === null) return;
    const width = pending;
    pending = null;
    void tabs.resizePanel(panel, width);
  }

  function request(width: number) {
    pending = Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, width));
    if (frame === 0) frame = requestAnimationFrame(send);
  }

  function down(event: PointerEvent) {
    if (event.button !== 0) return;
    event.preventDefault();
    pointer = event.pointerId;
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  }

  function move(event: PointerEvent) {
    if (pointer !== event.pointerId) return;
    request(widthAt(event.clientX));
  }

  function up(event: PointerEvent) {
    if (pointer !== event.pointerId) return;
    pointer = null;
    request(widthAt(event.clientX));
  }
</script>

<!-- The gutter between page and panel is chrome beneath both; dragging it
     resizes the panel, as a split divider resizes its panes. -->
<div
  class="handle"
  style:left={`${place.x - GAP}px`}
  style:top={`${place.y}px`}
  style:height={`${place.height}px`}
>
  <input
    type="range"
    aria-label={m.panels_resize()}
    aria-orientation="vertical"
    min={MIN_WIDTH}
    max={MAX_WIDTH}
    step="8"
    value={Math.round(place.width)}
    onpointerdown={down}
    onpointermove={move}
    onpointerup={up}
    onpointercancel={() => (pointer = null)}
    oninput={(event) => request(Number(event.currentTarget.value))}
  />
  <span class="grip" aria-hidden="true"></span>
</div>

<style>
  .handle {
    position: fixed;
    inline-size: 8px;
  }

  input {
    position: absolute;
    inset: 0;
    inline-size: 100%;
    block-size: 100%;
    margin: 0;
    appearance: none;
    background: transparent;
    opacity: 0;
    cursor: col-resize;
    touch-action: none;
  }

  .grip {
    position: absolute;
    inset-block: 40%;
    inset-inline-start: 3px;
    inline-size: 2px;
    border-radius: var(--radius-capsule);
    background: var(--color-faint);
    opacity: 0;
    pointer-events: none;
    transition: opacity var(--motion-fast) var(--ease-out);
  }

  input:focus-visible + .grip,
  .handle:hover .grip {
    opacity: 1;
  }
</style>
