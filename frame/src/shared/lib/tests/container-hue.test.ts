import { describe, expect, it } from "vitest";
import { CONTAINER_COLORS, containerHue } from "../container-hue";

describe("containerHue", () => {
  it("draws each container colour from its own avatar hue", () => {
    expect(containerHue(0)).toBe("var(--avatar-1-top)");
    expect(containerHue(CONTAINER_COLORS - 1)).toBe(`var(--avatar-${CONTAINER_COLORS}-top)`);
  });

  it("never names a hue outside the curated set", () => {
    expect(containerHue(CONTAINER_COLORS)).toBe("var(--avatar-1-top)");
    expect(containerHue(-3)).toBe("var(--avatar-1-top)");
    expect(containerHue(2.7)).toBe("var(--avatar-3-top)");
  });
});
