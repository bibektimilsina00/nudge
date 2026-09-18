import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

/**
 * Asking for the two permissions Nudge cannot work without.
 *
 * macOS owns the dialog and it cannot be restyled, so the job here is the
 * sentence *before* it: what this lets Nudge do, said plainly, so the system's
 * own alarming wording arrives already explained rather than as an ambush.
 *
 * One at a time, never a checklist. Four rows with four switches reads as a
 * form, and people fill in the easy ones and stop -- which is how somebody ends
 * up with a microphone granted and no way to see the screen.
 *
 * Only the essential two. The microphone is asked for by the key that needs it,
 * at the moment holding that key is the thing somebody just did, and speech
 * recognition is a privacy improvement rather than a capability -- neither
 * belongs in front of somebody who has not used the app yet.
 */

type State = "granted" | "denied" | "unasked";
type Permit = {
  key: string;
  name: string;
  without: string;
  state: State;
  essential: boolean;
};

/** Remembered here rather than in Rust: it is a fact about this person having
 *  seen a screen, not about the machine, and it must survive a restart. */
const SKIPPED = "nudge.permissions.skipped";

export function skippedPermissions(): boolean {
  return localStorage.getItem(SKIPPED) === "yes";
}

export function Allow({ onDone }: { onDone: () => void }) {
  const [permits, setPermits] = useState<Permit[] | null>(null);
  const [asking, setAsking] = useState(false);

  // Polled, because the answer usually arrives from another application. Both
  // of these flip without a restart, so the page can notice and move on by
  // itself instead of telling somebody to relaunch.
  useEffect(() => {
    const read = () => void invoke<Permit[]>("permits").then(setPermits).catch(() => {});
    read();
    const every = setInterval(read, 700);
    return () => clearInterval(every);
  }, []);

  const needed = (permits ?? []).filter((p) => p.essential);
  const next = needed.find((p) => p.state !== "granted");

  // Nothing left to ask for. Said once, not rendered as a state to sit in.
  useEffect(() => {
    if (permits && !next) onDone();
  }, [permits, next, onDone]);

  if (!permits || !next) return null;

  const refused = next.state === "denied";

  const go = () => {
    setAsking(true);
    // A refused permission cannot be asked for again -- macOS records a
    // dismissal as a refusal and only the Settings pane can change it. Offering
    // "Allow" there is a button that does nothing.
    void invoke(refused ? "open_permit" : "ask_permit", { key: next.key })
      .catch(() => {})
      .finally(() => setAsking(false));
  };

  const skip = () => {
    localStorage.setItem(SKIPPED, "yes");
    onDone();
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col items-center px-8 pt-[calc(var(--notch-h)+14px)] pb-5">
      <div className="my-auto flex w-full flex-col items-center text-center">
        <Shield />
        <h1 className="mt-4 text-[19px] leading-tight font-semibold tracking-[-0.01em] text-ink">
          {next.name}
        </h1>
        {/* Its own words for what is lost, turned around into what is gained.
            "Accessibility" is the name of a settings pane, not a reason. */}
        <p className="mt-1.5 max-w-[19rem] text-[12.5px] leading-[1.55] text-ink-2">
          {refused
            ? `${next.without} macOS will not ask twice, so this one has to be switched on by hand.`
            : next.without}
        </p>

        <div className="mt-5 flex w-full max-w-[16rem] flex-col gap-2">
          <button
            onClick={go}
            disabled={asking}
            className={[
              "grid h-9 place-items-center rounded-control bg-blue",
              "text-[12.5px] font-medium text-white",
              "transition-[background-color,transform] duration-150 ease-[cubic-bezier(0.23,1,0.32,1)]",
              "active:scale-[0.985] disabled:opacity-45 hover:bg-blue-hi",
            ].join(" ")}
          >
            {refused ? "Open Settings" : `Allow ${next.name}`}
          </button>
          {/* Skippable on purpose. A dismissal here is permanent, so a hard
              wall costs the retry as well as the grant -- and the first time
              somebody asks for something that needs this, the reason will be
              obvious in a way it is not now. */}
          <button
            onClick={skip}
            className="h-8 text-[11.5px] text-ink-3 transition-colors duration-150 hover:text-ink-2"
          >
            Not now
          </button>
        </div>
      </div>

      <p className="mt-auto pt-4 text-center text-[10.5px] leading-relaxed text-ink-3">
        {needed.filter((p) => p.state === "granted").length} of {needed.length} granted
      </p>
    </div>
  );
}

function Shield() {
  return (
    <svg viewBox="0 0 48 48" className="size-11 text-blue" fill="none" aria-hidden>
      <path
        d="M24 5 39 10.5v12c0 9-6.4 15.6-15 18.3C15.4 38.1 9 31.5 9 22.5v-12Z"
        stroke="currentColor"
        strokeWidth={2.4}
        strokeLinejoin="round"
      />
      <path
        d="m17.5 23.5 4.6 4.6 8.4-9"
        stroke="currentColor"
        strokeWidth={2.4}
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}
