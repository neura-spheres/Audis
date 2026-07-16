import React from "react";
import ReactDOM from "react-dom/client";

import { App } from "@/app/App";
import { CaptionWindow } from "@/features/captions/CaptionWindow";
import { ControllerWindow } from "@/features/controller/ControllerWindow";
import { startThemeSync } from "@/stores/theme";
import "@/styles/theme.css";

if (import.meta.env.DEV) {
  const { installDevIpcMock } = await import("@/services/devMock");
  installDevIpcMock();
}

startThemeSync();

const container = document.getElementById("root");
if (!container) {
  throw new Error("Audis could not find its root element. index.html is malformed.");
}

const whichWindow = new URLSearchParams(window.location.search).get("window");
const isOverlay = whichWindow === "captions" || whichWindow === "controller";

if (isOverlay) {
  document.documentElement.dataset.audisWindow = whichWindow ?? "";
}

function selectRoot() {
  switch (whichWindow) {
    case "captions":
      return <CaptionWindow />;
    case "controller":
      return <ControllerWindow />;
    default:
      return <App />;
  }
}

ReactDOM.createRoot(container).render(<React.StrictMode>{selectRoot()}</React.StrictMode>);
