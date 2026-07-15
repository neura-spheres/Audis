import type { ReactNode } from "react";

/**
 * The inset grouped list from macOS System Settings: a titled group of rows on
 * a raised surface, divided by hairlines. This is the workhorse layout for
 * settings and detail panes, kept as a component so every surface inherits the
 * same rhythm instead of each view inventing its own spacing.
 */

interface GroupedListProps {
  /** Group heading, rendered above the surface in secondary text. */
  title?: string;
  /** Explanatory text below the group, for anything the rows cannot say. */
  footnote?: ReactNode;
  children: ReactNode;
}

export function GroupedList({ title, footnote, children }: GroupedListProps) {
  return (
    <section className="flex flex-col">
      {title ? (
        <h2
          className="mb-2 px-3 text-footnote font-medium"
          style={{ color: "var(--label-secondary)" }}
        >
          {title}
        </h2>
      ) : null}

      <div
        className="overflow-hidden"
        style={{
          background: "var(--surface-content)",
          borderRadius: "var(--radius-card)",
          boxShadow: "var(--shadow-card)",
        }}
      >
        {children}
      </div>

      {footnote ? (
        <p className="mt-2 px-3 text-footnote" style={{ color: "var(--label-secondary)" }}>
          {footnote}
        </p>
      ) : null}
    </section>
  );
}

interface RowProps {
  label: ReactNode;
  /** Trailing content: a value, a control, a status. */
  value?: ReactNode;
  /** Secondary line under the label. */
  description?: ReactNode;
  /**
   * Stack the value under the label instead of beside it. Use for long values
   * such as file paths, which have no sensible truncation point.
   */
  stacked?: boolean;
}

/**
 * One row. Rows draw their own top hairline rather than the parent inserting
 * dividers, so the first row is automatically clean and callers can compose
 * rows conditionally without tracking indices.
 *
 * Both columns carry `min-w-0`. Without it a long unbroken value (a Windows
 * path, say) refuses to shrink and pushes itself outside the card.
 */
export function Row({ label, value, description, stacked = false }: RowProps) {
  if (stacked) {
    return (
      <div
        className="flex min-w-0 flex-col gap-1.5 px-3 py-2.5 first:border-t-0"
        style={{ borderTop: "0.5px solid var(--separator)" }}
      >
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
        {value ? <div className="min-w-0">{value}</div> : null}
      </div>
    );
  }

  return (
    <div
      className="flex min-h-[44px] items-center justify-between gap-4 px-3 py-2.5 first:border-t-0"
      style={{ borderTop: "0.5px solid var(--separator)" }}
    >
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

      {value ? (
        <div
          className="flex min-w-0 shrink-0 items-center justify-end text-subheadline"
          style={{ color: "var(--label-secondary)" }}
        >
          {value}
        </div>
      ) : null}
    </div>
  );
}
