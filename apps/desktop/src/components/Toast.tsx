import { useEffect } from "react";
import { create } from "zustand";

import { CheckIcon, CloseIcon } from "@/components/icons";

/** How long a toast stays before it leaves on its own. */
const VISIBLE_MS = 3600;
const EXIT_MS = 200;

interface Toast {
  id: number;
  title: string;
  message?: string;
  tone: "success" | "neutral";
  action?: { label: string; run: () => void };
  leaving: boolean;
}

interface ToastState {
  toasts: Toast[];
  push: (toast: Omit<Toast, "id" | "leaving">) => void;
  dismiss: (id: number) => void;
}

let nextId = 1;

const useToasts = create<ToastState>((set, get) => ({
  toasts: [],
  push: (toast) => {
    const id = nextId++;
    set((state) => ({ toasts: [...state.toasts.slice(-2), { ...toast, id, leaving: false }] }));
  },
  dismiss: (id) => {
    if (!get().toasts.some((toast) => toast.id === id && !toast.leaving)) return;
    set((state) => ({
      toasts: state.toasts.map((toast) => (toast.id === id ? { ...toast, leaving: true } : toast)),
    }));
    setTimeout(
      () => set((state) => ({ toasts: state.toasts.filter((toast) => toast.id !== id) })),
      EXIT_MS,
    );
  },
}));

/**
 * Confirm that something happened, briefly, without taking focus: an export
 * finished, a key was saved. Errors are not toasts; they stay on the page.
 */
export function showToast({
  title,
  message,
  tone = "success",
  action,
}: {
  title: string;
  message?: string | undefined;
  tone?: "success" | "neutral";
  action?: { label: string; run: () => void } | undefined;
}): void {
  useToasts.getState().push({
    title,
    tone,
    ...(message ? { message } : {}),
    ...(action ? { action } : {}),
  });
}

/** Where toasts appear: the top centre of the content pane it is placed in. */
export function ToastHost() {
  const toasts = useToasts((state) => state.toasts);

  return (
    <div
      aria-live="polite"
      className="pointer-events-none absolute top-14 left-1/2 z-40 flex w-[min(420px,calc(100%-32px))] -translate-x-1/2 flex-col items-center gap-2"
    >
      {toasts.map((toast) => (
        <ToastCard key={toast.id} toast={toast} />
      ))}
    </div>
  );
}

function ToastCard({ toast }: { toast: Toast }) {
  const dismiss = useToasts((state) => state.dismiss);

  useEffect(() => {
    const timer = setTimeout(() => dismiss(toast.id), VISIBLE_MS);
    return () => clearTimeout(timer);
  }, [toast.id, dismiss]);

  return (
    <div
      role="status"
      className="pointer-events-auto flex w-full items-center gap-3 py-2.5 pr-2.5 pl-3.5"
      style={{
        background: "var(--surface-elevated)",
        borderRadius: 14,
        boxShadow: "var(--shadow-sheet)",
        animation: toast.leaving
          ? `audis-toast-out ${EXIT_MS}ms var(--ease-standard) both`
          : `audis-toast-in var(--spring-bouncy-duration) var(--spring-bouncy) both`,
      }}
    >
      {toast.tone === "success" ? (
        <span
          aria-hidden
          className="flex h-6 w-6 shrink-0 items-center justify-center rounded-full"
          style={{ background: "var(--color-success)", color: "#ffffff" }}
        >
          <CheckIcon />
        </span>
      ) : null}
      <div className="flex min-w-0 flex-1 flex-col">
        <span className="text-footnote font-semibold">{toast.title}</span>
        {toast.message ? (
          <span className="truncate text-caption1" style={{ color: "var(--label-secondary)" }}>
            {toast.message}
          </span>
        ) : null}
      </div>
      {toast.action ? (
        <button
          type="button"
          onClick={() => {
            toast.action?.run();
            dismiss(toast.id);
          }}
          className="audis-pressable shrink-0 px-2.5 py-1 text-footnote font-semibold"
          style={{ color: "var(--color-accent)", borderRadius: "var(--radius-control)" }}
        >
          {toast.action.label}
        </button>
      ) : null}
      <button
        type="button"
        aria-label="Dismiss"
        onClick={() => dismiss(toast.id)}
        className="flex h-6 w-6 shrink-0 items-center justify-center rounded-full"
        style={{ color: "var(--label-tertiary)" }}
      >
        <CloseIcon />
      </button>
    </div>
  );
}
