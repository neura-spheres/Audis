import type { ReactNode } from "react";

/** Shared form controls, styled to match macOS rather than a web form. */

interface SegmentedControlProps<T extends string> {
  label: string;
  value: T;
  options: readonly { id: T; label: string }[];
  onChange: (next: T) => void;
}

/** A sunken track with a single raised selected segment. */
export function SegmentedControl<T extends string>({
  label,
  value,
  options,
  onChange,
}: SegmentedControlProps<T>) {
  return (
    <div
      role="radiogroup"
      aria-label={label}
      className="flex gap-0.5 p-0.5"
      style={{ background: "var(--surface-sunken)", borderRadius: "var(--radius-control)" }}
    >
      {options.map((option) => {
        const isSelected = option.id === value;
        return (
          <button
            key={option.id}
            type="button"
            role="radio"
            aria-checked={isSelected}
            onClick={() => onChange(option.id)}
            className="px-2.5 py-1 text-footnote whitespace-nowrap transition-colors"
            style={{
              borderRadius: "calc(var(--radius-control) - 2px)",
              transitionDuration: "var(--duration-fast)",
              transitionTimingFunction: "var(--ease-standard)",
              background: isSelected ? "var(--surface-elevated)" : "transparent",
              color: isSelected ? "var(--label-primary)" : "var(--label-secondary)",
              boxShadow: isSelected ? "var(--shadow-card)" : "none",
              fontWeight: isSelected ? 500 : 400,
            }}
          >
            {option.label}
          </button>
        );
      })}
    </div>
  );
}

interface SwitchProps {
  label: string;
  checked: boolean;
  onChange: (next: boolean) => void;
}

/** The macOS pill switch. */
export function Switch({ label, checked, onChange }: SwitchProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      onClick={() => onChange(!checked)}
      className="relative h-[22px] w-[38px] shrink-0 transition-colors"
      style={{
        borderRadius: "var(--radius-chip)",
        background: checked ? "var(--color-accent)" : "var(--surface-sunken)",
        transitionDuration: "var(--duration-fast)",
        transitionTimingFunction: "var(--ease-standard)",
        border: checked ? "none" : "0.5px solid var(--border-control)",
      }}
    >
      <span
        aria-hidden
        className="absolute top-[2px] block h-[18px] w-[18px] transition-transform"
        style={{
          borderRadius: "var(--radius-chip)",
          background: "#ffffff",
          boxShadow: "0 1px 2px rgb(0 0 0 / 0.2)",
          transform: checked ? "translateX(18px)" : "translateX(2px)",
          transitionDuration: "var(--duration-fast)",
          transitionTimingFunction: "var(--ease-standard)",
        }}
      />
    </button>
  );
}

type ButtonVariant = "standard" | "accent" | "danger";

interface ButtonProps {
  children: ReactNode;
  onClick: () => void;
  variant?: ButtonVariant;
  disabled?: boolean;
  /** Tooltip text. Also the accessible name unless `ariaLabel` is given. */
  title?: string;
  /** Accessible name. Needed when the visible label is ambiguous out of context */
  ariaLabel?: string;
}

export function Button({
  children,
  onClick,
  variant = "standard",
  disabled = false,
  title,
  ariaLabel,
}: ButtonProps) {
  const palette: Record<ButtonVariant, { background: string; color: string; border: string }> = {
    standard: {
      background: "var(--surface-elevated)",
      color: "var(--label-primary)",
      border: "0.5px solid var(--border-control)",
    },
    accent: {
      background: "var(--color-accent)",
      color: "var(--label-on-accent)",
      border: "none",
    },
    danger: {
      background: "transparent",
      color: "var(--color-danger)",
      border: "0.5px solid var(--color-danger)",
    },
  };
  const style = palette[variant];

  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      title={title}
      aria-label={ariaLabel ?? title}
      className="flex shrink-0 items-center gap-1.5 px-3 py-[5px] text-footnote font-medium whitespace-nowrap transition-opacity"
      style={{
        borderRadius: "var(--radius-control)",
        background: style.background,
        color: style.color,
        border: style.border,
        boxShadow: variant === "standard" ? "var(--shadow-card)" : "none",
        opacity: disabled ? 0.4 : 1,
        cursor: disabled ? "default" : "pointer",
        transitionDuration: "var(--duration-fast)",
      }}
    >
      {children}
    </button>
  );
}
