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
 * Several sections are
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
  home: "h-[276px]",
  agents: "h-[330px]",
  settings: "h-[560px]",
  integrations: "h-[560px]",
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
          "overflow-hidden text-white select-none",
          // Glass when open, solid black when closed.
          //
          // The pill is pretending to be the notch, and the notch is a hole in a
          // screen -- translucent it would read as a smudge on the bezel. Open,
          // it is a panel floating over your desktop, which is exactly what the
          // system's own material is for.
          open ? "glass bg-ink rounded-b-[18px]" : "notch-corner bg-black",
          open ? "" : "shadow-[0_6px_18px_rgba(0,0,0,0.45)]",
          "transition-[width,height] duration-[420ms] ease-[cubic-bezier(0.32,0.72,0,1)]",
          open
            ? `${HEIGHT[view]} w-[520px]`
            : busy
              ? "h-(--notch-h) w-[300px]"
              : "h-(--notch-h) w-[220px]",
        ].join(" ")}
      >
        {busy && !open ? <StatusPill status={status} /> : <Pill open={open} />}

        <div
          className={[
            "flex transition-opacity duration-200",
            HEIGHT[view],
            open ? "opacity-100 delay-150" : "pointer-events-none opacity-0",
          ].join(" ")}
        >
          {/* The rail.
              Tabs across the top put the middle of the header behind the notch,
              which is why the old one had to be 540px wide to keep them clear of
              it. Down the side, nothing is ever behind the notch and the width is
              free to be whatever the content wants. */}
          <nav className="flex w-[54px] shrink-0 flex-col items-center gap-1 border-r border-hair pt-2.5 pb-2">
            <Rail active={view === "home"} label="Home" onClick={() => setView("home")}>
              <Home />
            </Rail>
            <Rail active={view === "agents"} label="Agents" onClick={() => setView("agents")}>
              <Sparkle />
            </Rail>
            <Rail
              active={view === "settings" || view === "integrations"}
              label="Settings"
              onClick={() => setView((v) => (v === "settings" ? "home" : "settings"))}
            >
              <Gear />
            </Rail>

            <span className="flex-1" />

            {/* The companion's perch, at the foot of the rail. It used to sit in a
                row of three buttons on Home, where it read as a setting. Here it
                reads as where the cat lives. */}
            <Perch docked={docked} onToggle={() => dock(!docked)} />
          </nav>

          <div className="flex min-w-0 flex-1 flex-col">
            <div className="min-h-0 flex-1 overflow-y-auto">
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
                <HomeView onIntegrations={() => openIntegrations("home")} />
              )}
            </div>

            {/* The footer.
                Raycast's idea and a good one: the keys live in the chrome rather
                than in a panel you have to navigate to, so they are there while
                you are doing the thing rather than only while you are reading
                about it. It also gives the window a bottom edge, which a floating
                list of sections never had. */}
            <footer className="flex h-[30px] shrink-0 items-center gap-2 border-t border-hair px-3 text-[10px] text-white/35">
              <span className="text-white/50">Hold</span>
              <Key>⌃ control</Key>
              <span>to talk</span>
              <span className="flex-1" />
              <Key>esc</Key>
              <span>stop</span>
            </footer>
          </div>
        </div>
      </div>
    </div>
  );
}

