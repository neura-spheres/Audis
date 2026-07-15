import { GroupedList, Row } from "@/components/GroupedList";
import { useAppInfo } from "@/hooks/useAppInfo";
import { ErrorNotice } from "@/components/ErrorNotice";
import { Wordmark } from "@/components/Wordmark";

/** About Audis. Every value comes from the Rust core, never a frontend constant. */
export function AboutView() {
  const state = useAppInfo();

  if (state.status === "error") {
    return <ErrorNotice error={state.error} />;
  }

  const info = state.status === "ready" ? state.info : undefined;
  const placeholder = "Unknown";

  return (
    <div className="flex flex-col gap-8">
      <header className="flex flex-col items-center gap-3 pt-6 pb-2">
        <Wordmark size={56} />
        <div className="flex flex-col items-center gap-1">
          <h1
            className="text-title1 font-semibold"
            style={{ letterSpacing: "var(--tracking-tighter)" }}
          >
            {info?.appName ?? "Audis"}
          </h1>
          <p className="text-subheadline" style={{ color: "var(--label-secondary)" }}>
            {info?.tagline ?? placeholder}
          </p>
          <p className="text-footnote" style={{ color: "var(--label-tertiary)" }}>
            Version {info?.version ?? placeholder}
          </p>
        </div>
      </header>

      <GroupedList title="Product">
        <Row label="Publisher" value={info?.publisher ?? placeholder} />
        <Row label="Company" value={info?.company ?? placeholder} />
        <Row label="Bundle identifier" value={info?.bundleId ?? placeholder} />
      </GroupedList>

      <GroupedList
        title="Legal"
        footnote="Audis captures audio only when you ask it to. You are responsible for obtaining any consent required where you record."
      >
        <Row label="Licence" value="Proprietary" />
        <Row label="Copyright" value={`© ${new Date().getFullYear()} Neura Audis`} />
      </GroupedList>
    </div>
  );
}
