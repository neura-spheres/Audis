import { NAV_GROUPS, type ViewId } from "@/app/navigation";
import { Wordmark } from "@/components/Wordmark";

/** Source-list navigation, in the manner of Finder and macOS System Settings. */
interface SidebarProps {
  activeId: ViewId;
  onSelect: (id: ViewId) => void;
}

export function Sidebar({ activeId, onSelect }: SidebarProps) {
  return (
    <nav
      aria-label="Sections"
      className="flex h-full w-[228px] shrink-0 flex-col border-r"
      style={{ borderColor: "var(--separator)", background: "var(--surface-window)" }}
    >
      <div data-tauri-drag-region className="flex h-11 shrink-0 items-center px-3.5">
        <Wordmark size={22} />
      </div>

      <div className="flex-1 overflow-y-auto px-2.5 pb-3">
        {NAV_GROUPS.map((group, index) => (
          <div key={group.title ?? `group-${index}`} className={index > 0 ? "mt-4" : "mt-1"}>
            {group.title ? (
              <h2
                className="mb-1 px-2.5 text-caption2 font-semibold uppercase"
                style={{ color: "var(--label-tertiary)", letterSpacing: "0.06em" }}
              >
                {group.title}
              </h2>
            ) : null}

            <ul className="flex flex-col gap-0.5">
              {group.items.map((item) => {
                const isActive = item.id === activeId;
                return (
                  <li key={item.id}>
                    <button
                      type="button"
                      onClick={() => onSelect(item.id)}
                      aria-current={isActive ? "page" : undefined}
                      className="flex w-full items-center gap-2.5 px-2.5 py-[6px] text-left text-subheadline transition-colors"
                      style={{
                        borderRadius: "var(--radius-control)",
                        transitionDuration: "var(--duration-fast)",
                        transitionTimingFunction: "var(--ease-standard)",
                        background: isActive ? "var(--color-accent)" : "transparent",
                        color: isActive ? "var(--label-on-accent)" : "var(--label-primary)",
                        fontWeight: isActive ? 500 : 400,
                      }}
                      onMouseEnter={(event) => {
                        if (!isActive)
                          event.currentTarget.style.background = "var(--surface-hover)";
                      }}
                      onMouseLeave={(event) => {
                        if (!isActive) event.currentTarget.style.background = "transparent";
                      }}
                    >
                      <span
                        aria-hidden
                        className="flex h-4 w-4 shrink-0 items-center justify-center"
                        style={{ opacity: isActive ? 1 : 0.55 }}
                      >
                        {item.icon}
                      </span>
                      <span className="truncate">{item.label}</span>
                    </button>
                  </li>
                );
              })}
            </ul>
          </div>
        ))}
      </div>
    </nav>
  );
}
