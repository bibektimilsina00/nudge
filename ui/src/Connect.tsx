import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

/**
 * What a connection would be *for*, not what it is.
 *
 * `examples` is the whole idea. "Connect GitHub?" is a permission request and
 * people say no to those out of habit; a row of things it would let them ask is
 * an offer, and an offer can be judged. Nobody can weigh access to their issues
 * in the abstract -- everybody can decide whether they want to say "summarise
 * the open PRs".
 */
export type Offer = {
  /** The service, as somebody would name it out loud. */
  name: string;
  /** Monogram for the tile. Shipping other companies' marks is a licensing
   *  question and a coloured initial carries the same recognition at 34px. */
  mark: string;
  tint: string;
  /** Dark text on a light tile. */
  dark?: boolean;
  /** Things they could ask for once it is connected. */
  examples: string[];
};

/**
 * The connect prompt: a bar that hangs from the notch and asks once.
 *
 * Three answers rather than two, and the middle one is the important one. *No*
 * is forever, *Not now* is this week, and without the distinction a prompt gets
 * dismissed permanently by somebody who only meant "not while I am doing this" --
 * so the honest way to build it is to make the cheap answer available.
 */
export default function Connect() {
  const [offer, setOffer] = useState<Offer | null>(null);

  useEffect(() => {
    // Asked, not only listened for.
    //
    // This window exists from startup and is merely hidden, so the component
    // mounts long before there is anything to show: it asks once, correctly gets
    // nothing, and then has to hear about it. Hearing alone was not enough --
    // the event is emitted from a background task and the subscription is a
    // promise, so there is a window in which nobody is listening yet.
    //
    // Being shown is the reliable signal. A hidden window's document is hidden,
    // so `visibilitychange` fires exactly when this becomes worth asking again.
    const ask = () => void invoke<Offer | null>("pending_offer").then(setOffer).catch(() => {});
    ask();
    const sub = listen<Offer>("offer", (e) => setOffer(e.payload));
    const woke = () => document.visibilityState === "visible" && ask();
    document.addEventListener("visibilitychange", woke);
    window.addEventListener("focus", woke);
    return () => {
      void sub.then((un) => un());
      document.removeEventListener("visibilitychange", woke);
      window.removeEventListener("focus", woke);
    };
  }, []);

  // Answering plays the exit before anything else happens.
  //
  // The window is hidden by Rust, and hiding it is instant -- so telling Rust
  // first means the bar vanishes and the animation plays to nobody. It leaves
  // first, then says so.
  const [leaving, setLeaving] = useState(false);
  const answer = (said: "yes" | "later" | "no") => {
    if (!offer || leaving) return;
    setLeaving(true);
    const name = offer.name;
    window.setTimeout(() => {
      void invoke("answer_offer", { service: name, said }).catch(() => {});
      setOffer(null);
      setLeaving(false);
      // Matches `--animate-into-notch`. A number in two places, and the wrong
      // half to leave to chance: too short and it is cut off, too long and the
      // bar sits there finished, waiting.
    }, 220);
  };

  // Escape is no different from Not now: the key people press to make something
  // go away should never be the answer that makes it never come back.
  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape") answer("later");
    };
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  });

  if (!offer) return null;

  return (
    // Square at the top, rounded at the bottom: it hangs off the edge of the
    // screen rather than floating in the middle of it, and a thing attached to an
    // edge does not have corners on that edge.
    //
    // Keyed on the service so a second offer arriving into a window that is
    // already open remounts and drops in again. Without it the first one animates
    // and every one after simply changes its text, which reads as the same bar
    // rewording itself rather than as a new thing being asked.
    <div
      key={offer.name}
      // Origin at the top centre, which is where the notch is: it grows out of
      // that rectangle and shrinks back into it.
      className={[
        "w-full origin-top rounded-b-[26px] px-5 py-4 text-white backdrop-blur-2xl",
        "bg-[#0c0c0e]/95 inset-ring-1 inset-ring-white/[0.08]",
        leaving ? "motion-safe:animate-into-notch" : "motion-safe:animate-from-notch",
      ].join(" ")}
      style={{
        // A wash of the service's own colour, off to one side and very faint.
        // It ties the bar to the thing it is asking about without printing a
        // logo, and at four percent it is a warmth rather than a colour.
        backgroundImage:
          `radial-gradient(120% 140% at 88% 0%, ${offer.tint}14 0%, transparent 60%),` +
          " radial-gradient(90% 120% at 10% 0%, rgba(110,120,255,0.10) 0%, transparent 55%)",
      }}
    >
      <div className="flex items-center gap-3">
        <Marks offer={offer} />

        <div className="min-w-0 flex-1">
          <h1 className="truncate text-[13.5px] font-semibold tracking-tight">
            Connect {offer.name} to Nudge
          </h1>
          <p className="mt-[2px] text-[11px] text-white/40">Use Nudge to:</p>
        </div>

        <div className="flex shrink-0 items-center gap-2">
          <Choice onClick={() => answer("no")} label="No">
            <path d="M4 4l8 8M12 4l-8 8" />
          </Choice>
          <Choice onClick={() => answer("later")} label="Not now">
            <circle cx="8" cy="8" r="5.5" />
            <path d="M8 5v3.2l2 1.2" />
          </Choice>
          <Choice onClick={() => answer("yes")} label="Yes" primary>
            <path d="M6.6 9.4a2.8 2.8 0 0 0 4 0l2-2a2.8 2.8 0 1 0-4-4l-.6.6" />
            <path d="M9.4 6.6a2.8 2.8 0 0 0-4 0l-2 2a2.8 2.8 0 1 0 4 4l.6-.6" />
          </Choice>
        </div>
      </div>

      <Examples examples={offer.examples} />
    </div>
  );
}

