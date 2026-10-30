import "$styles/global.css";
import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import { render } from "vitest-browser-svelte";
import Host from "./MenuLayerHost.svelte";

test("menus opened from a modal sheet draw over the sheet", async () => {
  await page.viewport(640, 480);
  const screen = await render(Host);
  try {
    for (const [label, row] of [
      ["Actions", "menuitem"],
      ["Choice", "option"],
    ] as const) {
      await page.getByRole("button", { name: label, exact: true }).click();
      const item = page.getByRole(row, { name: "One", exact: true });
      await expect.element(item).toBeVisible();
      await expect
        .poll(() => {
          const box = (item.element() as HTMLElement).getBoundingClientRect();
          const hit = document.elementFromPoint(box.left + box.width / 2, box.top + box.height / 2);
          return Boolean(hit && item.element().contains(hit));
        })
        .toBe(true);
      await userEvent.keyboard("{Escape}");
    }
  } finally {
    await screen.unmount();
  }
});
