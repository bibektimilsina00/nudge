import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./lib/rive";
import AgentCard from "./Agent";
import App from "./App";
import Connect from "./Connect";
import Panel from "./Panel";
import "./index.css";

// One bundle, four windows. The overlay covers the screen and never takes focus,
// the panel is the notch dropdown, the card floats top-right while an agent runs,
// and the connect bar hangs off the top edge to ask about a service. They share
// components, not behaviour.
const label = getCurrentWindow().label;

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    {label === "panel" ? (
      <Panel />
    ) : label === "agents" ? (
      <AgentCard />
    ) : label === "connect" ? (
      <Connect />
    ) : (
      <App />
    )}
  </StrictMode>,
);
