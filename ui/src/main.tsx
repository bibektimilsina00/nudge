import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./lib/rive";
import AgentCard from "./Agent";
import App from "./App";
import Panel from "./Panel";
import "./index.css";

// One bundle, three windows. The overlay covers the screen and never takes focus,
// the panel is the notch dropdown, and the card floats top-right while an agent
// runs. They share components, not behaviour.
const label = getCurrentWindow().label;

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    {label === "panel" ? <Panel /> : label === "agents" ? <AgentCard /> : <App />}
  </StrictMode>,
);