/**
 * Home, as one list.
 *
 * It was a two-column grid with a heading, a big square button and a shortcut
 * table -- three different shapes competing in 238px. A single column of rows
 * reads in one pass, and every row is the same target, which matters more than it
 * sounds when the window is this small.
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

function HomeView({ onIntegrations }: { onIntegrations: () => void }) {
  return (
    <div className="px-2.5 py-2.5">
      <ul className="space-y-px">
        <Item icon="+" label="Add a skill" note="Power-ups that attach to Nudge" />
        <Item icon="◇" label="Browse integrations" note="Connect the apps you use" onClick={onIntegrations} />
        <Item icon="✦" label="What's new" note="Recent changes" />
      </ul>

      <h2 className="mt-3 mb-1 px-1.5 text-[9.5px] font-medium tracking-[0.09em] text-white/30 uppercase">
        Shortcuts
      </h2>
      <dl className="space-y-px">
        {SHORTCUTS.map(([name, keys]) => (
          <div
            key={name}
            className="flex items-center gap-2 rounded-lg px-1.5 py-[5px]"
          >
            <dt className="truncate text-[11px] text-white/55">{name}</dt>
            <span className="h-px flex-1 bg-hair" />
            <dd className="flex shrink-0 gap-1">
              {keys.map((k) => (
                <Key key={k}>{k}</Key>
              ))}
            </dd>
          </div>
        ))}
      </dl>
    </div>
  );
}

/** One row of the home list. */
function Item({
  icon,
  label,
  note,
  onClick,
}: {
  icon: string;
  label: string;
  note: string;
  onClick?: () => void;
}) {
  return (
    <li>
      <button
        onClick={onClick}
        className="flex w-full items-center gap-2.5 rounded-lg px-1.5 py-[7px] text-left transition-colors duration-150 hover:bg-hover"
      >
        <span className="on-glass grid size-[26px] shrink-0 place-items-center rounded-md bg-raised text-[12px] text-white/55">
          {icon}
        </span>
        <span className="min-w-0 flex-1">
          <span className="block truncate text-[11.5px] font-medium">{label}</span>
          <span className="block truncate text-[10px] text-white/35">{note}</span>
        </span>
        <span className="shrink-0 text-[10px] text-white/20">›</span>
      </button>
    </li>
  );
}

/**
 * One stop on the rail.
 *
 * The active mark is a bar on the edge rather than a filled pill. A filled tab
 * says "this is a button that is on"; a bar in the margin says "you are here",
 * which is what a rail is for.
 */
function Rail({
  active,
  label,
  onClick,
  children,
}: {
  active: boolean;
  label: string;
  onClick: () => void;
  children: ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      aria-label={label}
      aria-pressed={active}
      title={label}
      className={[
        "group relative grid size-[34px] place-items-center rounded-[10px]",
        "transition-colors duration-150",
        active ? "bg-raised text-white" : "text-white/35 hover:bg-hover hover:text-white/70",
      ].join(" ")}
    >
      <span
        aria-hidden
        className={[
          "absolute -left-[9px] w-[2px] rounded-full bg-accent",
          "transition-all duration-200 ease-[cubic-bezier(0.23,1,0.32,1)]",
          active ? "h-[15px] opacity-100" : "h-0 opacity-0",
        ].join(" ")}
      />
      {children}
    </button>
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
      aria-label={docked ? "Let Nudge out" : "Call Nudge back"}
      title={docked ? "Let Nudge out" : "Call Nudge back"}
      className="grid size-[34px] place-items-center rounded-[10px] transition-colors duration-150 hover:bg-hover"
    >
      <span
        className={[
          "grid size-[26px] shrink-0 place-items-center overflow-hidden rounded-full",
          "transition-colors duration-300",
          // Empty, the socket still reads as a spot something belongs in.
          docked ? "bg-black/40" : "bg-black/25 inset-ring-1 inset-ring-white/15",
        ].join(" ")}
      >
        <span
          className={[
            "block transition-[transform,opacity] duration-300",
            docked
              ? "scale-[0.46] opacity-100 ease-[cubic-bezier(0.34,1.4,0.44,1)]"
              : "scale-[0.95] opacity-0 ease-[cubic-bezier(0.4,0,1,1)]",
          ].join(" ")}
        >
          <Companion mode="idle" anchored />
        </span>
      </span>
    </button>
  );
}

function Key({ children }: { children: ReactNode }) {
  return (
    <kbd className="on-glass rounded-[4px] bg-raised px-[5px] py-[2px] font-mono text-[9px] leading-none whitespace-nowrap text-white/60">
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
