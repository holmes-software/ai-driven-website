import { describe, expect, test } from "vitest";
import { escapeHtml, normalizeTheme } from "./util";

describe("normalizeTheme", () => {
  test("lowercases", () => {
    expect(normalizeTheme("DARK")).toBe("dark");
    expect(normalizeTheme("MoDeRn")).toBe("modern");
  });

  test("strips disallowed characters", () => {
    expect(normalizeTheme("dark mode!")).toBe("darkmode");
    expect(normalizeTheme("dark-mode")).toBe("darkmode");
    expect(normalizeTheme("../../etc")).toBe("etc");
    expect(normalizeTheme("forest 🌲")).toBe("forest");
  });

  test("truncates to 10 chars", () => {
    expect(normalizeTheme("abcdefghijklmnop")).toBe("abcdefghij");
    expect(normalizeTheme("dark".repeat(5))).toBe("darkdarkda");
  });

  test("handles empty / whitespace", () => {
    expect(normalizeTheme("")).toBe("");
    expect(normalizeTheme("    ")).toBe("");
  });

  test("preserves valid digits", () => {
    expect(normalizeTheme("retro80")).toBe("retro80");
    expect(normalizeTheme("0123456789")).toBe("0123456789");
  });
});

describe("escapeHtml", () => {
  test("escapes HTML metacharacters", () => {
    expect(escapeHtml('<script>alert("x")</script>')).toBe(
      "&lt;script&gt;alert(&quot;x&quot;)&lt;/script&gt;",
    );
    expect(escapeHtml("a & b")).toBe("a &amp; b");
    expect(escapeHtml("'quote'")).toBe("&#39;quote&#39;");
  });

  test("leaves plain text unchanged", () => {
    expect(escapeHtml("hello world")).toBe("hello world");
  });
});
