import { useEffect, useState, type ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Companion } from "./components/Companion";
import { Agents } from "./panel/Agents";
import { StatusPill, type Status } from "./panel/Status";
import { Integrations } from "./panel/Integrations";
import { Settings } from "./panel/Settings";

/**
 * The notch dock.
 *
 * Laid out to match HeyClicky's panel closely, on request. Several sections are
 * deliberately inert -- Agents, Upgrade, Skills, Integrations -- because the
 * features do not exist yet; they are here as placeholders so the shape is settled
 * before the behaviour arrives. Anything wired up is marked below; everything else
 * is a button that does nothing on purpose.
 *
 * The window never resizes: it is permanently the open size, transparent, and
 * click-through until the pointer arrives, so opening is one CSS transition rather
 * than a sequence of window resizes, which cannot be animated and tear on a Retina
 * display. Only the bottom corners are rounded, because the illusion is that the
 * notch got wider.
 */
const SHORTCUTS: [string, string[]][] = [
  ["Talk", ["⌃ control", "⇧ shift", "space"]],
  ["Next step", ["tap", "⌃⇧ space"]],
  ["Type", ["tap", "then type"]],
  ["Stop", ["esc"]],
];

/**
 * Each view gets the height it needs; the pill is a separate case.
 *
 * Width is shared and deliberately generous: the panel is centred on the notch, so
 * the middle of its top row is *behind* the notch. The header has to be wide enough
 * that the tabs sit outside that band -- at 460px the Agents tab ended exactly at
 * the notch's left edge and disappeared into it.
 */
const HEIGHT = {
  home: "h-[238px]",
  agents: "h-[320px]",
  settings: "h-[640px]",
  integrations: "h-[640px]",
} as const;

