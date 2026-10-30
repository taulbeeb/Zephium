import { describe, expect, it } from "vitest";
import { badgeText, unreadCount } from "../lib/badge";

describe("unreadCount", () => {
  it("reads the counts chat and mail sites put in their titles", () => {
    expect(unreadCount("(3) Inbox")).toBe(3);
    expect(unreadCount("Inbox (12)")).toBe(12);
    expect(unreadCount("[7] Team Chat")).toBe(7);
    expect(unreadCount("4 • Slack")).toBe(4);
    expect(unreadCount("(99+) Inbox")).toBe(99);
  });

  it("does not read a number inside the words", () => {
    expect(unreadCount("Top 10 albums of 2024")).toBe(0);
    expect(unreadCount("Inbox")).toBe(0);
    expect(unreadCount("(0) Inbox")).toBe(0);
    expect(unreadCount(null)).toBe(0);
  });

  it("caps the badge text", () => {
    expect(badgeText(5)).toBe("5");
    expect(badgeText(150)).toBe("99+");
  });
});
