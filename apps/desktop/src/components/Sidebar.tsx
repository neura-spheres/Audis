import { useLayoutEffect, useRef, useState, type RefObject } from "react";

import { NAV_GROUPS, searchNav, type NavItem, type ViewId } from "@/app/navigation";
import { IconBadge } from "@/components/IconBadge";
import { SearchIcon, CloseIcon } from "@/components/icons";
import { Wordmark } from "@/components/Wordmark";
import { MiniPlayer } from "@/features/home/MiniPlayer";

/** Source-list navigation, in the manner of macOS System Settings. */
interface SidebarProps {
  activeId: ViewId;
  onSelect: (id: ViewId) => void;
  /** The search field, so a keyboard shortcut can focus it. */
  searchRef: RefObject<HTMLInputElement | null>;
}

export function Sidebar({ activeId, onSelect, searchRef }: SidebarProps) {
  const [query, setQuery] = useState("");
  const searching = query.trim().length > 0;
  const results = searching ? searchNav(query) : [];

  const list = useRef<HTMLDivElement>(null);
  const [pill, setPill] = useState<{ top: number; height: number } | undefined>();

  // The selection highlight is one element that moves, rather than one per
  // item that switches on and off: moving is what reads as "the same thing,
  // now here".
  useLayoutEffect(() => {
    const place = () => {
      const container = list.current;
      const active = container?.querySelector<HTMLElement>('[aria-current="page"]');
      if (!container || !active) {
        setPill(undefined);
        return;
      }
      const top =
        active.getBoundingClientRect().top -
        container.getBoundingClientRect().top +
        container.scrollTop;
      setPill({ top, height: active.offsetHeight });
    };
    place();
    window.addEventListener("resize", place);
    return () => window.removeEventListener("resize", place);
  }, [activeId, query]);

  const choose = (id: ViewId) => {
    onSelect(id);
    setQuery("");
  };

  return (
    <nav
      aria-label="Sections"
      className="relative flex h-full w-[236px] shrink-0 flex-col"
      style={{
        background: "var(--surface-sidebar)",
        boxShadow: "inset -0.5px 0 0 var(--separator)",
        backdropFilter: "blur(30px) saturate(180%)",
      }}
    >
      <div data-tauri-drag-region className="flex h-12 shrink-0 items-center gap-2 px-4">
        <Wordmark size={22} />
        <span
          data-tauri-drag-region
          className="text-subheadline font-semibold"
          style={{ letterSpacing: "var(--tracking-tight)" }}
        >
          Audis
        </span>
      </div>

      <div className="px-3 pb-2">
        <label
          className="flex items-center gap-1.5 px-2"
          style={{
            height: 28,
            background: "var(--surface-active)",
            borderRadius: "var(--radius-control)",
            color: "var(--label-secondary)",
          }}
        >
          <span aria-hidden className="flex h-3.5 w-3.5 shrink-0 [&>svg]:h-full [&>svg]:w-full">
            <SearchIcon />
          </span>
          <input
            ref={searchRef}
            type="search"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Escape") setQuery("");
              if (event.key === "Enter" && results[0]) choose(results[0].id);
            }}
            placeholder="Search"
            aria-label="Search settings and pages"
            className="min-w-0 flex-1 bg-transparent text-footnote outline-none [&::-webkit-search-cancel-button]:hidden"
            style={{ color: "var(--label-primary)" }}
          />
          {searching ? (
            <button
              type="button"
              aria-label="Clear search"
              onClick={() => setQuery("")}
              className="flex h-4 w-4 shrink-0 items-center justify-center rounded-full [&>svg]:h-2.5 [&>svg]:w-2.5"
              style={{ background: "var(--label-tertiary)", color: "var(--surface-content)" }}
            >
              <CloseIcon />
            </button>
          ) : null}
        </label>
      </div>

      <div ref={list} className="relative flex-1 overflow-y-auto px-3 pb-3">
        {pill ? (
          <span
            aria-hidden
            className="pointer-events-none absolute right-3 left-3"
            style={{
              top: 0,
              height: pill.height,
              transform: `translateY(${pill.top}px)`,
              background: "var(--color-accent)",
              borderRadius: "var(--radius-control)",
              transition: `transform var(--spring-snappy-duration) var(--spring-snappy), height var(--spring-snappy-duration) var(--spring-snappy)`,
            }}
          />
        ) : null}

        {searching ? (
          results.length > 0 ? (
            <ul className="relative flex flex-col gap-px">
              {results.map((item) => (
                <li key={item.id}>
                  <SidebarItem item={item} active={item.id === activeId} onSelect={choose} />
                </li>
              ))}
            </ul>
          ) : (
            <p
              className="relative px-2 py-6 text-center text-footnote"
              style={{ color: "var(--label-tertiary)" }}
            >
              No results for “{query.trim()}”
            </p>
          )
        ) : (
          NAV_GROUPS.map((group, index) => (
            <div key={group.title ?? `group-${index}`} className={index > 0 ? "mt-4" : ""}>
              {group.title ? (
                <h2
                  className="relative mb-1 px-2 text-caption1 font-semibold"
                  style={{ color: "var(--label-tertiary)" }}
                >
                  {group.title}
                </h2>
              ) : null}
              <ul className="relative flex flex-col gap-px">
                {group.items.map((item) => (
                  <li key={item.id}>
                    <SidebarItem item={item} active={item.id === activeId} onSelect={choose} />
                  </li>
                ))}
              </ul>
            </div>
          ))
        )}
      </div>

      <MiniPlayer onOpen={() => choose("home")} />
    </nav>
  );
}

function SidebarItem({
  item,
  active,
  onSelect,
}: {
  item: NavItem;
  active: boolean;
  onSelect: (id: ViewId) => void;
}) {
  return (
    <button
      type="button"
      onClick={() => onSelect(item.id)}
      aria-current={active ? "page" : undefined}
      className="audis-side-item relative flex w-full items-center gap-2.5 px-2 py-[5px] text-left text-subheadline"
      style={{
        borderRadius: "var(--radius-control)",
        color: active ? "var(--label-on-accent)" : "var(--label-primary)",
        fontWeight: active ? 500 : 400,
        transition: "color var(--duration-fast) var(--ease-standard)",
      }}
    >
      <IconBadge tint={item.tint} size={22}>
        {item.icon}
      </IconBadge>
      <span className="truncate">{item.label}</span>
    </button>
  );
}
