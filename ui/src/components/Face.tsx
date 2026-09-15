import { useEffect, useRef } from "react";
import { Alignment, Fit, Layout, useRive, useStateMachineInput } from "@rive-app/react-canvas";
import agent from "../assets/agent.riv?url";
import type { AgentState } from "../Agent";

/**
 * The agent, as something that is visibly working rather than a coloured dot.
 *
 * A status pill says RUNNING and stops saying anything else for four minutes. An
 * agent is the one part of Nudge that takes real time, and the whole question
 * someone has while it does is "is this still going?" -- which a pill answers
 * once and an animation answers continuously.
 */
const MACHINE = "State Machine 1";

/**
 * What the file actually offers. Read out of the binary rather than guessed --
 * see `assets/README.md`. Rive returns null for an input it cannot find and says
 * nothing about it, so every one of these is optional here on purpose: a renamed
 * input costs the expression, not the component.
 */
export function Face({
  state,
  step,
  size = 22,
}: {
  state: AgentState["state"];
  step: number;
  /** Pixels. A chip on the tile at 22; the whole point of an empty state at 64. */
  size?: number;
}) {
  const { rive, RiveComponent } = useRive({
    src: agent,
    stateMachines: MACHINE,
    autoplay: true,
    layout: new Layout({ fit: Fit.Contain, alignment: Alignment.Center }),
  });

  const float = useStateMachineInput(rive, MACHINE, "float");
  const recording = useStateMachineInput(rive, MACHINE, "recording");
  const zoom = useStateMachineInput(rive, MACHINE, "zoom");
  const press = useStateMachineInput(rive, MACHINE, "press");

  useEffect(() => {
    // Waiting is the state that wants a person, so it gets the one that looks
    // like listening. Running is work; everything else is over.
    //
    // `zoom` is what actually draws the face at tile size -- tried the other way
    // round, driving `float` while working, and the tile rendered nothing at all:
    // Floating composes the character into a larger scene, so at 46px there is
    // nothing in frame. Whatever "working" should look like, this file does not
    // have a second state that reads as it.
    const busy = state === "running";
    const asking = state === "waiting";
    if (recording) recording.value = asking;
    if (zoom) zoom.value = busy;
    if (float) float.value = !busy && !asking;
  }, [state, float, recording, zoom]);

  // One pulse per turn. The status line changes when a step lands, and this
  // makes that visible from across the room -- the difference between an agent
  // that is thinking and one that has quietly died is otherwise invisible.
  const last = useRef(step);
  useEffect(() => {
    if (step !== last.current) {
      last.current = step;
      press?.fire();
    }
  }, [step, press]);

  return (
    <div style={{ width: size, height: size }} className="shrink-0" aria-hidden>
      <RiveComponent />
    </div>
  );
}
