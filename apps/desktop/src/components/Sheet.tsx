import { useEffect, useRef, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";

import { Button } from "@/components/controls";

/** How long the closing animation runs before the sheet is removed. */
const EXIT_MS = 180;

/**
 * Keep something mounted for a moment after it is asked to go, so it can
 * animate out instead of vanishing.
 */
function usePresence(open: boolean): { mounted: boolean; leaving: boolean } {
  const [mounted, setMounted] = useState(open);
  const [leaving, setLeaving] = useState(false);

  useEffect(() => {
    if (open) {
      setMounted(true);
      setLeaving(false);
      return;
    }
    if (!mounted) return;
    setLeaving(true);
    const timer = setTimeout(() => {
      setMounted(false);
      setLeaving(false);
    }, EXIT_MS);
    return () => clearTimeout(timer);
  }, [open, mounted]);

  return { mounted, leaving };
}

/**
 * A modal sheet: dims the window and asks one question. Escape and a click on
 * the backdrop both mean "no".
 */
export function Sheet({
  open,
  onClose,
  labelledBy,
  children,
}: {
  open: boolean;
  onClose: () => void;
  /** Id of the element that names the dialog. */
  labelledBy: string;
  children: ReactNode;
}) {
  const { mounted, leaving } = usePresence(open);
  const panel = useRef<HTMLDivElement>(null);
  // Held in a ref so a parent passing a fresh arrow every render does not
  // re-run the effect below, which would bounce focus in and out of the sheet.
  const close = useRef(onClose);
  close.current = onClose;

  useEffect(() => {
    // Waits for the panel to exist: on the render that opens the sheet it has
    // not mounted yet.
    if (!open || !mounted) return;
    const previous = document.activeElement as HTMLElement | null;
    // The first button is the safe one, and in a destructive confirmation that
    // is where focus belongs.
    panel.current?.querySelector<HTMLElement>("button")?.focus();

    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        close.current();
        return;
      }
      // Keep Tab inside the sheet while it is up.
      if (event.key === "Tab" && panel.current) {
        const focusable = [
          ...panel.current.querySelectorAll<HTMLElement>("button, input, textarea, select"),
        ].filter((element) => !element.hasAttribute("disabled"));
        const first = focusable[0];
        const last = focusable[focusable.length - 1];
        if (!first || !last) return;
        if (event.shiftKey && document.activeElement === first) {
          event.preventDefault();
          last.focus();
        } else if (!event.shiftKey && document.activeElement === last) {
          event.preventDefault();
          first.focus();
        }
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
      previous?.focus?.();
    };
  }, [open, mounted]);

  if (!mounted) return null;

  return createPortal(
    <div className="fixed inset-0 z-50 flex items-center justify-center p-6">
      <div
        aria-hidden
        onClick={onClose}
        className="absolute inset-0"
        style={{
          background: "var(--surface-backdrop)",
          animation: `${leaving ? "audis-fade-out" : "audis-fade-in"} ${leaving ? EXIT_MS : 260}ms var(--ease-standard) both`,
        }}
      />
      <div
        ref={panel}
        role="alertdialog"
        aria-modal="true"
        aria-labelledby={labelledBy}
        className="relative w-full max-w-[380px] p-5"
        style={{
          background: "var(--surface-elevated)",
          borderRadius: 14,
          boxShadow: "var(--shadow-sheet)",
          animation: leaving
            ? `audis-sheet-out ${EXIT_MS}ms var(--ease-standard) both`
            : `audis-sheet-in var(--spring-bouncy-duration) var(--spring-bouncy) both`,
        }}
      >
        {children}
      </div>
    </div>,
    document.body,
  );
}

/** A yes-or-no question about something that cannot be undone. */
export function ConfirmSheet({
  open,
  title,
  message,
  confirmLabel,
  destructive = false,
  onConfirm,
  onCancel,
}: {
  open: boolean;
  title: string;
  message: ReactNode;
  confirmLabel: string;
  destructive?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  return (
    <Sheet open={open} onClose={onCancel} labelledBy="audis-confirm-title">
      <div className="flex flex-col gap-1.5">
        <h2 id="audis-confirm-title" className="text-body font-semibold">
          {title}
        </h2>
        <p className="text-footnote" style={{ color: "var(--label-secondary)" }}>
          {message}
        </p>
      </div>
      <div className="mt-5 flex justify-end gap-2">
        <Button onClick={onCancel}>Cancel</Button>
        <Button onClick={onConfirm} variant={destructive ? "danger" : "accent"}>
          {confirmLabel}
        </Button>
      </div>
    </Sheet>
  );
}
