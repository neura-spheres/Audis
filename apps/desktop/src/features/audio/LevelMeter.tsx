import type { AudioLevelEvent } from "@/schemas/ipc";

/** A signal-level meter. */
export function LevelMeter({ level }: { level: AudioLevelEvent | undefined }) {
  const rms = toScale(level?.rms ?? 0);
  const peak = toScale(level?.peak ?? 0);
  const clipping = level?.clipping ?? false;

  return (
    <div className="flex min-w-0 flex-1 flex-col gap-1">
      <div
        className="relative h-2 w-full overflow-hidden"
        style={{ background: "var(--surface-sunken)", borderRadius: "var(--radius-chip)" }}
        role="meter"
        aria-label="Signal level"
        aria-valuenow={Math.round((level?.rms ?? 0) * 100)}
        aria-valuemin={0}
        aria-valuemax={100}
      >
        <div
          className="absolute inset-y-0 left-0"
          style={{
            width: `${rms * 100}%`,
            background: clipping ? "var(--color-danger)" : "var(--color-success)",
            borderRadius: "var(--radius-chip)",
          }}
        />
        {peak > 0.01 ? (
          <div
            className="absolute inset-y-0 w-[2px]"
            style={{
              left: `calc(${Math.min(peak, 1) * 100}% - 1px)`,
              background: clipping ? "var(--color-danger)" : "var(--label-secondary)",
            }}
          />
        ) : null}
      </div>
    </div>
  );
}

/** Map a 0..1 amplitude onto a 0..1 bar position using dBFS. */
const FLOOR_DB = -60;

function toScale(amplitude: number): number {
  if (amplitude <= 0) return 0;
  const db = 20 * Math.log10(amplitude);
  if (db <= FLOOR_DB) return 0;
  return Math.min(1, (db - FLOOR_DB) / -FLOOR_DB);
}
