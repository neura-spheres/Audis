import { NotBuiltYet } from "@/components/NotBuiltYet";

/**
 * The session library.
 *
 * Empty because there is no capture yet, rather than empty because you have not
 * recorded anything. Saying so is the honest difference.
 */
export function SessionsView() {
  return (
    <NotBuiltYet
      summary="Your recorded sessions will live here: searchable transcripts, summaries, speakers, bookmarks and recordings. Audis cannot record yet, so there is nothing to show."
      planned={[
        "Search across every transcript by phrase, speaker, date or tag",
        "Session detail with transcript, summary, speakers and bookmarks",
        "Edit a transcript, rename speakers, and correct mistakes",
        "Jump from any line to that moment in the recording",
        "Export to text, Markdown, JSON, SRT, WebVTT, DOCX and PDF",
        "Re-run recognition on a saved recording with a different engine",
      ]}
    />
  );
}
