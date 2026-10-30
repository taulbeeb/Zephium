import "$styles/global.css";
import { expect, test } from "vitest";
import { page } from "vitest/browser";
import { render } from "vitest-browser-svelte";
import PanelSheet from "../components/PanelSheet.svelte";

test("the panel sheet loads on demand and opens for a floating panel", async () => {
  const panel = {
    tab: { id: "p", url: "https://chat.example/", title: "Chat", container: null },
    width: 400,
    open: true,
    settings: {
      name: null,
      mobile: false,
      load_on_startup: false,
      unload_on_close: false,
      unload_after_minutes: null,
      reload_every_seconds: null,
      floating: true,
      selector: null,
      icon: { kind: "emoji", text: "💬" },
    },
  };
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  render(PanelSheet, { open: true, panel: panel as any });
  await expect.element(page.getByRole("dialog")).toBeVisible();
  await expect.element(page.getByText("Slide over the page")).toBeVisible();
});
