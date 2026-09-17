import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Bubble } from "./components/Bubble";
import { Companion } from "./components/Companion";
import { Pointer } from "./components/Pointer";
import { Ring } from "./components/Ring";
import { useDocked } from "./lib/useDocked";
import { useNudge } from "./lib/useNudge";
import type { Point } from "./lib/nudge";

export default function App() {
  const { phase, message, point, act, typing } = useNudge();
  const docked = useDocked();
  const hand = useHandOnCursor();
  const mode =
    phase === "listening" ? "listening" : phase === "thinking" ? "thinking" : "idle";

  return (
    <>
      {/* Parked in the panel means not on the screen -- otherwise there are two. */}
      {!docked && <Companion mode={mode} />}
      {/* The ring says where; the hand does it. The real pointer goes there and
          comes straight back, so this is the part that is actually watchable. */}
      {point && <Ring at={point} act={act} />}
      {point && <Pointer at={point} act={act} />}
      {/* The same hand, parked on the real cursor, for looking at it. A hand
          that exists for 150ms at a target cannot be judged; this one holds
          still. It yields whenever there is a real one, so debugging never
          shows two. */}
      {hand && !point && <Pointer at={hand} act="click" />}
      {/* No status bubble. The notch already says Listening, Thinking and
          Speaking, and the step itself is spoken aloud -- repeating both at the
          bottom of the screen was two captions for one event.

          Two things still appear, because nothing else carries them: an error
          (an invisible failure is indistinguishable from nothing happening) and
          text you have been asked to type yourself, which the voice cannot
          dictate character by character. */}
      {(phase === "error" || typing) && (
        <Bubble tone={phase === "error" ? "error" : "normal"} typing={typing}>
          {phase === "error" ? message : `Type this${typing ? ":" : ""}`}
        </Bubble>
      )}
    </>
  );
}

/**
 * The cursor's position, but only while the debug switch is on.
 *
 * Two subscriptions rather than one, and the cursor one is the reason: it
 * arrives sixty times a second and would re-render this component at that rate
 * for the whole session. It is only attached while the switch is on, so the
 * cost is paid by whoever asked for it.
 */
function useHandOnCursor(): Point | null {
  const [on, setOn] = useState(false);
  const [at, setAt] = useState<Point | null>(null);

  useEffect(() => {
    void invoke<boolean>("show_hand").then(setOn).catch(() => {});
    const sub = listen<boolean>("show-hand", (e) => setOn(e.payload));
    return () => {
      void sub.then((off) => off());
    };
  }, []);

  useEffect(() => {
    if (!on) {
      setAt(null);
      return;
    }
    const sub = listen<[number, number]>("cursor", (e) =>
      setAt({ x: e.payload[0], y: e.payload[1] }),
    );
    return () => {
      void sub.then((off) => off());
    };
  }, [on]);

  return on ? at : null;
}
