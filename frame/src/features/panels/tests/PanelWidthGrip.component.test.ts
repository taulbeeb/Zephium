import "$styles/global.css";
import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import { render } from "vitest-browser-svelte";

const resized = vi.hoisted(() => [] as number[]);
vi.mock("$domain/tabs", () => ({
  tabs: {
    resizePanel: (_id: string, width: number) => {
      resized.push(width);
      return Promise.resolve({ outcome: "applied" });
    },
  },
}));

import PanelWidthGrip from "../components/PanelWidthGrip.svelte";

const frame = () => new Promise((done) => requestAnimationFrame(() => done(null)));

test("a floating panel grows toward the page as the grip is dragged or keyed", async () => {
  await page.viewport(800, 600);
  render(PanelWidthGrip, { panel: "p", width: 400 });
  const grip = page.getByRole("button", { name: "Panel width" });
  (grip.element() as HTMLElement).focus();
  await userEvent.keyboard("{ArrowLeft}");
  await frame();
  expect(resized.at(-1)).toBe(416);
  await userEvent.keyboard("{ArrowRight}");
  await frame();
  expect(resized.at(-1)).toBe(384);
  const box = (grip.element() as HTMLElement).getBoundingClientRect();
  const x = box.left + box.width / 2;
  const y = box.top + box.height / 2;
  const target = grip.element() as HTMLElement;
  const pointer = { pointerId: 1, bubbles: true, button: 0, clientY: y };
  target.dispatchEvent(new PointerEvent("pointerdown", { ...pointer, clientX: x }));
  target.dispatchEvent(new PointerEvent("pointermove", { ...pointer, clientX: x - 100 }));
  target.dispatchEvent(new PointerEvent("pointerup", { ...pointer, clientX: x - 100 }));
  await frame();
  expect(resized.at(-1)).toBe(500);
  target.dispatchEvent(new PointerEvent("pointerdown", { ...pointer, clientX: x }));
  target.dispatchEvent(new PointerEvent("pointerup", { ...pointer, clientX: x + 900 }));
  await frame();
  expect(resized.at(-1)).toBe(280);
});
