import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";

/** One entry in an {@link OverlayMenu}. */
export interface OverlayMenuItem {
  /** Stable key. */
  id: string;
  /** What the item says. */
  label: string;
  /** Optional leading glyph. */
  icon?: ReactNode;
  /** What it does. The menu closes after this runs. */
  onSelect: () => void;
  /** Render in the destructive tint, for stop/close. */
  danger?: boolean;
  /** A hairline above this item, to group actions. */
  separatorBefore?: boolean;
}

/** A right-click menu for the floating overlays. */
export function OverlayMenu({
  x,
  y,
  items,
  onClose,
}: {
  x: number;
  y: number;
  items: OverlayMenuItem[];
  onClose: () => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const [position, setPosition] = useState({ x, y });

  useLayoutEffect(() => {
    const element = ref.current;
    if (!element) return;

    const rect = element.getBoundingClientRect();
    const margin = 8;
    const nextX = Math.min(x, window.innerWidth - rect.width - margin);
    const nextY = Math.min(y, window.innerHeight - rect.height - margin);
    setPosition({ x: Math.max(margin, nextX), y: Math.max(margin, nextY) });
  }, [x, y]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    const onPointerDown = (event: PointerEvent) => {
      if (ref.current && !ref.current.contains(event.target as Node)) onClose();
    };

    window.addEventListener("keydown", onKey);
    window.addEventListener("pointerdown", onPointerDown, true);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("pointerdown", onPointerDown, true);
    };
  }, [onClose]);

  return (
    <div
      ref={ref}
      role="menu"
      className="fixed z-50 flex min-w-[184px] flex-col p-1"
      style={{
        left: position.x,
        top: position.y,
        background: "rgba(38, 38, 42, 0.86)",
        backdropFilter: "blur(24px) saturate(160%)",
        borderRadius: 12,
        border: "0.5px solid rgba(255, 255, 255, 0.12)",
        boxShadow: "0 12px 40px rgba(0, 0, 0, 0.5)",
        animation: "audis-menu-in 90ms ease-out",
      }}
    >
      {items.map((item) => (
        <div key={item.id}>
          {item.separatorBefore ? (
            <div
              aria-hidden
              className="mx-1 my-1 h-px"
              style={{ background: "rgba(255, 255, 255, 0.1)" }}
            />
          ) : null}
          <button
            type="button"
            role="menuitem"
            className="audis-menu-item flex w-full items-center gap-2.5 rounded-[7px] px-2.5 py-[7px] text-left text-footnote"
            style={{ color: item.danger ? "#ff6b6b" : "rgba(255, 255, 255, 0.92)" }}
            onClick={() => {
              item.onSelect();
              onClose();
            }}
          >
            {item.icon ? (
              <span aria-hidden className="flex w-4 shrink-0 justify-center opacity-80">
                {item.icon}
              </span>
            ) : null}
            <span className="flex-1">{item.label}</span>
          </button>
        </div>
      ))}
    </div>
  );
}

/** Hook that wires an element's `contextmenu` event to an {@link OverlayMenu}. */
export function useOverlayMenu(items: OverlayMenuItem[]) {
  const [at, setAt] = useState<{ x: number; y: number }>();

  const onContextMenu = (event: React.MouseEvent) => {
    event.preventDefault();
    setAt({ x: event.clientX, y: event.clientY });
  };

  const menu = at ? (
    <OverlayMenu x={at.x} y={at.y} items={items} onClose={() => setAt(undefined)} />
  ) : null;

  return { menu, onContextMenu, isOpen: at !== undefined };
}
