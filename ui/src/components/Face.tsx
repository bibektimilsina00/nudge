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
  hue,
}: {
  state: AgentState["state"];
  step: number;
  /** Pixels. A chip on the tile at 22; the whole point of an empty state at 64. */
  size?: number;
  /**
   * Degrees to rotate the artwork's colour by, or nothing to leave it alone.
   *
   * The purple is painted into `agent.riv` and the file exposes no colour input,
   * so it cannot be set the way a state is. What can be done is turn the whole
   * canvas on the colour wheel, which costs nothing and leaves the eyes alone --
   * they are white, and white has no hue to rotate.
   *
   * A real colour would mean binding one in the Rive editor. This is the version
   * that works with the file as it is.
   */
  hue?: number;
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

  // Working has to look like working, and this file cannot say it.
  //
  // What movement it has is on a pointer listener inside the state machine, so it
  // played when the tile was clicked and at no other time -- an agent that looks
  // alive only while you poke it is worse than one that never moves, because the
  // one time nobody is touching it is the whole time it is working.
  //
  // Firing `press` on a beat was tried first and measured: the ball's size varied
  // by one percent across a full cycle, which is nothing. `zoom` is what draws the
  // face at all, and turning it off to make a pulse makes the face vanish.
  //
  // So it is driven from outside, which is how the companion cat is driven for the
  // same reason -- that file has no inputs whatsoever. A slow breath, not a flash:
  // scale only, no glow and no opacity, because a thing blinking in the corner of
  // a screen all day is the behaviour that got the halo removed.

  return (
    <div
      style={{
        width: size,
        height: size,
        filter: hue ? `hue-rotate(${hue}deg)` : undefined,
      }}
      className={`shrink-0 ${state === "running" ? "motion-safe:animate-breath" : ""}`}
      aria-hidden
    >
      <RiveComponent />
    </div>
  );
}
