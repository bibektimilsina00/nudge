import cat from "./assets/cat.riv?url";

/**
 * The characters the companion can wear.
 *
 * **This list is the whole feature.** Adding one is adding a `.riv` beside the
 * others and a line here -- nothing in Rust, nothing in the picker, nothing in
 * the component that draws it. The key is what gets stored, so it has to stay
 * stable once shipped; the file behind it can be redrawn freely.
 *
 * There is one today, and a picker showing one option is worth having anyway: it
 * is the difference between a setting that does not exist and a setting with
 * nothing else in it yet, and only the second one gets a second option.
 *
 * What a new entry needs: an artboard that reads at 24px (the socket in the
 * panel) as well as at full size, and a state machine that idles on its own --
 * the point of carrying a Rive runtime is that the thing is alive when nobody is
 * asking it for anything. See `assets/README.md` for what is inside each file.
 */
export type Look = {
  /** Stored, so it must not change once shipped. */
  key: string;
  name: string;
  about: string;
  /** The bundled `.riv`. */
  src: string;
  /** The state machine to run, which is per-file rather than a convention. */
  machine: string;
};

export const LOOKS: Look[] = [
  {
    key: "cat",
    name: "Cat",
    about: "Idles, blinks, and stretches when it falls behind.",
    src: cat,
    machine: "State Machine 1",
  },
];

export const DEFAULT_LOOK = "cat";

/** The chosen one, or the default if the stored key names art that is gone. */
export function lookUp(key: string): Look {
  return LOOKS.find((l) => l.key === key) ?? LOOKS[0];
}
