import { ErrorNotice } from "@/components/ErrorNotice";
import { SegmentedControl, Switch } from "@/components/controls";
import { useSettings } from "@/hooks/useSettings";
import type { CaptionSettings } from "@/schemas/ipc";

/** How captions look on screen. */
export function CaptionsView() {
  const { settings, error, update } = useSettings();

  if (error) return <ErrorNotice error={error} />;
  if (!settings) return null;

  const { captions } = settings;

  const set = (change: Partial<CaptionSettings>) =>
    update((current) => ({ ...current, captions: { ...current.captions, ...change } }));

  return (
    <div className="flex flex-col gap-5">
      <CaptionPreview captions={captions} />

      <section className="flex flex-col gap-3">
        <Row
          label="Background"
          help="A panel behind the words. Turn it off to see everything underneath; captions keep a dark outline so they stay readable."
        >
          <div className="flex items-center gap-3">
            <input
              type="range"
              min={0}
              max={100}
              step={5}
              value={captions.backgroundOpacity}
              onChange={(event) => set({ backgroundOpacity: Number(event.target.value) })}
              aria-label="Background opacity"
              className="w-40"
            />
            <span
              className="w-14 shrink-0 text-right text-footnote tabular-nums"
              style={{ color: "var(--label-secondary)" }}
            >
              {captions.backgroundOpacity === 0 ? "Off" : `${captions.backgroundOpacity}%`}
            </span>
          </div>
        </Row>

        <Row label="Text size" help="Bigger text is easier to read from across a room.">
          <SegmentedControl<string>
            label="Caption text size"
            value={sizeName(captions.fontSize)}
            options={[
              { id: "Small", label: "Small" },
              { id: "Medium", label: "Medium" },
              { id: "Large", label: "Large" },
              { id: "Huge", label: "Huge" },
            ]}
            onChange={(name) => set({ fontSize: SIZES[name] ?? 22 })}
          />
        </Row>

        <Row label="Lines on screen" help="How much of the conversation stays visible.">
          <SegmentedControl<string>
            label="Lines on screen"
            value={String(captions.maxLines)}
            options={[
              { id: "1", label: "1" },
              { id: "2", label: "2" },
              { id: "3", label: "3" },
              { id: "5", label: "5" },
            ]}
            onChange={(lines) => set({ maxLines: Number(lines) })}
          />
        </Row>

        <Row
          label="Show who is speaking"
          help="A small label and a coloured dot: green for you, blue for your computer's audio."
        >
          <Switch
            label="Show who is speaking"
            checked={captions.showSourceLabels}
            onChange={(showSourceLabels) => set({ showSourceLabels })}
          />
        </Row>

        <Row
          label="Click through captions"
          help="Captions ignore the mouse so clicks reach whatever is behind them; a small handle above them stays available to bring them back. When off, the captions light up and become draggable only while you point at them."
        >
          <Switch
            label="Click through captions"
            checked={captions.clickThrough}
            onChange={(clickThrough) => set({ clickThrough })}
          />
        </Row>
      </section>
    </div>
  );
}

/** A live sample. */
function CaptionPreview({ captions }: { captions: CaptionSettings }) {
  const opacity = captions.backgroundOpacity / 100;
  const hasPanel = opacity > 0.01;

  return (
    <div
      className="flex items-center justify-center overflow-hidden p-6"
      style={{
        borderRadius: "var(--radius-card)",
        background:
          "repeating-conic-gradient(#3a3f47 0% 25%, #22262c 0% 50%) 50% / 24px 24px, linear-gradient(120deg, #1f6feb33, #d2a8ff33)",
        minHeight: 130,
      }}
    >
      <div
        className="flex w-full flex-col gap-2"
        style={{
          padding: hasPanel ? "10px 16px" : "4px 8px",
          borderRadius: 16,
          background: hasPanel ? `rgba(14, 15, 17, ${opacity})` : "transparent",
          backdropFilter: hasPanel
            ? `blur(${Math.round(opacity * 22)}px) saturate(140%)`
            : undefined,
          border: hasPanel ? `1px solid rgba(255,255,255,${0.1 * opacity})` : undefined,
        }}
      >
        <p
          className="flex items-baseline gap-2.5 leading-[1.35]"
          style={{
            fontSize: Math.min(captions.fontSize, 30),
            fontWeight: 600,
            color: "#ffffff",
            textShadow: hasPanel
              ? "0 1px 2px rgba(0,0,0,0.5)"
              : "0 0 3px rgba(0,0,0,0.95), 0 0 8px rgba(0,0,0,0.8), 0 2px 4px rgba(0,0,0,0.9)",
          }}
        >
          {captions.showSourceLabels ? (
            <span
              className="flex shrink-0 items-center gap-1.5 whitespace-nowrap"
              style={{
                fontSize: Math.max(11, Math.round(Math.min(captions.fontSize, 30) * 0.42)),
                fontWeight: 600,
                letterSpacing: "0.04em",
                color: "rgba(255,255,255,0.62)",
                textShadow: "0 1px 3px rgba(0,0,0,0.9)",
              }}
            >
              <span
                aria-hidden
                style={{
                  width: 6,
                  height: 6,
                  borderRadius: "50%",
                  background: "#4ade80",
                  boxShadow: "0 0 8px #4ade80",
                }}
              />
              You
            </span>
          ) : null}
          <span>Ini contoh teks caption.</span>
        </p>
      </div>
    </div>
  );
}

const SIZES: Record<string, number> = { Small: 16, Medium: 22, Large: 30, Huge: 42 };

/** The nearest named size, so a saved value always selects something. */
function sizeName(fontSize: number): string {
  return (
    Object.entries(SIZES).sort(
      ([, a], [, b]) => Math.abs(a - fontSize) - Math.abs(b - fontSize),
    )[0]?.[0] ?? "Medium"
  );
}

function Row({
  label,
  help,
  children,
}: {
  label: string;
  help: string;
  children: React.ReactNode;
}) {
  return (
    <div
      className="flex items-center justify-between gap-4 p-3"
      style={{
        background: "var(--surface-content)",
        borderRadius: "var(--radius-card)",
        boxShadow: "var(--shadow-card)",
      }}
    >
      <div className="flex min-w-0 flex-col gap-0.5">
        <span className="text-subheadline">{label}</span>
        <span className="text-footnote" style={{ color: "var(--label-secondary)" }}>
          {help}
        </span>
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}
