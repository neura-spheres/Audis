/** Formatting helpers shared across views. */

const UNITS = ["B", "KB", "MB", "GB", "TB"] as const;

/** Human-readable file size. */
export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";

  const exponent = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), UNITS.length - 1);
  const value = bytes / 1024 ** exponent;
  const rounded = exponent === 0 ? String(Math.round(value)) : value.toFixed(1).replace(/\.0$/, "");

  return `${rounded} ${UNITS[exponent]}`;
}

/** Relative time for a file's last-modified stamp, falling back to a date once */
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

/** A running clock: 0:07, 4:32, 1:02:09. */
export function formatClock(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 1000));
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const seconds = String(total % 60).padStart(2, "0");
  return hours > 0
    ? `${hours}:${String(minutes).padStart(2, "0")}:${seconds}`
    : `${minutes}:${seconds}`;
}

/** How long something lasted, in words: 45s, 12m 04s, 1h 02m. */
export function formatDuration(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 1000));
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const seconds = total % 60;
  if (hours > 0) return `${hours}h ${String(minutes).padStart(2, "0")}m`;
  if (minutes === 0) return `${seconds}s`;
  return `${minutes}m ${String(seconds).padStart(2, "0")}s`;
}

/** Which day group a session belongs in: Today, Yesterday, This Week, or a month. */
export function dayGroup(iso: string, now: Date = new Date()): string {
  const when = new Date(iso);
  if (Number.isNaN(when.getTime())) return "Earlier";

  const startOf = (date: Date) =>
    new Date(date.getFullYear(), date.getMonth(), date.getDate()).getTime();
  const days = Math.round((startOf(now) - startOf(when)) / 86_400_000);

  if (days <= 0) return "Today";
  if (days === 1) return "Yesterday";
  if (days < 7) return "This Week";
  return when.toLocaleDateString(undefined, { month: "long", year: "numeric" });
}

/** The time of day a session started: 14:05 or 2:05 PM, as Windows is set. */
export function formatTime(iso: string): string {
  const when = new Date(iso);
  if (Number.isNaN(when.getTime())) return "";
  return when.toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" });
}

/** A greeting for the time of day. */
export function greeting(now: Date = new Date()): string {
  const hour = now.getHours();
  if (hour < 5) return "Good evening";
  if (hour < 12) return "Good morning";
  if (hour < 18) return "Good afternoon";
  return "Good evening";
}