export default function Panel() {
  const [open, setOpen] = useState(false);
  // Three places to be, so a name rather than a pile of booleans that can all be
  // true at once.
  const [view, setView] = useState<"home" | "agents" | "settings" | "integrations">("home");
  // Integrations opens from two places, so "back" has to mean the one you left
  // rather than a fixed destination -- entering from Home and landing in Settings
  // is the kind of small wrongness that makes a panel feel untrustworthy.
  const [cameFrom, setCameFrom] = useState<"home" | "settings">("home");
  const [status, setStatus] = useState<Status>("idle");

  const openIntegrations = (from: "home" | "settings") => {
    setCameFrom(from);
    setView("integrations");
  };
  const [docked, setDocked] = useState(false);

  const dock = (next: boolean) => {
    setDocked(next);
    void invoke("set_docked", { docked: next });
  };

  useEffect(() => {
    void invoke<boolean>("docked").then(setDocked);
    const subs = [
      listen<boolean>("notch", (e) => setOpen(e.payload)),
      listen<boolean>("docked", (e) => setDocked(e.payload)),
      listen<Status>("status", (e) => setStatus(e.payload)),
    ];
    return () => subs.forEach((s) => void s.then((un) => un()));
  }, []);

  // Closing should not leave the panel parked three screens deep.
  useEffect(() => {
    if (!open) setView("home");
  }, [open]);

  // Keep the hover region the same shape as what is on screen. Held at the
  // tallest view's size, the panel stayed open far below anything visible.
  useEffect(() => {
    const [w, h] = open
      ? [540, view === "home" ? 266 : view === "agents" ? 320 : 640]
      : [248, 33];
    void invoke("set_open_size", { w: w + 30, h: h + 12 });
  }, [open, view]);

  // The docked pill is the notch, so it is sized by the hardware rather than by a
  // number we picked. Measured once -- the notch does not change while we run.
  useEffect(() => {
    void invoke<number>("notch_height").then((h) =>
      document.documentElement.style.setProperty("--notch-h", `${h}px`),
    );
  }, []);

  // Busy takes over the closed pill; the open panel keeps its own header.
  const busy = status !== "idle";

  return (
    <div className="flex w-full justify-center">
      <div
        className={[
          "overflow-hidden bg-black text-white select-none",
          // The busy pill is a strip in the menu bar, not a panel: a big radius
          // on something 34px tall reads as a lozenge stuck to the notch.
          // Matching the notch exactly is what makes the pill read as part of the
          // hardware. The open panel is far bigger than the notch and carries a
          // slightly larger radius, or 12px on a 500px sheet looks like a mistake.
          open ? "rounded-b-[20px]" : "notch-corner",
          // No shadow while open. The panel is black on a dark menu bar, so the
          // drop shadow never read as depth -- it read as a grey smear along the
          // bottom edge. The resting pill keeps a faint one so it separates from
          // the wallpaper behind the menu bar.
          open ? "" : "shadow-[0_6px_18px_rgba(0,0,0,0.45)]",
          // One curve for both dimensions, so it unfolds rather than growing in
          // two directions at slightly different rates.
          "transition-[width,height] duration-[420ms] ease-[cubic-bezier(0.32,0.72,0,1)]",
          open
            ? `${HEIGHT[view]} w-[540px]`
            : busy
              // Both pills are the notch's own height, so the strip reads as the
              // hardware getting wider rather than as a bar hanging below it.
              ? "h-(--notch-h) w-[300px]"
              // At rest it holds only the companion, so it needs to be barely
              // wider than the notch rather than a bar parked across the menu bar.
              : "h-(--notch-h) w-[220px]",
        ].join(" ")}
      >
        {busy && !open ? <StatusPill status={status} /> : <Pill open={open} />}

        <div
          className={[
            "flex flex-col transition-opacity duration-200",
            HEIGHT[view],
            open ? "opacity-100 delay-150" : "pointer-events-none opacity-0",
          ].join(" ")}
        >
          <header className="flex items-center gap-1.5 px-3 pt-2.5">
            <Tab active={view === "home"} icon={<Home />} onClick={() => setView("home")}>
              Home
            </Tab>
            <Tab active={view === "agents"} icon={<Sparkle />} onClick={() => setView("agents")}>
              Agents
            </Tab>
            <span className="flex-1" />
            <button
              // Settings toggles against wherever you were, rather than always
              // dumping you on Home when you leave it.
              onClick={() => setView((v) => (v === "settings" ? "home" : "settings"))}
              aria-label="Settings"
              aria-pressed={view === "settings"}
              // Same height as the tabs beside it, so the header reads as one row
              // rather than a row with something floating in it -- and a 30px
              // target instead of a 15px glyph with padding round it.
              className={[
                "grid size-[26px] shrink-0 place-items-center rounded-full",
                "transition-colors duration-150",
                view === "settings"
                  ? "bg-[#2e2e2e] text-white"
                  : "text-white/45 hover:bg-[#1e1e1e] hover:text-white/80",
              ].join(" ")}
            >
              <Gear />
            </button>
          </header>

          {view === "agents" ? (
            <Agents />
          ) : view === "integrations" ? (
            <Integrations onBack={() => setView(cameFrom)} />
          ) : view === "settings" ? (
            <Settings
              docked={docked}
              onDock={dock}
              onIntegrations={() => openIntegrations("settings")}
            />
          ) : (
          <>
          <div className="grid flex-1 grid-cols-[1fr_auto] gap-4 px-3.5 pt-1.5">
            <section>
              <h2 className="text-[13.5px] font-semibold tracking-tight">Add skills</h2>
              <p className="mt-0.5 text-[10.5px] text-white/40">
                Skills give Nudge superpowers
              </p>
              <button className="mt-2.5 grid size-[44px] place-items-center rounded-xl bg-[#272727] text-[20px] font-light text-white/70 transition-colors duration-150 hover:bg-[#303030]">
                +
              </button>
            </section>

            <section className="w-[214px]">
              <h3 className="mb-1.5 text-[10.5px] text-white/45">⌘ Shortcuts</h3>
              <dl className="space-y-[6px]">
                {SHORTCUTS.map(([name, keys]) => (
                  <div key={name} className="flex items-center justify-between gap-2">
                    <dt className="truncate text-[10.5px] text-white/45">{name}</dt>
                    <dd className="flex shrink-0 gap-1">
                      {keys.map((k) => (
                        <Key key={k}>{k}</Key>
                      ))}
                    </dd>
                  </div>
                ))}
              </dl>
            </section>
          </div>

          <div className="px-3.5 pb-3">
            <p className="mb-1.5 text-[10.5px] text-white/40">Integrations</p>
            <div className="flex items-center gap-2">
              {/* The whole field opens the browser, not just the little square --
                  a 26px target inside a 34px row that looks pressable is a
                  needlessly small thing to hit. */}
              <button
                onClick={() => openIntegrations("home")}
                aria-label="Browse integrations"
                className="flex h-[30px] flex-1 items-center rounded-[10px] bg-[#1e1e1e] px-1.5 text-left transition-colors duration-150 hover:bg-[#262626] inset-ring-1 inset-ring-white/[0.09]"
              >
                <span className="grid size-[22px] place-items-center rounded-md bg-white/[0.11] text-[13px] font-light text-white/60">
                  +
                </span>
                <span className="ml-2 text-[11px] text-white/35">Add an integration</span>
              </button>

              <Perch docked={docked} onToggle={() => dock(!docked)} />

              <button className="grid size-[30px] place-items-center rounded-[10px] bg-[#1e1e1e] text-[11px] text-white/45 transition-colors duration-150 hover:text-white/70 inset-ring-1 inset-ring-white/[0.09]">
                i
              </button>
            </div>
          </div>
          </>
          )}
        </div>
      </div>
    </div>
  );
}

/**
 * The companion's perch: a socket it sits in when parked, and leaps out of when
 * released.
 *
 * A button that *is* the thing's home reads better than one labelled "Undock
 * Cursor" -- you can see where it went. The companion cannot literally fly between
 * two windows, so the illusion is scale: releasing it grows and fades it out of the
 * socket, calling it back drops it in from larger with a small overshoot, like
 * something landing.
 */