/**
 * Nudge's face and the service's, overlapping.
 *
 * Two marks rather than one, because the sentence is about a relationship: this
 * thing and that thing, joined. One icon would be a notification from whichever
 * one it showed.
 */
function Marks({ offer }: { offer: Offer }) {
  return (
    <div className="flex shrink-0 items-center">
      <span className="grid size-[36px] place-items-center rounded-[10px] bg-gradient-to-br from-[#6b5bff] to-[#3f8cff] text-[14px] font-bold">
        N
      </span>
      <span
        className="-ml-2 grid size-[36px] place-items-center rounded-[10px] text-[13px] font-bold ring-[2.5px] ring-[#0c0c0e]"
        style={{ backgroundColor: offer.tint, color: offer.dark ? "#111" : "#fff" }}
      >
        {offer.mark}
      </span>
    </div>
  );
}

/**
 * The row of things it would be for, drifting past.
 *
 * There are more of these than fit, and a row that overflows silently is a row
 * whose far half nobody reads. So it moves -- slowly, one pass every few seconds
 * per item, which is reading speed rather than attention-seeking speed.
 *
 * Seamless by holding the list twice and sliding exactly half its width: at the
 * moment the first copy leaves, the second is where it started, so there is no
 * point at which it jumps back. The duration scales with the number of items so
 * that the *speed* is constant -- a fixed duration would make a seven-item row
 * crawl and a nine-item row race.
 *
 * It stops when pointed at, because the one thing somebody does with a moving
 * list is try to read one item of it.
 */
function Examples({ examples }: { examples: string[] }) {
  if (examples.length === 0) return null;
  // Slow. Each item takes about three and a half seconds to cross, which is
  // long enough to read one without waiting for it.
  const seconds = examples.length * 3.5;

  return (
    <div className="group relative mt-3 overflow-hidden">
      <div
        className="flex w-max gap-2 motion-safe:animate-drift group-hover:[animation-play-state:paused]"
        style={{ animationDuration: `${seconds}s` }}
      >
        {/* Twice, and the copy is hidden from anything reading the page aloud --
            it is the same list, present only so the loop has somewhere to go. */}
        {examples.map((e) => (
          <Chip key={e} text={e} />
        ))}
        {examples.map((e) => (
          <Chip key={`again-${e}`} text={e} aria-hidden />
        ))}
      </div>

      {/* Both edges, always. With the row always moving there is always
          something arriving on one side and leaving on the other, so a fade that
          appeared and disappeared would be its own distraction. */}
      <span
        aria-hidden
        className="pointer-events-none absolute inset-y-0 left-0 w-12 bg-gradient-to-r from-[#0c0c0e] to-transparent"
      />
      <span
        aria-hidden
        className="pointer-events-none absolute inset-y-0 right-0 w-12 bg-gradient-to-l from-[#0c0c0e] to-transparent"
      />
    </div>
  );
}

function Chip({ text, ...rest }: { text: string } & React.HTMLAttributes<HTMLSpanElement>) {
  return (
    <span
      {...rest}
      className="shrink-0 rounded-full bg-white/[0.06] px-3 py-[6px] text-[11.5px] whitespace-nowrap text-white/70 inset-ring-1 inset-ring-white/[0.06]"
    >
      {text}
    </span>
  );
}

/** One of the three answers. Only *Yes* is filled; the other two are equals. */
function Choice({
  label,
  onClick,
  primary,
  children,
}: {
  label: string;
  onClick: () => void;
  primary?: boolean;
  children: React.ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      className={[
        "flex items-center gap-1.5 rounded-full px-3 py-[6px] text-[11.5px] font-medium transition-colors duration-150 active:scale-[0.97]",
        primary
          ? "bg-[#0a84ff] text-white hover:bg-[#0a7ae8]"
          : "bg-white/[0.07] text-white/85 hover:bg-white/[0.12]",
      ].join(" ")}
    >
      <svg
        viewBox="0 0 16 16"
        className="size-[13px]"
        fill="none"
        stroke="currentColor"
        strokeWidth={1.8}
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        {children}
      </svg>
      {label}
    </button>
  );
}
