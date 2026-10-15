import type { CSSProperties, ReactNode } from "react";

/**
 * The inset grouped list from macOS System Settings: a titled group of rows on
 * one rounded surface, separated by hairlines rather than boxed one by one.
 */

interface GroupedListProps {
  /** Group heading, rendered above the surface in secondary text. */
  title?: string;
  /** Trailing content in the heading line, such as a count or a small action. */
  accessory?: ReactNode;
  /** Explanatory text below the group, for anything the rows cannot say. */
  footnote?: ReactNode;
  /** Anchor, so another page can link straight to this group. */
  id?: string;
  children: ReactNode;
}

export function GroupedList({ title, accessory, footnote, id, children }: GroupedListProps) {
  return (
    <section id={id} className="flex scroll-mt-16 flex-col">
      {title || accessory ? (
        <div className="mb-1.5 flex items-baseline justify-between gap-3 px-3">
          {title ? (
            <h2 className="text-footnote font-semibold" style={{ color: "var(--label-secondary)" }}>
              {title}
            </h2>
          ) : (
            <span />
          )}
          {accessory ? (
            <span className="text-caption1" style={{ color: "var(--label-tertiary)" }}>
              {accessory}
            </span>
          ) : null}
        </div>
      ) : null}

      <div className="overflow-hidden" style={surfaceStyle}>
        {children}
      </div>

      {footnote ? (
        <p className="mt-1.5 px-3 text-footnote" style={{ color: "var(--label-secondary)" }}>
          {footnote}
        </p>
      ) : null}
    </section>
  );
}

/** The rounded content surface every group and card sits on. */
export const surfaceStyle: CSSProperties = {
  background: "var(--surface-content)",
  borderRadius: "var(--radius-card)",
  boxShadow: "var(--shadow-card)",
};

interface RowProps {
  label: ReactNode;
  /** Trailing content: a value, a control, a status. */
  value?: ReactNode;
  /** Secondary line under the label. */
  description?: ReactNode;
  /** Leading icon, usually an {@link IconBadge}. */
  icon?: ReactNode;
  /** Stack the value under the label instead of beside it. Use for long values */
  stacked?: boolean;
  /** Makes the whole row a button with a disclosure chevron. */
  onClick?: () => void;
}

/** One row: a label, an optional description, and trailing content. */
export function Row({ label, value, description, icon, stacked = false, onClick }: RowProps) {
  // The separator starts where the text does, so it lines up under an icon.
  const inset = icon ? "icon" : "text";
  const text = (
    <div className="flex min-w-0 flex-col gap-0.5">
      <span className="text-subheadline" style={{ color: "var(--label-primary)" }}>
        {label}
      </span>
      {description ? (
        <span className="text-footnote" style={{ color: "var(--label-secondary)" }}>
          {description}
        </span>
      ) : null}
    </div>
  );

  if (stacked) {
    return (
      <div className="audis-row flex min-w-0 flex-col gap-2 px-3.5 py-3" data-inset={inset}>
        <div className="flex min-w-0 items-center gap-3">
          {icon}
          {text}
        </div>
        {value ? <div className="min-w-0">{value}</div> : null}
      </div>
    );
  }

  const content = (
    <>
      <div className="flex min-w-0 flex-1 items-center gap-3">
        {icon}
        {text}
      </div>

      {value ? (
        <div
          className="flex min-w-0 shrink-0 items-center justify-end text-subheadline"
          style={{ color: "var(--label-secondary)" }}
        >
          {value}
        </div>
      ) : null}

      {onClick ? <Chevron /> : null}
    </>
  );

  if (onClick) {
    return (
      <button
        type="button"
        onClick={onClick}
        data-inset={inset}
        className="audis-row flex min-h-[46px] w-full items-center justify-between gap-4 px-3.5 py-2.5 text-left transition-colors"
        style={{ transitionDuration: "var(--duration-fast)" }}
        onMouseEnter={(event) => (event.currentTarget.style.background = "var(--surface-hover)")}
        onMouseLeave={(event) => (event.currentTarget.style.background = "transparent")}
      >
        {content}
      </button>
    );
  }

  return (
    <div
      className="audis-row flex min-h-[46px] items-center justify-between gap-4 px-3.5 py-2.5"
      data-inset={inset}
    >
      {content}
    </div>
  );
}

/** The disclosure chevron at the end of a row that opens something. */
export function Chevron() {
  return (
    <svg
      aria-hidden
      width="7"
      height="12"
      viewBox="0 0 7 12"
      fill="none"
      className="shrink-0"
      style={{ color: "var(--label-tertiary)" }}
    >
      <path
        d="M1 1l5 5-5 5"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}
