import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, type Act, type Phase, type Point, type Step } from "./nudge";

/** Base time a final message stays up before the overlay clears. */
const DONE_MS = 2600;
/** Added per character. A joke needs longer on screen than "done". */
/** Roughly a speaking rate: ~14 characters a second. The bubble should not vanish
 *  while the voice is still on the sentence. */
const READ_MS_PER_CHAR = 68;
const READ_MAX = 15000;
const ERROR_MS = 5000;
/**
 * How close a click has to land to count as "they clicked the thing".
 * Generous on purpose -- people aim at the middle of a button, not at our dot,
 * and the ring is 76px across.
 */
const HIT_RADIUS = 70;
/** How long the pointer must rest on a hover target before moving on. Long enough
 *  to be deliberate, short enough not to feel stuck. */
const DWELL_MS = 550;

export function useNudge() {
  const [phase, setPhase] = useState<Phase>("idle");
  const [message, setMessage] = useState("");
  const [point, setPoint] = useState<Point | null>(null);
  const [act, setAct] = useState<Act>("click");
  // What the thing being pointed at is called, when the system knew. Shown on
  // the ring, so the name is beside the thing rather than in a caption
  // somewhere else on the screen.
  const [control, setControl] = useState<string | null>(null);
  // Its size, when the system knew it. A thing with edges gets a box; a guessed
  // pixel gets a ring.
  const [span, setSpan] = useState<[number, number] | null>(null);
  /** Text to enter, when the step is a typing one. Shown verbatim so it can be
   *  copied by eye in guide mode, where Nudge does not type it for you. */
  const [typing, setTyping] = useState<string | null>(null);
  const timer = useRef<number | undefined>(undefined);
  // Read inside listeners registered once, so these must not be state.
  const target = useRef<{ at: Point; act: Act } | null>(null);
  const dwell = useRef<number | undefined>(undefined);

  const clearTimer = () => window.clearTimeout(timer.current);
  const clearDwell = () => {
    window.clearTimeout(dwell.current);
    dwell.current = undefined;
  };

  /** `silence` false lets the voice finish -- see `commands::cancel`. */
  const dismiss = useCallback((silence = true) => {
    clearTimer();
    clearDwell();
    setPhase("idle");
    setMessage("");
    setPoint(null);
    setControl(null);
    setSpan(null);
    setTyping(null);
    target.current = null;
    void api.cancel(silence);
  }, []);

  const render = useCallback(
    (step: Step | null) => {
      clearTimer();
      if (!step) return dismiss();
      setMessage(step.say);
      clearDwell();
      const at = step.kind === "point" ? step.at : null;
      target.current = at && step.kind === "point" ? { at, act: step.act } : null;
      setPoint(at);
      setAct(step.kind === "point" ? step.act : "click");
      setControl(step.kind === "point" ? (step.control ?? null) : null);
      setSpan(step.kind === "point" ? (step.size ?? null) : null);
      setTyping(step.kind === "type" ? step.text : null);
      setPhase(
        step.kind === "unsure"
          ? "unsure"
          : step.kind === "launch" || step.kind === "open"
            ? "launching"
            : "showing",
      );
      // Nothing to point at means nothing to look at -- let it read, then clear.
      // "unsure" lingers too: the fix is usually to open the right app and retry.
      // Scaled by length, because dismissing a joke mid-sentence is worse than
      // leaving it up a beat too long.
      if (step.kind !== "point") {
        const read = Math.min(READ_MAX, DONE_MS + step.say.length * READ_MS_PER_CHAR);
        // Clearing itself is not the user asking for quiet: let the sentence land.
        timer.current = window.setTimeout(() => dismiss(false), read);
      }
    },
    [dismiss],
  );

  const fail = useCallback(
    (e: unknown) => {
      clearTimer();
      setPhase("error");
      setMessage(String(e));
      timer.current = window.setTimeout(() => dismiss(false), ERROR_MS);
    },
    [dismiss],
  );

  /** The user did the thing the ring was pointing at. */
  const reached = useCallback(() => {
    clearDwell();
    target.current = null;
    setPoint(null);
    setControl(null);
    setSpan(null);
    void run(() => api.advance());
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const run = useCallback(
    async (call: () => Promise<Step | null>) => {
      setPhase("thinking");
      setMessage("Looking…");
      try {
        render(await call());
      } catch (e) {
        fail(e);
      }
    },
    [render, fail],
  );

  useEffect(() => {
    // Note: "cursor" and "level" are deliberately absent here. They arrive at 60Hz
    // and 30Hz, and holding them in React state re-rendered the whole overlay on
    // every frame of every mouse movement. Companion subscribes to them itself and
    // writes the DOM directly.
    const subs = [
      listen("advance", () => void run(() => api.advance())),
      listen("listening", () => {
        clearTimer();
        setPoint(null);
    setControl(null);
    setSpan(null);
        setPhase("listening");
        setMessage("Listening…");
      }),
      // Always show what was heard before acting on it. A voice UI that mishears
      // in silence is worse than no voice UI.
      // The transcript is no longer shown on screen -- the notch covers the
      // state and the step is spoken. Kept as a message so an error can still
      // say what it thought it heard.
      listen<string>("heard", (e) => setMessage(`“${e.payload}”`)),
      listen<Step | null>("step", (e) => render(e.payload)),

      // Nudge posts its own clicks, and they arrive here indistinguishable from
      // yours -- which is how a click advances the sequence without a second
      // code path for "it did it" versus "you did it".
      listen<[number, number]>("click", (e) => {
        const t = target.current;
        // A click on a hover target is the user closing a menu, not progress.
        if (!t || t.act === "hover") return;
        const [x, y] = e.payload;
        if (Math.hypot(x - t.at.x, y - t.at.y) > HIT_RADIUS) return;
        reached();
      }),

      // Hover targets advance on dwell. Inside an open menu there is no click to
      // wait for -- a submenu opens because the pointer rested there, and clicking
      // would dismiss the menu and undo the previous step.
      listen<[number, number]>("cursor", (e) => {
        const t = target.current;
        if (!t || t.act !== "hover") return;
        const [x, y] = e.payload;
        const near = Math.hypot(x - t.at.x, y - t.at.y) <= HIT_RADIUS;
        if (near && dwell.current === undefined) {
          dwell.current = window.setTimeout(reached, DWELL_MS);
        } else if (!near) {
          clearDwell();
        }
      }),
      listen<string>("error", (e) => fail(e.payload)),

      // The window lost focus while it had the keyboard. Give it straight back --
      // an overlay that keeps input it is no longer using locks up the machine.
      listen("dismiss", () => dismiss(true)),
    ];

    // Escape is the one place that should cut the voice off mid-word.
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && dismiss(true);
    window.addEventListener("keydown", onKey);

    return () => {
      subs.forEach((p) => void p.then((un) => un()));
      window.removeEventListener("keydown", onKey);
      clearTimer();
    };
  }, [run, render, fail, dismiss]);

  return { phase, message, point, act, control, span, typing, dismiss };
}
