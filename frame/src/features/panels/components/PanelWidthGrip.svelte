<script lang="ts">
  import * as m from "$shared/i18n/messages";
  import { ArrowLeftRightIcon } from "@hugeicons/core-free-icons";
  import Icon from "$shared/ui/Icon";
  import { tabs } from "$domain/tabs";

  let { panel, width }: { panel: string; width: number } = $props();

  /** Match the panel width bounds in core panels. */
  const MIN_WIDTH = 280;
  const MAX_WIDTH = 960;
  const STEP = 16;

  // A floating panel lies over the page, where the interface cannot reach
  // its edge; this grip on the strip moves that edge instead. The panel
  // grows toward the page as the pointer does.
  let start: { pointer: number; x: number; width: number } | null = null;
  let frame = 0;
  let pending: number | null = null;

  function send() {
    frame = 0;
    if (pending === null) return;
    const next = pending;
    pending = null;
    void tabs.resizePanel(panel, next);
  }

  function request(next: number) {
    pending = Math.round(Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, next)));
    if (frame === 0) frame = requestAnimationFrame(send);
  }

  function down(event: PointerEvent) {
    if (event.button !== 0) return;
    event.preventDefault();
    start = { pointer: event.pointerId, x: event.clientX, width };
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  }

  function move(event: PointerEvent) {
    if (start?.pointer !== event.pointerId) return;
    request(start.width + start.x - event.clientX);
  }

  function up(event: PointerEvent) {
    if (start?.pointer !== event.pointerId) return;
    request(start.width + start.x - event.clientX);
    start = null;
  }

  function key(event: KeyboardEvent) {
    const step = event.key === "ArrowLeft" ? STEP : event.key === "ArrowRight" ? -STEP : 0;
    if (step === 0) return;
    event.preventDefault();
    request(width + step);
  }
</script>

<button
  type="button"
  class="grip"
  aria-label={m.panels_resize()}
  title={m.panels_resize_floating()}
  onpointerdown={down}
  onpointermove={move}
  onpointerup={up}
  onpointercancel={() => (start = null)}
  onkeydown={key}><Icon icon={ArrowLeftRightIcon} size={16} strokeWidth={1.8} /></button
>

<style>
  .grip {
    display: grid;
    inline-size: 28px;
    block-size: 28px;
    padding: 0;
    border: 0;
    border-radius: var(--radius-inset);
    background: transparent;
    color: var(--color-muted);
    place-items: center;
    cursor: col-resize;
    touch-action: none;
  }

  .grip:hover,
  .grip:focus-visible {
    outline: none;
    background: var(--row-hover);
    color: var(--color-text);
  }
</style>
