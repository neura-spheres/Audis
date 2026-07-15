import type { AudioLevelEvent } from "@/schemas/ipc";

/**
 * A signal-level meter.
 *
 * The bar tracks RMS because that is what "how loud is this" means to a
 * listener, while the thin marker tracks peak, which is what tells you whether
 * you are about to clip. Showing only one of the two would hide half the story.
 *
 * The scale is logarithmic. A linear meter spends most of its width on levels
 * nobody speaks at and leaves normal speech bunched near the left.
 */
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
            // No CSS transition: the meter already updates 25 times a second,
            // and animating between frames would add visible lag to a control
            // whose entire job is to feel immediate.
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

/**
 * Map a 0..1 amplitude onto a 0..1 bar position using dBFS.
 *
 * -60 dB is the floor: quieter than that is inaudible and does not deserve
 * meter width.
 */
const FLOOR_DB = -60;

function toScale(amplitude: number): number {
  if (amplitude <= 0) return 0;
  const db = 20 * Math.log10(amplitude);
  if (db <= FLOOR_DB) return 0;
  return Math.min(1, (db - FLOOR_DB) / -FLOOR_DB);
}
