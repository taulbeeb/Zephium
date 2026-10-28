/**
 * A container's colour index drawn from the avatar palette, so containers
 * read as kin to profiles and spaces. Matches `CONTAINER_COLORS` in
 * `zephium-core::containers`.
 */
export const CONTAINER_COLORS = 8;

export const containerHue = (color: number): string =>
  `var(--avatar-${(Math.max(0, Math.trunc(color)) % CONTAINER_COLORS) + 1}-top)`;
