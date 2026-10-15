import type { ReactNode } from "react";

/**
 * Content that opens and closes by animating its height. Closed content is
 * inert, so it can neither be tabbed into nor read out while hidden.
 */
export function Collapse({ open, children }: { open: boolean; children: ReactNode }) {
  return (
    <div className="audis-collapse" data-open={open} aria-hidden={!open} inert={!open}>
      <div>{children}</div>
    </div>
  );
}
