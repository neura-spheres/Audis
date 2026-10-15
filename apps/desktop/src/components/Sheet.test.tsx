import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";

import { ConfirmSheet } from "./Sheet";

function Harness({ onConfirm }: { onConfirm: () => void }) {
  const [open, setOpen] = useState(false);
  return (
    <>
      <button type="button" onClick={() => setOpen(true)}>
        Delete session
      </button>
      <ConfirmSheet
        open={open}
        title="Delete this session?"
        message="This cannot be undone."
        confirmLabel="Delete"
        destructive
        onConfirm={() => {
          setOpen(false);
          onConfirm();
        }}
        onCancel={() => setOpen(false)}
      />
    </>
  );
}

describe("ConfirmSheet", () => {
  /// A destructive question must start on the safe answer, so Enter on an
  /// accidental open cancels rather than deletes.
  it("puts focus on Cancel, and Escape answers no", async () => {
    const confirmed = vi.fn();
    render(<Harness onConfirm={confirmed} />);

    await userEvent.click(screen.getByRole("button", { name: "Delete session" }));
    const dialog = await screen.findByRole("alertdialog", { name: "Delete this session?" });
    await waitFor(() => expect(screen.getByRole("button", { name: "Cancel" })).toHaveFocus());

    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(dialog).not.toBeInTheDocument());
    expect(confirmed).not.toHaveBeenCalled();
    // Focus goes back to where it was, not to the top of the page.
    expect(screen.getByRole("button", { name: "Delete session" })).toHaveFocus();
  });

  it("keeps Tab inside the sheet", async () => {
    render(<Harness onConfirm={() => undefined} />);
    await userEvent.click(screen.getByRole("button", { name: "Delete session" }));
    await waitFor(() => expect(screen.getByRole("button", { name: "Cancel" })).toHaveFocus());

    await userEvent.tab();
    expect(screen.getByRole("button", { name: "Delete" })).toHaveFocus();
    await userEvent.tab();
    expect(screen.getByRole("button", { name: "Cancel" })).toHaveFocus();
  });

  it("runs the action only when confirmed", async () => {
    const confirmed = vi.fn();
    render(<Harness onConfirm={confirmed} />);
    await userEvent.click(screen.getByRole("button", { name: "Delete session" }));
    await screen.findByRole("alertdialog");

    await userEvent.click(screen.getByRole("button", { name: "Delete" }));
    expect(confirmed).toHaveBeenCalledOnce();
  });
});
