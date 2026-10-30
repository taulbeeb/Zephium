/**
 * The unread count a page puts in its title, as chat and mail sites do:
 * "(3) Inbox", "Inbox (3)", "[3] Chat" or "3 • Slack". Only a count the
 * title leads or ends with is read, so a number inside the words is not.
 */
const LEADING = /^\s*[([]\s*(\d{1,5})\s*\+?\s*[)\]]/u;
const TRAILING = /[([]\s*(\d{1,5})\s*\+?\s*[)\]]\s*$/u;
const BULLETED = /^\s*(\d{1,5})\+?\s*[•·|-]\s/u;

export function unreadCount(title: string | null | undefined): number {
  if (!title) return 0;
  const found = LEADING.exec(title) ?? TRAILING.exec(title) ?? BULLETED.exec(title);
  const count = found ? Number(found[1]) : 0;
  return Number.isFinite(count) && count > 0 ? count : 0;
}

/** A count drawn in the strip's small badge. */
export const badgeText = (count: number): string => (count > 99 ? "99+" : String(count));
