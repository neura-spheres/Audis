import { NotBuiltYet } from "@/components/NotBuiltYet";

/** Sections whose backend does not exist yet. */

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
