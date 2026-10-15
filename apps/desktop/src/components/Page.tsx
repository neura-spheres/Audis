import type { ReactNode } from "react";

/**
 * A page: a large title that the toolbar takes over as it scrolls away, an
 * optional line saying what the page is for, and the content beneath.
 *
 * Children arrive one after another rather than all at once, which is what
 * makes a page feel placed rather than swapped.
 */
export function Page({
  title,
  subtitle,
  accessory,
  children,
}: {
  title: string;
  subtitle?: ReactNode;
  /** Trailing content beside the title, such as the page's main action. */
  accessory?: ReactNode;
  children: ReactNode;
}) {
  return (
    <div className="flex flex-col gap-7">
      <header className="audis-page-in flex items-end justify-between gap-4">
        <div className="flex min-w-0 flex-col gap-1">
          <h1
            data-page-title
            className="text-title1 font-bold"
            style={{ letterSpacing: "var(--tracking-tighter)" }}
          >
            {title}
          </h1>
          {subtitle ? (
            <p className="text-subheadline" style={{ color: "var(--label-secondary)" }}>
              {subtitle}
            </p>
          ) : null}
        </div>
        {accessory ? <div className="flex shrink-0 items-center gap-2">{accessory}</div> : null}
      </header>

      <div className="audis-stagger flex flex-col gap-7">{children}</div>
    </div>
  );
}

/** A titled block inside a page, for content that is not a list of rows. */
export function Section({
  title,
  accessory,
  footnote,
  id,
  large = false,
  children,
}: {
  title?: string;
  accessory?: ReactNode;
  footnote?: ReactNode;
  id?: string;
  /** A headline rather than a group label. For Home, not for settings panes. */
  large?: boolean;
  children: ReactNode;
}) {
  return (
    <section id={id} className={`flex scroll-mt-16 flex-col ${large ? "gap-2.5" : "gap-1.5"}`}>
      {title || accessory ? (
        <div className="flex items-baseline justify-between gap-3 px-1">
          {title ? (
            large ? (
              <h2 className="text-title3 font-semibold">{title}</h2>
            ) : (
              <h2
                className="px-2 text-footnote font-semibold"
                style={{ color: "var(--label-secondary)" }}
              >
                {title}
              </h2>
            )
          ) : (
            <span />
          )}
          {accessory}
        </div>
      ) : null}
      {children}
      {footnote ? (
        <p className="px-3 text-footnote" style={{ color: "var(--label-secondary)" }}>
          {footnote}
        </p>
      ) : null}
    </section>
  );
}
