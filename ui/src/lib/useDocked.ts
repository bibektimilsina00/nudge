import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

/** Whether the companion is parked in the panel rather than out on the screen. */
export function useDocked() {
  const [docked, setDocked] = useState(false);

  useEffect(() => {
    void invoke<boolean>("docked").then(setDocked);
    const sub = listen<boolean>("docked", (e) => setDocked(e.payload));
    return () => void sub.then((un) => un());
  }, []);

  return docked;
}
