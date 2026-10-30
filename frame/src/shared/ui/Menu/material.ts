/** Carry the trigger's surface policy across a DOM portal. No global menu override. */
export function menuMaterialFor(trigger: HTMLElement | null | undefined): "opaque" | undefined {
  return trigger?.closest("[data-menu-material]")?.getAttribute("data-menu-material") === "opaque"
    ? "opaque"
    : undefined;
}

/** A menu opened from a modal sheet portals to the body, under the sheet's
    layer; this lifts it over the sheet that owns it. */
export function menuLayerFor(trigger: HTMLElement | null | undefined): "dialog" | undefined {
  return trigger?.closest('[role="dialog"], [role="alertdialog"]') ? "dialog" : undefined;
}
