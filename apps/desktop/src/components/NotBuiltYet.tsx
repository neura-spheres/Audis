import type { ReactNode } from "react";

import { ConstructionIcon } from "@/components/icons";

/**
 * Placeholder for a section whose backend does not exist yet.
 *
 * It states plainly that the feature is not built and lists what will live
 * here, rather than showing controls that look real and do nothing. A dead
 * toggle is worse than an empty page: it makes the user doubt the parts that
 * do work.
 */
interface NotBuiltYetProps {
  /** What this section will do, in one sentence. */
  summary: string;
  /** The controls or capabilities planned for this section. */
  planned: readonly string[];
  /** Optional extra note, such as a privacy caveat. */
  children?: ReactNode;
}

export function NotBuiltYet({ summary, planned, children }: NotBuiltYetProps) {
  return (
    <div className="flex flex-col gap-5">
      <div
        className="flex flex-col items-center gap-3 px-6 py-10 text-center"
        style={{
          background: "var(--surface-content)",
          borderRadius: "var(--radius-card)",
          boxShadow: "var(--shadow-card)",
        }}
      >
        <span style={{ color: "var(--label-tertiary)" }}>
          <ConstructionIcon />
        </span>
        <div className="flex max-w-[420px] flex-col gap-1.5">
          <h2 className="text-body font-semibold">Not built yet</h2>
          <p className="text-subheadline" style={{ color: "var(--label-secondary)" }}>
            {summary}
          </p>
        </div>
      </div>

      <section className="flex flex-col">
        <h3
          className="mb-2 px-3 text-footnote font-medium"
          style={{ color: "var(--label-secondary)" }}
        >
          Planned for this section
        </h3>
        <div
          className="overflow-hidden"
          style={{
            background: "var(--surface-content)",
            borderRadius: "var(--radius-card)",
            boxShadow: "var(--shadow-card)",
          }}
        >
          {planned.map((entry) => (
            <div
              key={entry}
              className="flex items-center gap-2.5 px-3 py-2.5 text-subheadline first:border-t-0"
              style={{
                borderTop: "0.5px solid var(--separator)",
                color: "var(--label-secondary)",
              }}
            >
              <span
                aria-hidden
                className="h-1 w-1 shrink-0 rounded-full"
                style={{ background: "var(--label-tertiary)" }}
              />
              {entry}
            </div>
          ))}
        </div>
      </section>

      {children}
    </div>
  );
}
