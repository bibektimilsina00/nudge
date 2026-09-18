import { Bubble } from "./components/Bubble";
import { Companion } from "./components/Companion";
import { Ink } from "./components/Ink";
import { Ring } from "./components/Ring";
import { useDocked } from "./lib/useDocked";
import { useNudge } from "./lib/useNudge";

export default function App() {
  const { phase, message, point, act, control, span, typing } = useNudge();
  const docked = useDocked();
  const mode =
    phase === "listening" ? "listening" : phase === "thinking" ? "thinking" : "idle";

  return (
    <>
      {/* Drawn while the key is held, and drawn again into the screenshot the
          model is given, so circling a thing and asking about "this" works.
          When to draw comes from the key watcher rather than from `phase`:
          listening lasts until the answer arrives, and the pen comes up with
          the key. */}
      <Ink />
      {/* Parked in the panel means not on the screen -- otherwise there are two. */}
      {!docked && <Companion mode={mode} />}
      {/* The ring says where. The hand that used to press it is gone: the real
          pointer travels there anyway, and a second pointer beside the cat was
          one more thing on screen than the moment needed. */}
      {point && <Ring at={point} act={act} control={control} span={span} />}
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
