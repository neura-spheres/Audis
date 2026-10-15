import { useEffect, useState } from "react";

import type { Navigate } from "@/app/navigation";
import { ErrorNotice } from "@/components/ErrorNotice";
import { GroupedList, Row } from "@/components/GroupedList";
import { IconBadge } from "@/components/IconBadge";
import { ChipIcon, KeyIcon } from "@/components/icons";
import { Page, Section } from "@/components/Page";
import { Button } from "@/components/controls";
import { useSession } from "@/hooks/useSession";
import { formatDuration, formatWhen, greeting } from "@/lib/format";
import { AUDIS_EVENTS, subscribe } from "@/services/events";
import { AudisIpcError, listFeatures, listSessions } from "@/services/ipc";
import type { Feature, SessionSummary, UserFacingError } from "@/schemas/ipc";

import { FEATURE_STYLE, featureName } from "./featureStyle";
import { describeBlocker, fixFor } from "./blockers";
import { FeatureTile, type TileAction } from "./FeatureTile";
import { NowListening } from "./NowListening";
import { QuickSettings } from "./QuickSettings";

/**
 * Home: start anything in one click, and see what is running.
 *
 * Replaces the old Quick Start, Dashboard and Features pages, which were three
 * views of the same question: what can Audis do right now, and is it doing it.
 */
export function HomeView({ onNavigate }: { onNavigate: Navigate }) {
  const [features, setFeatures] = useState<Feature[]>();
  const [recent, setRecent] = useState<SessionSummary[]>();
  const [error, setError] = useState<UserFacingError>();
  const [pending, setPending] = useState<Feature["id"]>();
  const { session, starting, error: sessionError, start, stop, setPaused } = useSession();

  useEffect(() => {
    listFeatures()
      .then(setFeatures)
      .catch((cause: unknown) => setError(toUserFacing(cause)));
  }, []);

  // A session that ends leaves a transcript behind, so the recent list is
  // re-read whenever one finishes.
  useEffect(() => {
    const load = () =>
      listSessions()
        .then((sessions) => setRecent(sessions.slice(0, 3)))
        .catch(() => setRecent([]));
    load();
    return subscribe(AUDIS_EVENTS.sessionState, (payload) => {
      const state = (payload as { state?: string } | null)?.state;
      if (state === "completed" || state === "failed") load();
    });
  }, []);

  if (error) return <ErrorNotice error={error} />;

  const blockers = unresolvedBlockers(features ?? []);

  const actionFor = (feature: Feature): TileAction => {
    if (session?.mode === feature.id) return "running";
    if (feature.status === "ready") return "start";
    if (feature.status === "needsSetup") return "setUp";
    return "unavailable";
  };

  const run = (feature: Feature) => {
    switch (actionFor(feature)) {
      case "start":
        setPending(feature.id);
        void start(feature.id).finally(() => setPending(undefined));
        // The session card appears at the top; bring it into view as it does.
        document
          .querySelector("[data-page-title]")
          ?.closest("[data-scroll-root]")
          ?.scrollTo({ top: 0, behavior: "smooth" });
        return;
      case "setUp":
        onNavigate(...fixFor(feature.blocker));
        return;
      case "running":
        document.getElementById("now-listening")?.scrollIntoView({ behavior: "smooth" });
        return;
      case "unavailable":
        return;
    }
  };

  return (
    <Page
      title={greeting()}
      subtitle={
        !session
          ? "What would you like Audis to do?"
          : session.state === "paused"
            ? "Paused. Nothing is being heard until you resume."
            : "Audis is listening. Your captions are on screen."
      }
    >
      {sessionError ? <ErrorNotice error={sessionError} /> : null}

      {session ? (
        <div id="now-listening" className="scroll-mt-16">
          <NowListening
            session={session}
            onPause={(paused) => void setPaused(paused)}
            onStop={() => void stop()}
          />
        </div>
      ) : null}

      {blockers.length > 0 ? (
        <GroupedList
          title="Finish setting up"
          footnote="Each takes a minute. Everything that can run without them already works."
        >
          {blockers.map((blocker) => {
            const fix = fixFor(blocker);
            const copy = describeBlocker(blocker);
            const isModel = fix[0] === "speech";
            return (
              <Row
                key={blocker}
                icon={
                  <IconBadge tint={isModel ? "purple" : "green"} size={30}>
                    {isModel ? <ChipIcon /> : <KeyIcon />}
                  </IconBadge>
                }
                label={copy.title}
                description={copy.detail}
                value={
                  <Button variant="tinted" onClick={() => onNavigate(...fix)}>
                    {copy.action}
                  </Button>
                }
              />
            );
          })}
        </GroupedList>
      ) : null}

      <Section large title={session ? "Features" : "Start"}>
        {features === undefined ? (
          <div className="grid grid-cols-2 gap-3">
            {[0, 1, 2, 3].map((index) => (
              <div
                key={index}
                className="audis-skeleton h-[168px]"
                style={{ borderRadius: "var(--radius-tile)" }}
              />
            ))}
          </div>
        ) : (
          <div className="audis-stagger grid grid-cols-2 gap-3">
            {features.map((feature) => (
              <FeatureTile
                key={feature.id}
                feature={feature}
                action={actionFor(feature)}
                listening={session?.state === "listening"}
                starting={starting && pending === feature.id}
                blockedByOther={
                  (session !== null && session.mode !== feature.id) ||
                  (starting && pending !== feature.id)
                }
                onClick={() => run(feature)}
              />
            ))}
          </div>
        )}
      </Section>

      <Section large title="Quick settings">
        <QuickSettings onNavigate={onNavigate} running={session !== null} />
      </Section>

      {recent && recent.length > 0 ? (
        <GroupedList
          title="Recent"
          accessory={
            <button
              type="button"
              onClick={() => onNavigate("library")}
              className="text-footnote font-medium"
              style={{ color: "var(--color-accent)" }}
            >
              See all
            </button>
          }
        >
          {recent.map((item) => {
            const style = FEATURE_STYLE[item.mode];
            return (
              <Row
                key={item.id}
                icon={
                  <IconBadge tint={style.tint} size={30}>
                    {style.icon}
                  </IconBadge>
                }
                label={featureName(item.mode)}
                description={`${formatWhen(item.startedAt)} · ${formatDuration(item.elapsedMs)} · ${item.segmentCount} lines`}
                onClick={() => onNavigate("library", item.id)}
              />
            );
          })}
        </GroupedList>
      ) : null}
    </Page>
  );
}

/** The distinct reasons features cannot start, each listed once. */
function unresolvedBlockers(features: Feature[]): string[] {
  return [...new Set(features.flatMap((feature) => (feature.blocker ? [feature.blocker] : [])))];
}

function toUserFacing(cause: unknown): UserFacingError {
  if (cause instanceof AudisIpcError) return cause.userFacing;
  return {
    title: "Audis could not check what is available",
    explanation: "The feature list could not be loaded. Nothing was changed.",
    dataPreserved: true,
    suggestedAction: "Try again.",
    technicalDetails: String(cause),
    diagnosticCode: "UNEXPECTED",
  };
}
