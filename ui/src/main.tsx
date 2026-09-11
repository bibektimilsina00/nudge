import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import App from "./App";
import Panel from "./Panel";
import "./index.css";

// One bundle, two windows. The overlay covers the screen and never takes focus;
// the panel is a focusable dropdown. They share components, not behaviour.
const isPanel = getCurrentWindow().label === "panel";

createRoot(document.getElementById("root")!).render(
  <StrictMode>{isPanel ? <Panel /> : <App />}</StrictMode>,
);
