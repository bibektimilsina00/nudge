import { Bubble } from "./components/Bubble";
import { Companion } from "./components/Companion";
import { Pointer } from "./components/Pointer";
import { Ring } from "./components/Ring";
import { useDocked } from "./lib/useDocked";
import { useNudge } from "./lib/useNudge";

export default function App() {
  const { phase, message, point, act, typing } = useNudge();
  const docked = useDocked();
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
