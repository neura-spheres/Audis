/** Formatting helpers shared across views. */

const UNITS = ["B", "KB", "MB", "GB", "TB"] as const;

/**
 * Human-readable file size.
 *
 * Uses 1024 as the divisor with KB/MB labels, matching what Windows shows in
 * File Explorer. Being consistent with the OS matters more here than being
 * pedantic about KiB.
 */
export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";

  const exponent = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), UNITS.length - 1);
  const value = bytes / 1024 ** exponent;
  // Whole numbers for bytes, one decimal above that, but not a trailing ".0".
  const rounded = exponent === 0 ? String(Math.round(value)) : value.toFixed(1).replace(/\.0$/, "");

  return `${rounded} ${UNITS[exponent]}`;
}

/**
 * Relative time for a file's last-modified stamp, falling back to a date once
 * it is old enough that "6 days ago" stops being useful.
 */
export function formatWhen(iso: string | null): string {
  if (!iso) return "";

  const when = new Date(iso);
  if (Number.isNaN(when.getTime())) return "";

  const seconds = (Date.now() - when.getTime()) / 1000;

  if (seconds < 60) return "just now";
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
  if (seconds < 86_400) return `${Math.floor(seconds / 3600)}h ago`;
  if (seconds < 604_800) return `${Math.floor(seconds / 86_400)}d ago`;

  return when.toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" });
}
