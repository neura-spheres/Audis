import type { ReactNode } from "react";

/** Apple's system colours, as the theme names them. */
export type Tint =
  | "blue"
  | "indigo"
  | "purple"
  | "pink"
  | "red"
  | "orange"
  | "yellow"
  | "green"
  | "teal"
  | "gray";

/**
 * A white glyph on a coloured rounded square, the way System Settings marks
 * each pane. Colour carries the category so the eye finds a section before it
 * reads the label.
 */
export function IconBadge({
  tint,
  size = 26,
  children,
}: {
  tint: Tint;
  size?: number;
  children: ReactNode;
}) {
  const colour = `var(--tint-${tint})`;
  const glyph = Math.round(size * 0.6);

  return (
    <span
      aria-hidden
      className="flex shrink-0 items-center justify-center"
      style={{
        width: size,
        height: size,
        borderRadius: Math.round(size * 0.27),
        color: "#ffffff",
        background: `linear-gradient(180deg, color-mix(in srgb, ${colour} 78%, #ffffff) 0%, ${colour} 100%)`,
        boxShadow: `inset 0 0 0 0.5px rgb(0 0 0 / 0.08), 0 0.5px 1px rgb(0 0 0 / 0.12)`,
      }}
    >
      <span
        className="flex items-center justify-center [&>svg]:h-full [&>svg]:w-full"
        style={{ width: glyph, height: glyph }}
      >
        {children}
      </span>
    </span>
  );
}

/** A provider's initial on a neutral square, standing in for a logo. */
export function ProviderMonogram({ name, size = 34 }: { name: string; size?: number }) {
  const initial =
    name
      .replace(/^(Google|Custom)\s*\(?/i, "")
      .charAt(0)
      .toUpperCase() || "?";
  return (
    <span
      aria-hidden
      className="flex shrink-0 items-center justify-center font-semibold"
      style={{
        width: size,
        height: size,
        borderRadius: Math.round(size * 0.27),
        background: "var(--surface-sunken)",
        color: "var(--label-secondary)",
        fontSize: Math.round(size * 0.44),
      }}
    >
      {initial}
    </span>
  );
}
