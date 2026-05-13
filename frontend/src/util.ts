/**
 * Normalize a raw theme string to the slug format the backend accepts:
 * lowercase ASCII letters and digits, single token, max 10 chars.
 *
 * Mirrors `validate_theme` in `backend/src/styles.rs`. The frontend uses this
 * to filter input live and to surface validation feedback (shake animation).
 */
export function normalizeTheme(raw: string): string {
  return raw
    .toLowerCase()
    .replace(/[^a-z0-9]/g, "")
    .slice(0, 10);
}

export function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}
