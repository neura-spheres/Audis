import React from "react";
import ReactDOM from "react-dom/client";

import { App } from "@/app/App";
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

ReactDOM.createRoot(container).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
