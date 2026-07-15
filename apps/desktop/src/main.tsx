import React from "react";
import ReactDOM from "react-dom/client";

import { App } from "@/app/App";
import { CaptionWindow } from "@/features/captions/CaptionWindow";
import { startThemeSync } from "@/stores/theme";
import "@/styles/theme.css";

// Browser-dev only: stand in for the Rust core so the UI renders under vite dev.
if (import.meta.env.DEV) {
  const { installDevIpcMock } = await import("@/services/devMock");
  installDevIpcMock();
}

// Applied before first paint so the window never flashes the wrong appearance.
startThemeSync();

const container = document.getElementById("root");
if (!container) {
  throw new Error("Audis could not find its root element. index.html is malformed.");
}

// Both windows load this bundle; the query parameter picks which one renders.
// A separate entry point would duplicate the theme and IPC setup for no gain.
const isCaptionWindow = new URLSearchParams(window.location.search).get("window") === "captions";

// Marked on the root element rather than styled from the component, because the
// element that needs to stop painting is <body>, which is above React's tree.
// Set before first paint so the overlay never flashes an opaque background.
if (isCaptionWindow) {
  document.documentElement.dataset.audisWindow = "captions";
}

ReactDOM.createRoot(container).render(
  <React.StrictMode>{isCaptionWindow ? <CaptionWindow /> : <App />}</React.StrictMode>,
);
