import { AskInput } from "./components/AskInput";
import { Bubble } from "./components/Bubble";
import { Companion } from "./components/Companion";
import { Ring } from "./components/Ring";
import { useDocked } from "./lib/useDocked";
import { useNudge } from "./lib/useNudge";

export default function App() {
  const { phase, message, heard, point, act, typing, submit } = useNudge();
  const docked = useDocked();
  const mode =
    phase === "listening" ? "listening" : phase === "thinking" ? "thinking" : "idle";

  // Asking shrinks the native window to the prompt, so screen-absolute elements
  // would render against the wrong origin. Nothing else belongs on screen anyway.
  if (phase === "asking") return <AskInput onSubmit={submit} />;

  return (
    <>
      {/* Parked in the panel means not on the screen -- otherwise there are two. */}
      {!docked && <Companion mode={mode} />}
      {point && <Ring at={point} act={act} />}
      {(
        message && (
          <Bubble
            tone={phase === "error" ? "error" : phase === "unsure" ? "unsure" : "normal"}
            // Only worth repeating once there is a result to compare it against.
            heard={phase === "idle" || phase === "thinking" ? undefined : heard}
            typing={typing}
          >
            {message}
          </Bubble>
        ))}
    </>
  );
}