function Perch({ docked, onToggle }: { docked: boolean; onToggle: () => void }) {
  return (
    <button
      onClick={onToggle}
      aria-pressed={docked}
      className="flex h-[30px] items-center gap-2 rounded-full bg-[#1e1e1e] pr-3 pl-[3px] text-[11.5px] font-medium transition-colors duration-150 hover:bg-[#262626] inset-ring-1 inset-ring-white/[0.09]"
    >
      <span
        className={[
          "grid size-[24px] shrink-0 place-items-center rounded-full overflow-hidden",
          "transition-colors duration-300",
          // Empty, the socket still reads as a spot something belongs in.
          docked ? "bg-black/40" : "bg-black/25 inset-ring-1 inset-ring-white/15",
        ].join(" ")}
      >
        <span
          className={[
            "block transition-[transform,opacity] duration-300",
            docked
              ? "scale-[0.42] opacity-100 ease-[cubic-bezier(0.34,1.4,0.44,1)]"
              : "scale-[0.95] opacity-0 ease-[cubic-bezier(0.4,0,1,1)]",
          ].join(" ")}
        >
          <Companion mode="idle" anchored />
        </span>
      </span>
      {docked ? "Let Nudge out" : "Call Nudge back"}
    </button>
  );
}

/**
 * Collapsed state: what sits in the notch.
 *
 * No label -- the point of living in the notch is reading as part of the hardware,
 * and a word beside the Apple menu reads as an app announcing itself.
 */
function Pill({ open }: { open: boolean }) {
  return (
    <div
      // One height class, not two. Listing `h-[38px]` and `h-0` together lets CSS
      // source order decide the winner rather than the condition -- the pill kept
      // its 38px while open, pushed the panel down, and clipped exactly that much
      // off the bottom.
      className={[
        "flex items-center justify-end pr-3 transition-opacity duration-200",
        open ? "pointer-events-none h-0 opacity-0" : "h-(--notch-h) opacity-100 delay-150",
      ].join(" ")}
    >
      {/* Nudged up and in from the corner: sitting hard against the right edge
          it reads as clipped by the pill rather than resting in it. */}
      <div className="-translate-y-[6px] scale-[0.5]">
        <Companion mode="idle" anchored />
      </div>
    </div>
  );
}

function Tab({
  active = false,
  icon,
  children,
  onClick,
}: {
  active?: boolean;
  icon: ReactNode;
  children: ReactNode;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      className={[
        "flex items-center gap-1.5 rounded-full px-2 py-[3px] text-[11px] transition-colors duration-150",
        active ? "bg-[#2e2e2e] text-white" : "text-white/40 hover:text-white/65",
      ].join(" ")}
    >
      {icon}
      {children}
    </button>
  );
}

/** A keycap: small, monospaced, faintly ringed -- the shape of a key, not a badge. */
function Key({ children }: { children: ReactNode }) {
  return (
    <kbd className="rounded-[5px] bg-[#262626] px-1.5 py-[2.5px] font-mono text-[9px] leading-none whitespace-nowrap text-white/65 inset-ring-1 inset-ring-white/[0.08]">
      {children}
    </kbd>
  );
}

const stroke = {
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 1.6,
  strokeLinecap: "round" as const,
  strokeLinejoin: "round" as const,
};

function Home() {
  return (
    <svg viewBox="0 0 16 16" className="size-3" {...stroke}>
      <path d="M2.5 7 8 2.5 13.5 7v6a.8.8 0 0 1-.8.8H3.3a.8.8 0 0 1-.8-.8Z" />
    </svg>
  );
}

function Sparkle() {
  return (
    <svg viewBox="0 0 16 16" className="size-3" {...stroke}>
      <path d="M8 2.2 9.3 6 13 7.3 9.3 8.6 8 12.4 6.7 8.6 3 7.3 6.7 6Z" />
    </svg>
  );
}

/**
 * A cog, not a sun. The previous icon was a circle with eight radiating lines --
 * which is the brightness glyph, and reads as a display control rather than
 * settings.
 */
function Gear() {
  return (
    <svg viewBox="0 0 24 24" className="size-[15px]" {...stroke} strokeWidth={1.8}>
      <circle cx="12" cy="12" r="3" />
      <path d="M19.4 15a1.6 1.6 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.6 1.6 0 0 0-1.8-.3 1.6 1.6 0 0 0-1 1.5v.2a2 2 0 1 1-4 0v-.1a1.6 1.6 0 0 0-1-1.5 1.6 1.6 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.6 1.6 0 0 0 .3-1.8 1.6 1.6 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.6 1.6 0 0 0 1.5-1 1.6 1.6 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.6 1.6 0 0 0 1.8.3H9a1.6 1.6 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.6 1.6 0 0 0 1 1.5 1.6 1.6 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.6 1.6 0 0 0-.3 1.8V9a1.6 1.6 0 0 0 1.5 1h.2a2 2 0 1 1 0 4h-.1a1.6 1.6 0 0 0-1.5 1Z" />
    </svg>
  );
}
