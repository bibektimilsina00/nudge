import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

/**
 * What macOS has to be told to let Nudge do, all of it on one page.
 *
 * The system dialog cannot be restyled, so what this adds is the sentence
 * before it and somewhere to see the answer afterwards. Each row says what the
 * permission is for in terms of what Nudge cannot do without it -- the name of
 * a settings pane is not a reason -- and carries its own button, because the
 * state of each one is genuinely independent.
 *
 * A refused permission gets a button to the Settings pane instead of another
 * ask. macOS records a dismissal as a refusal and will not prompt twice, so
 * "Allow" there would be a button that does nothing.
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
 *  seen the page, not about the machine, and it must survive a restart. */
const SKIPPED = "nudge.permissions.skipped";

export function skippedPermissions(): boolean {
  return localStorage.getItem(SKIPPED) === "yes";
}

export function Allow({ onDone }: { onDone: () => void }) {
  const [permits, setPermits] = useState<Permit[] | null>(null);

  // Polled, because the answer usually arrives from another application. Both
  // of the essential two flip without a restart, so the page notices by itself
  // rather than telling somebody to relaunch.
  useEffect(() => {
    const read = () => void invoke<Permit[]>("permits").then(setPermits).catch(() => {});
    read();
    const every = setInterval(read, 700);
    return () => clearInterval(every);
  }, []);

  if (!permits) return null;

  const missing = permits.filter((p) => p.essential && p.state !== "granted").length;

  const done = () => {
    // Remembered either way. Having reached the end of this page once is the
    // fact worth keeping, whether or not everything was granted.
    localStorage.setItem(SKIPPED, "yes");
    onDone();
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col px-5 pt-[calc(var(--notch-h)+12px)] pb-4">
      <h1 className="text-center text-[16px] font-semibold tracking-[-0.01em] text-ink">
        {missing ? "Nudge needs permission" : "All set"}
      </h1>
      <p className="mx-auto mt-1 max-w-[20rem] text-center text-[11.5px] leading-[1.5] text-ink-2">
        Nothing runs in the background. Nudge looks at your screen when you hold
        the key, and not otherwise.
      </p>

      <div className="mt-3.5 min-h-0 flex-1 overflow-y-auto">
        <div className="divide-y divide-line overflow-hidden rounded-card border border-line bg-raise">
          {permits.map((p) => (
            <Row key={p.key} permit={p} />
          ))}
        </div>
      </div>

      <button
        onClick={done}
        className={[
          "mt-3 h-9 w-full shrink-0 rounded-control text-[12.5px] font-medium",
          "transition-[background-color,transform] duration-150 ease-[cubic-bezier(0.23,1,0.32,1)]",
          "active:scale-[0.99]",
          missing
            ? "text-ink-2 hover:bg-raise hover:text-ink"
            : "bg-blue text-white hover:bg-blue-hi",
        ].join(" ")}
      >
        {/* Skippable on purpose. A dismissal cannot be undone, so a hard wall
            costs the retry as well as the grant -- and the first time somebody
            asks for something that needs the screen, the reason will be obvious
            in a way it is not on first launch. */}
        {missing ? "Not now" : "Start using Nudge"}
      </button>
    </div>
  );
}

/** One small action on a row. */
const PILL = [
  "rounded-full bg-blue px-2.5 py-[3px] text-[10.5px] font-semibold text-white",
  "transition-[background-color,transform] duration-150 ease-[cubic-bezier(0.23,1,0.32,1)]",
  "active:scale-[0.97] hover:bg-blue-hi",
].join(" ");

function Row({ permit }: { permit: Permit }) {
  const granted = permit.state === "granted";
  // Refused cannot be asked again -- only the pane can change it now.
  const refused = permit.state === "denied";

  return (
    <div className="flex items-center gap-2.5 px-3 py-2.5">
      <span className={granted ? "text-ink-3" : "text-[#ffd60a]"}>
        <Mark permit={permit.key} />
      </span>

      <span className="min-w-0 flex-1">
        <span className="block text-[12px] font-medium text-ink">{permit.name}</span>
        <span className="mt-px block text-[10.5px] leading-snug text-ink-2">
          {!granted && permit.key === "screen"
            ? "Allow it in System Settings, then restart — macOS only tells Nudge after it starts again."
            : permit.without}
        </span>
      </span>

      {granted ? (
        <span className="flex shrink-0 items-center gap-1.5 text-[10.5px] font-medium text-[#30d158]">
          <span className="size-1.5 rounded-full bg-[#30d158]" />
          Granted
        </span>
      ) : (
        <span className="flex shrink-0 items-center gap-1.5">
          <button
            onClick={() =>
              void invoke(refused ? "open_permit" : "ask_permit", { key: permit.key }).catch(
                () => {},
              )
            }
            className={PILL}
          >
            {refused ? "Settings" : "Grant"}
          </button>
          {/* Screen Recording cannot see its own grant until the process
              restarts, so this row carries the restart beside the switch that
              earns it. Every other permission answers live and needs none. */}
          {permit.key === "screen" && (
            <button
              onClick={() => void invoke("relaunch").catch(() => {})}
              className={PILL}
            >
              Restart
            </button>
          )}
        </span>
      )}
    </div>
  );
}

/** One glyph per permission, so a row is recognisable before it is read. */
function Mark({ permit }: { permit: string }) {
  const common = {
    viewBox: "0 0 16 16",
    className: "size-4",
    fill: "none",
    stroke: "currentColor",
    strokeWidth: 1.7,
    strokeLinecap: "round" as const,
    strokeLinejoin: "round" as const,
    "aria-hidden": true,
  };
  switch (permit) {
    case "screen":
      return (
        <svg {...common}>
          <rect x="1.8" y="3" width="12.4" height="8.4" rx="1.6" />
          <path d="M5.5 14h5" />
        </svg>
      );
    case "microphone":
      return (
        <svg {...common}>
          <rect x="6" y="1.8" width="4" height="7.4" rx="2" />
          <path d="M3.6 7.6a4.4 4.4 0 0 0 8.8 0M8 12v2.2" />
        </svg>
      );
    case "speech":
      return (
        <svg {...common}>
          <path d="M2.4 8a5.6 5.6 0 0 1 11.2 0v3.2a2.4 2.4 0 0 1-2.4 2.4H8" />
          <path d="M2.4 8.8v1.6M4.8 7.2v4.8M11.2 7.2v4.8" />
        </svg>
      );
    default:
      return (
        <svg {...common}>
          <path d="M8 1.6 13.4 3.8v4.4c0 3.3-2.3 5.8-5.4 6.8-3.1-1-5.4-3.5-5.4-6.8V3.8Z" />
          <path d="m5.9 8.2 1.5 1.5 2.7-3" />
        </svg>
      );
  }
}
