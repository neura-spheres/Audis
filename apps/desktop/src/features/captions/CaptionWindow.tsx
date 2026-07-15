import { useEffect, useState } from "react";

import { AUDIS_EVENTS, subscribe } from "@/services/events";
import { getSettings } from "@/services/ipc";
import {
  settingsSchema,
  transcriptSegmentSchema,
  type CaptionSettings,
  type TranscriptSegment,
} from "@/schemas/ipc";

/**
 * The caption overlay.
 *
 * Its own transparent, always-on-top window, sitting over whatever the user is
 * actually watching. Everything here serves one goal: the words must be easy to
 * read at a glance, over any background, without becoming the thing you look at.
 *
 * Two kinds of line appear. Finished sentences are permanent and are what gets
 * saved. Above them sits at most one interim line: the sentence being spoken
 * right now, re-decoded a couple of times a second so words appear while
 * someone is still talking rather than a second after they stop. It is replaced
 * in place by the finished sentence and is never written to disk.
 */
export function CaptionWindow() {
  const [lines, setLines] = useState<TranscriptSegment[]>([]);
  /// The sentence being spoken right now, before it is finished and replaced.
  const [partial, setPartial] = useState<TranscriptSegment>();
  const [captions, setCaptions] = useState<CaptionSettings>();

  useEffect(() => {
    const load = () => {
      getSettings()
        .then((settings) => setCaptions(settings.captions))
        .catch(() => undefined);
    };
    load();

    // Settings live in the other window, so changing the font size or opacity
    // must reach this one. Without this the overlay keeps whatever it read at
    // startup and the settings appear to do nothing.
    return subscribe(AUDIS_EVENTS.settingsChanged, (payload) => {
      const parsed = settingsSchema.safeParse(payload);
      if (parsed.success) setCaptions(parsed.data.captions);
    });
  }, []);

  const maxLines = captions?.maxLines ?? 3;

  useEffect(() => {
    const stopTranscript = subscribe(AUDIS_EVENTS.transcriptFinal, (payload) => {
      const parsed = transcriptSegmentSchema.safeParse(payload);
      if (!parsed.success) return;

      setLines((current) => [...current, parsed.data].slice(-maxLines));
      // The finished sentence supersedes the interim guess at it. Clearing here
      // rather than on a timer means the two are never on screen together.
      setPartial(undefined);
    });

    // Interim text, decoded while the sentence is still being spoken. It will
    // be replaced, so it is only ever held in memory and never appended.
    const stopPartial = subscribe(AUDIS_EVENTS.transcriptPartial, (payload) => {
      const parsed = transcriptSegmentSchema.safeParse(payload);
      if (parsed.success) setPartial(parsed.data);
    });

    // A new session starts on a clean screen rather than inheriting the last
    // one's words.
    const stopSession = subscribe(AUDIS_EVENTS.sessionState, () => {
      setLines([]);
      setPartial(undefined);
    });

    return () => {
      stopTranscript();
      stopPartial();
      stopSession();
    };
  }, [maxLines]);

  if (!captions) return null;

  const opacity = captions.backgroundOpacity / 100;
  const hasPanel = opacity > 0.01;

  // The interim line sits with the finished ones and counts against the limit,
  // so the panel never grows by a line as someone starts speaking.
  const visible = [...lines, ...(partial ? [partial] : [])].slice(-maxLines);

  return (
    <div className="flex h-screen w-screen items-end justify-center p-4">
      <div
        className="flex w-full flex-col gap-2 transition-opacity"
        style={{
          maxWidth: "min(100%, 1100px)",
          padding: hasPanel ? "18px 24px" : "4px 8px",
          borderRadius: 18,
          background: hasPanel ? `rgba(14, 15, 17, ${opacity})` : "transparent",
          // Blur lifts the words off busy video rather than relying on the
          // panel being opaque, so a low opacity still reads.
          backdropFilter: hasPanel ? "blur(20px) saturate(140%)" : undefined,
          border: hasPanel ? `1px solid rgba(255, 255, 255, ${0.1 * opacity})` : undefined,
          boxShadow: hasPanel ? "0 8px 40px rgba(0, 0, 0, 0.45)" : undefined,
          // An empty panel floating over someone's video is worse than nothing.
          opacity: visible.length > 0 ? 1 : 0,
          transitionDuration: "180ms",
        }}
      >
        {visible.map((line, index) => (
          <CaptionLine
            // The interim line keeps one identity for its whole life, so React
            // updates its text in place. Keying on `id` would remount it on
            // every re-decode and replay the entrance animation as a flicker.
            key={line.isFinal ? line.id : `interim-${line.source}`}
            line={line}
            settings={captions}
            hasPanel={hasPanel}
            // Older lines recede so the eye lands on the newest one.
            faded={index < visible.length - 1}
          />
        ))}
      </div>
    </div>
  );
}

function CaptionLine({
  line,
  settings,
  hasPanel,
  faded,
}: {
  line: TranscriptSegment;
  settings: CaptionSettings;
  hasPanel: boolean;
  faded: boolean;
}) {
  const size = settings.fontSize;

  return (
    <p
      className="flex items-baseline gap-2.5 leading-[1.35]"
      style={{
        fontSize: size,
        fontWeight: 600,
        color: "#ffffff",
        opacity: faded ? 0.55 : 1,
        // Without a panel the text sits directly on video, so it carries its
        // own contrast: a tight dark halo plus a soft drop shadow stays legible
        // over white and black alike.
        textShadow: hasPanel
          ? "0 1px 2px rgba(0, 0, 0, 0.5)"
          : "0 0 3px rgba(0,0,0,0.95), 0 0 8px rgba(0,0,0,0.8), 0 2px 4px rgba(0,0,0,0.9)",
        // Only finished lines animate in. The interim line changes its own text
        // continuously, and animating that would be motion under the reader's
        // eye rather than a cue that something new arrived.
        animation: line.isFinal ? "audis-caption-in 160ms ease-out" : undefined,
      }}
    >
      {settings.showSourceLabels ? <SourceLabel line={line} size={size} /> : null}
      <span className="min-w-0">{line.text}</span>
    </p>
  );
}

/**
 * Who said it.
 *
 * Deliberately quiet: a dot carries the source and the name sits at roughly
 * half the caption's size in a muted tone. The label is reference information,
 * not the message, and the previous loud red version competed with the words
 * for attention every single line.
 */
function SourceLabel({ line, size }: { line: TranscriptSegment; size: number }) {
  const colour = sourceColour(line.source);

  return (
    <span
      className="flex shrink-0 items-center gap-1.5 whitespace-nowrap"
      style={{
        fontSize: Math.max(11, Math.round(size * 0.42)),
        fontWeight: 600,
        letterSpacing: "0.04em",
        color: "rgba(255, 255, 255, 0.62)",
        textShadow: "0 1px 3px rgba(0, 0, 0, 0.9)",
        transform: "translateY(-0.08em)",
      }}
    >
      <span
        aria-hidden
        style={{
          width: Math.max(5, Math.round(size * 0.16)),
          height: Math.max(5, Math.round(size * 0.16)),
          borderRadius: "50%",
          background: colour,
          boxShadow: `0 0 8px ${colour}`,
        }}
      />
      {line.speaker}
    </span>
  );
}

/**
 * Microphone and computer audio get distinct hues.
 *
 * The label spells out the same thing, so colour reinforces rather than being
 * the only cue: these two are also easy to tell apart with common colour
 * blindness, which red and green would not be.
 */
function sourceColour(source: TranscriptSegment["source"]): string {
  return source === "microphone" ? "#4ade80" : "#60a5fa";
}
