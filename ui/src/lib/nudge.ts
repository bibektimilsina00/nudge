import { invoke } from "@tauri-apps/api/core";

export type Point = { x: number; y: number };

/** Mirrors `provider::Act`. Hover is not a kind of click: inside an open menu a
 *  click closes the menu, so waiting for one would stall forever. */
export type Act = "click" | "doubleClick" | "hover";

/**
 * Mirrors `provider::Step`. Three outcomes, not two: a model that may only point
 * or finish will invent coordinates when shown a screen with no matching control.
 */
export type Step =
  | { kind: "point"; at: Point; say: string; act: Act; control?: string | null }
  | { kind: "done"; say: string }
  | { kind: "unsure"; say: string }
  | { kind: "launch"; app: string; say: string }
  | { kind: "open"; url: string; say: string }
  | { kind: "reply"; say: string }
  | { kind: "type"; text: string; submit: boolean; say: string };

/**
 * What the overlay is doing. A single phase rather than a pile of booleans --
 * these states are mutually exclusive and were a source of flicker when they
 * could overlap.
 */
export type Phase =
  | "idle"
  | "listening"
  | "thinking"
  | "showing"
  | "unsure"
  | "launching"
  | "error";

export const api = {
  start: (goal: string) => invoke<Step | null>("start", { goal }),
  advance: () => invoke<Step | null>("advance"),
  /** `silence: false` lets a sentence finish after the overlay clears itself. */
  cancel: (silence: boolean) => invoke<void>("cancel", { silence }),
  /** Click-through off while we need the keyboard, on the rest of the time. */
};
