import { NotBuiltYet } from "@/components/NotBuiltYet";

/**
 * Sections whose backend does not exist yet.
 *
 * Each states what it will do and what will live there. They are real routes in
 * the product rather than hidden menu items, so the shape of Audis is legible,
 * but nothing here pretends to work.
 */

export function SpeakersView() {
  return (
    <NotBuiltYet
      summary="Audis separates the remote speakers in your computer's audio. Your microphone is already known to be you, so it does not need guessing."
      planned={[
        "Turn speaker separation on or off",
        "Expected number of speakers",
        "Provisional labels during a session, reconciled when it ends",
        "Rename, merge and split speakers",
        "Optional saved voice profiles, with deletion and export",
      ]}
    >
      <p className="px-3 text-footnote" style={{ color: "var(--label-secondary)" }}>
        Audis will never infer identity, age, gender or ethnicity from a voice, and real-time labels
        are always shown as provisional because they can change as more audio arrives.
      </p>
    </NotBuiltYet>
  );
}

export function AssistantView() {
  return (
    <NotBuiltYet
      summary="An opt-in assistant that can summarise the conversation, track decisions and action items, and answer questions you ask it."
      planned={[
        "Off, manual, question assist, or meeting copilot",
        "Choose a provider and model",
        "Response length and style",
        "Cost mode and a per-session budget",
        "Cooldown between automatic answers",
        "How much recent conversation to include",
      ]}
    >
      <p className="px-3 text-footnote" style={{ color: "var(--label-secondary)" }}>
        When the assistant is off, no transcript text is sent anywhere.
      </p>
    </NotBuiltYet>
  );
}

export function ShortcutsView() {
  return (
    <NotBuiltYet
      summary="Global shortcuts let you control Audis without leaving the call you are in."
      planned={[
        "Start and stop a session",
        "Pause and resume",
        "Show or hide captions",
        "Ask the assistant",
        "Add a bookmark, decision or action item",
        "Conflict detection before a shortcut is saved",
      ]}
    />
  );
}

export function UpdatesView() {
  return (
    <NotBuiltYet
      summary="Audis will update itself from signed releases, verifying the signature before anything is installed."
      planned={[
        "Stable and beta channels",
        "Automatic checking and downloading",
        "Release notes before you install",
        "Install on exit",
        "Update history",
      ]}
    >
      <p className="px-3 text-footnote" style={{ color: "var(--label-secondary)" }}>
        An update with an invalid signature will be refused, and a normal upgrade never touches your
        sessions or recordings.
      </p>
    </NotBuiltYet>
  );
}
