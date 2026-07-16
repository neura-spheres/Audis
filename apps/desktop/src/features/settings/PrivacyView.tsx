import { GroupedList, Row } from "@/components/GroupedList";

/** What Audis does and does not do with your audio. */
export function PrivacyView() {
  return (
    <div className="flex flex-col gap-8">
      <GroupedList
        title="What Audis does today"
        footnote="Audis has no audio capture yet, so nothing is being recorded or sent anywhere."
      >
        <Row
          label="Audio captured"
          value={<span style={{ color: "var(--color-success)" }}>None</span>}
        />
        <Row
          label="Data sent off this PC"
          value={<span style={{ color: "var(--color-success)" }}>None</span>}
        />
        <Row
          label="Analytics"
          description="Audis collects none, and there is no opt-in that would change that today."
          value={<span style={{ color: "var(--color-success)" }}>Off</span>}
        />
        <Row
          label="Crash reporting"
          value={<span style={{ color: "var(--color-success)" }}>Off</span>}
        />
      </GroupedList>

      <GroupedList title="Commitments">
        <Row
          label="Always visible when listening"
          description="There is no hidden recording mode, and Audis will never hide itself from screen recording."
        />
        <Row
          label="Local by default"
          description="Recordings, transcripts, models and logs stay on this PC. Audio leaves only if you pick a cloud engine, and Audis tells you before a session starts."
        />
        <Row label="Recording is your choice" description="Chosen per session, never assumed." />
        <Row
          label="Your data stays yours"
          description="You can delete a session and its files, delete everything, and export your data. A lapsed licence will never lock or delete your recordings."
        />
        <Row
          label="Secrets are never written to logs"
          description="API keys, transcripts, and audio are excluded from logs and diagnostic exports by design."
        />
      </GroupedList>

      <GroupedList
        title="Your responsibility"
        footnote="Recording other people is regulated differently depending on where you and they are. Audis cannot know that, so it is on you."
      >
        <Row
          label="Consent"
          description="You are responsible for obtaining any consent required where you record."
        />
      </GroupedList>
    </div>
  );
}
