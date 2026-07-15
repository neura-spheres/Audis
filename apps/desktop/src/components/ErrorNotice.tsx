import { useState } from "react";

import type { UserFacingError } from "@/schemas/ipc";

/**
 * Renders a UserFacingError: what happened, whether the data survived, and one
 * next step. Technical detail stays folded away so an ordinary user never meets
 * a stack trace but support can still get one.
 */
export function ErrorNotice({ error }: { error: UserFacingError }) {
  const [showDetails, setShowDetails] = useState(false);

  return (
    <div
      role="alert"
      className="flex flex-col gap-3 p-4"
      style={{
        background: "var(--surface-content)",
        borderRadius: "var(--radius-card)",
        boxShadow: "var(--shadow-card)",
      }}
    >
      <div className="flex flex-col gap-1.5">
        <h2 className="text-body font-semibold">{error.title}</h2>
        <p className="text-subheadline" style={{ color: "var(--label-secondary)" }}>
          {error.explanation}
        </p>
      </div>

      {/* Status carries an icon as well as colour, so it still reads for
          colour-blind users. */}
      <p
        className="flex items-center gap-1.5 text-footnote"
        style={{ color: error.dataPreserved ? "var(--color-success)" : "var(--color-danger)" }}
      >
        <span aria-hidden>{error.dataPreserved ? "✓" : "!"}</span>
        {error.dataPreserved ? "Your data was not affected." : "Some data may not have been saved."}
      </p>

      <p className="text-subheadline">{error.suggestedAction}</p>

      {error.technicalDetails ? (
        <div className="flex flex-col gap-2">
          <button
            type="button"
            onClick={() => setShowDetails((shown) => !shown)}
            className="self-start text-footnote"
            style={{ color: "var(--color-accent)" }}
            aria-expanded={showDetails}
          >
            {showDetails ? "Hide technical details" : "Show technical details"}
          </button>

          {showDetails ? (
            <pre
              data-selectable
              className="overflow-x-auto p-2.5 text-caption1"
              style={{
                background: "var(--surface-sunken)",
                borderRadius: "var(--radius-control)",
                color: "var(--label-secondary)",
                fontFamily: "var(--font-mono)",
                whiteSpace: "pre-wrap",
              }}
            >
              {error.technicalDetails}
              {"\n"}
              {error.diagnosticCode}
            </pre>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}
