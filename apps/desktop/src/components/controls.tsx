import { useState, type CSSProperties, type ReactNode } from "react";

/** Shared form controls, styled to match macOS rather than a web form. */

interface SegmentedControlProps<T extends string> {
  label: string;
  value: T;
  options: readonly { id: T; label: string }[];
  onChange: (next: T) => void;
}

/**
 * A sunken track with a raised thumb that slides to the selected segment.
 *
 * Segments share the width equally, as AppKit's do, which is also what lets
 * the thumb move by whole segments without measuring anything.
 */
export function SegmentedControl<T extends string>({
  label,
  value,
  options,
  onChange,
}: SegmentedControlProps<T>) {
  const index = Math.max(
    0,
    options.findIndex((option) => option.id === value),
  );
  const count = Math.max(1, options.length);

  return (
    <div
      role="radiogroup"
      aria-label={label}
      className="relative grid p-[2px]"
      style={{
        gridAutoFlow: "column",
        gridAutoColumns: "1fr",
        background: "var(--surface-sunken)",
        borderRadius: "var(--radius-control)",
      }}
    >
      <span
        aria-hidden
        className="pointer-events-none absolute top-[2px] bottom-[2px] left-[2px]"
        style={{
          width: `calc((100% - 4px) / ${count})`,
          transform: `translateX(${index * 100}%)`,
          background: "var(--surface-elevated)",
          borderRadius: "calc(var(--radius-control) - 2px)",
          boxShadow: "0 1px 3px rgb(0 0 0 / 0.12), 0 0 0 0.5px rgb(0 0 0 / 0.06)",
          transition: `transform var(--spring-snappy-duration) var(--spring-snappy)`,
        }}
      />
      {options.map((option) => {
        const isSelected = option.id === value;
        return (
          <button
            key={option.id}
            type="button"
            role="radio"
            aria-checked={isSelected}
            onClick={() => onChange(option.id)}
            className="relative px-3 py-[3px] text-footnote whitespace-nowrap"
            style={{
              color: isSelected ? "var(--label-primary)" : "var(--label-secondary)",
              fontWeight: isSelected ? 500 : 400,
              transition: "color var(--duration-fast) var(--ease-standard)",
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
  disabled?: boolean;
}

/**
 * The pill switch. The knob stretches while pressed and springs across on
 * release, as the native one does.
 */
export function Switch({ label, checked, onChange, disabled = false }: SwitchProps) {
  const [pressed, setPressed] = useState(false);
  const knob = pressed ? 22 : 18;

  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      onPointerDown={() => setPressed(true)}
      onPointerUp={() => setPressed(false)}
      onPointerLeave={() => setPressed(false)}
      className="relative h-[22px] w-[38px] shrink-0"
      style={{
        borderRadius: "var(--radius-chip)",
        background: checked ? "var(--color-accent)" : "var(--surface-sunken)",
        boxShadow: checked ? "none" : "inset 0 0 0 0.5px var(--border-control)",
        opacity: disabled ? 0.4 : 1,
        transition: "background-color var(--duration-standard) var(--ease-standard)",
      }}
    >
      <span
        aria-hidden
        className="absolute top-[2px] block h-[18px]"
        style={{
          width: knob,
          borderRadius: "var(--radius-chip)",
          background: "#ffffff",
          boxShadow: "0 1px 3px rgb(0 0 0 / 0.22), 0 0 0 0.5px rgb(0 0 0 / 0.04)",
          transform: checked ? `translateX(${36 - knob}px)` : "translateX(2px)",
          transition: `transform var(--spring-snappy-duration) var(--spring-snappy), width var(--spring-snappy-duration) var(--spring-snappy)`,
        }}
      />
    </button>
  );
}

type ButtonVariant = "standard" | "accent" | "danger" | "plain" | "tinted";

interface ButtonProps {
  children: ReactNode;
  onClick: () => void;
  variant?: ButtonVariant;
  /** `large` is for the one primary action on a surface. */
  size?: "regular" | "large";
  disabled?: boolean;
  /** Tooltip text. Also the accessible name unless `ariaLabel` is given. */
  title?: string;
  /** Accessible name. Needed when the visible label is ambiguous out of context */
  ariaLabel?: string;
}

const BUTTON_PALETTE: Record<ButtonVariant, CSSProperties> = {
  standard: {
    background: "var(--surface-elevated)",
    color: "var(--label-primary)",
    boxShadow: "0 0.5px 1.5px rgb(0 0 0 / 0.1), 0 0 0 0.5px var(--border-control)",
  },
  accent: {
    background: "var(--color-accent)",
    color: "var(--label-on-accent)",
    boxShadow: "0 1px 2px color-mix(in srgb, var(--color-accent) 35%, transparent)",
  },
  danger: {
    background: "color-mix(in srgb, var(--color-danger) 12%, transparent)",
    color: "var(--color-danger)",
  },
  tinted: {
    background: "var(--accent-soft)",
    color: "var(--color-accent)",
  },
  plain: {
    background: "transparent",
    color: "var(--color-accent)",
  },
};

export function Button({
  children,
  onClick,
  variant = "standard",
  size = "regular",
  disabled = false,
  title,
  ariaLabel,
}: ButtonProps) {
  const large = size === "large";

  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      title={title}
      aria-label={ariaLabel ?? title}
      className={`audis-pressable flex shrink-0 items-center justify-center gap-1.5 font-medium whitespace-nowrap ${
        large ? "px-4 py-[7px] text-subheadline" : "px-3 py-[5px] text-footnote"
      } ${variant === "plain" ? "px-1.5" : ""}`}
      style={{
        ...BUTTON_PALETTE[variant],
        borderRadius: large ? "var(--radius-chip)" : "var(--radius-control)",
        opacity: disabled ? 0.4 : 1,
      }}
    >
      {children}
    </button>
  );
}

/** A native select, dressed as an AppKit pop-up button. */
export function Select({
  value,
  onChange,
  label,
  children,
  wide = false,
}: {
  value: string;
  onChange: (value: string) => void;
  /** Accessible name, for a select with no visible label of its own. */
  label?: string;
  children: ReactNode;
  /** Fill the available width rather than sizing to the content. */
  wide?: boolean;
}) {
  return (
    <span className={`relative inline-flex ${wide ? "w-full" : "max-w-full"}`}>
      <select
        value={value}
        aria-label={label}
        onChange={(event) => onChange(event.target.value)}
        className={`appearance-none truncate py-[5px] pr-8 pl-2.5 text-footnote ${wide ? "w-full" : "max-w-[320px]"}`}
        style={fieldStyle}
      >
        {children}
      </select>
      <span
        aria-hidden
        className="pointer-events-none absolute top-1/2 right-1.5 flex h-[18px] w-[18px] -translate-y-1/2 items-center justify-center"
        style={{
          borderRadius: 5,
          background: "var(--color-accent)",
          color: "#ffffff",
        }}
      >
        <svg width="8" height="10" viewBox="0 0 8 10" fill="none" aria-hidden>
          <path
            d="M1.5 3.5 4 1l2.5 2.5M1.5 6.5 4 9l2.5-2.5"
            stroke="currentColor"
            strokeWidth="1.4"
            strokeLinecap="round"
            strokeLinejoin="round"
          />
        </svg>
      </span>
    </span>
  );
}

/** A single-line text field. */
export function TextField({
  value,
  onChange,
  placeholder,
  type = "text",
  label,
  onEnter,
  onBlur,
  disabled = false,
  secret = false,
}: {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  type?: "text" | "url" | "password" | "search";
  label?: string;
  onEnter?: () => void;
  onBlur?: () => void;
  disabled?: boolean;
  /** Never offer autocomplete or spellcheck: the value is a credential. */
  secret?: boolean;
}) {
  return (
    <input
      type={type}
      value={value}
      aria-label={label}
      placeholder={placeholder}
      disabled={disabled}
      onChange={(event) => onChange(event.target.value)}
      onBlur={onBlur}
      onKeyDown={(event) => {
        if (event.key === "Enter") onEnter?.();
      }}
      autoComplete={secret ? "off" : undefined}
      spellCheck={secret ? false : undefined}
      className="w-full px-2.5 py-[6px] text-footnote"
      style={{ ...fieldStyle, opacity: disabled ? 0.5 : 1 }}
    />
  );
}

/** The shared look of every field. */
export const fieldStyle: CSSProperties = {
  background: "var(--surface-elevated)",
  color: "var(--label-primary)",
  border: "none",
  boxShadow: "0 0.5px 1px rgb(0 0 0 / 0.06), 0 0 0 0.5px var(--border-control)",
  borderRadius: "var(--radius-control)",
};

/** A small capsule label: a status, a tier, a warning. */
export function Badge({
  children,
  tone = "neutral",
  title,
}: {
  children: ReactNode;
  tone?: "neutral" | "accent" | "success" | "warning" | "danger";
  title?: string;
}) {
  const colour = {
    neutral: "var(--label-secondary)",
    accent: "var(--color-accent)",
    success: "var(--color-success)",
    warning: "var(--color-warning)",
    danger: "var(--color-danger)",
  }[tone];

  return (
    <span
      title={title}
      className="inline-flex shrink-0 items-center px-2 py-[1px] text-caption2 font-semibold whitespace-nowrap"
      style={{
        color: colour,
        background:
          tone === "neutral"
            ? "var(--surface-sunken)"
            : `color-mix(in srgb, ${colour} 14%, transparent)`,
        borderRadius: "var(--radius-chip)",
      }}
    >
      {children}
    </span>
  );
}
