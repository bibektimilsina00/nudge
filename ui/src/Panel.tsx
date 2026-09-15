import { useEffect, useState, type ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Companion } from "./components/Companion";
import { Agents } from "./panel/Agents";
import { StatusPill, type Status } from "./panel/Status";
import { Integrations } from "./panel/Integrations";
import { Skills } from "./panel/Skills";
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

/**
 * Each view gets the height it needs; the pill is a separate case.
 *
 * Width is shared and deliberately generous: the panel is centred on the notch, so
 * the middle of its top row is *behind* the notch. The header has to be wide enough
 * that the tabs sit outside that band -- at 460px the Agents tab ended exactly at
 * the notch's left edge and disappeared into it.
 */
const HEIGHT = {
  // Home holds the work now, so it needs the room the Agents tab used to have --
  // a running agent, then a grid of what ran earlier.
  home: "h-[430px]",
  settings: "h-[640px]",
  integrations: "h-[640px]",
  // The same as the other browsers. A list that grows as folders are added needs
  // the room whether or not it is using it today.
  skills: "h-[640px]",
} as const;

export default function Panel() {
  const [open, setOpen] = useState(false);
  // Three places to be, so a name rather than a pile of booleans that can all be
  // true at once.
  const [view, setView] = useState<"home" | "settings" | "integrations" | "skills">("home");
  // Integrations opens from two places, so "back" has to mean the one you left
  // rather than a fixed destination -- entering from Home and landing in Settings
  // is the kind of small wrongness that makes a panel feel untrustworthy.
  const [cameFrom, setCameFrom] = useState<"home" | "settings">("home");
  const [status, setStatus] = useState<Status>("idle");

  // Both entry points -- the tile on the home page and the row in settings --
  // come back to where they were opened from, which is the only reason `cameFrom`
  // exists.
  const openSkills = (from: "home" | "settings") => {
    setCameFrom(from);
    setView("skills");
  };

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
      ? [540, view === "home" ? 430 : 640]
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
          "overflow-hidden text-ink select-none",
          // Black while it is a pill, glass once it is a panel.
          //
          // The pill has to read as part of the notch -- the same black, the same
          // corners -- and frosted glass stuck to a piece of hardware reads as a
          // sticker on it. Open, it is plainly a window and should be made of
          // what every other macOS window is made of.
          open ? "material" : "bg-black",
          // The busy pill is a strip in the menu bar, not a panel: a big radius
          // on something 34px tall reads as a lozenge stuck to the notch.
          // Matching the notch exactly is what makes the pill read as part of the
          // hardware. The open panel is far bigger than the notch and carries a
          // slightly larger radius, or 12px on a 500px sheet looks like a mistake.
          open ? "rounded-b-window" : "notch-corner",
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
          {/* No tabs.
           *
           * There were two -- Home and Agents -- and they split one question
           * across two places: Home held configuration nobody opens twice, and
           * the thing that actually changes was behind the other tab. The panel
           * gets opened to see what Nudge is doing or what it just did, so that
           * is what it opens onto. Configuration is a gear and a footer, which is
           * the weight it deserves in something used by voice. */}
          <header className="flex items-center gap-2 px-3.5 pt-3 pb-1">
            <h1 className="flex-1 text-[13px] font-semibold tracking-tight">Nudge</h1>
            <span className="text-[10.5px] text-ink-3">hold ⌃ to ask</span>
            <button
              onClick={() => setView((v) => (v === "settings" ? "home" : "settings"))}
              aria-label="Settings"
              aria-pressed={view === "settings"}
              className={[
                "grid size-[24px] shrink-0 place-items-center rounded-full",
                "transition-colors duration-150",
                view === "settings"
                  ? "bg-raise-hi text-ink"
                  : "text-ink-3 hover:bg-raise hover:text-ink",
              ].join(" ")}
            >
              <Gear />
            </button>
          </header>

          {view === "integrations" ? (
            <Integrations onBack={() => setView(cameFrom)} />
          ) : view === "skills" ? (
            <Skills onBack={() => setView(cameFrom)} />
          ) : view === "settings" ? (
            <Settings
              docked={docked}
              onDock={dock}
              onIntegrations={() => openIntegrations("settings")}
              onSkills={() => openSkills("settings")}
            />
          ) : (
            <>
              <Agents />
              {/* The one row of configuration this window still carries.
               *
               * Everything here is a place somebody goes once and forgets, so it
               * is a footer rather than a page: reachable, and never the first
               * thing seen. */}
              <footer className="flex items-center gap-2 border-t border-line px-3 py-2">
                <Quick label="Skills" onClick={() => openSkills("home")}>
                  <path d="M8.8 1.8 3.6 9.1h3.4l-.8 5.1 5.2-7.3H8l.8-5.1Z" />
                </Quick>
                <Quick label="Integrations" onClick={() => openIntegrations("home")}>
                  <path d="M2.5 2.5h4v4h-4zM9.5 2.5h4v4h-4zM2.5 9.5h4v4h-4zM9.5 9.5h4v4h-4z" />
                </Quick>
                <span className="flex-1" />
                <span className="text-[10px] text-ink-3">{"alpha"}</span>
              </footer>
            </>
          )}
        </div>
      </div>
    </div>
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

/**
 * A footer entry. Icon and word, quiet until pointed at.
 *
 * Deliberately not a tab: a tab claims to be one of the places this window is
 * about, and these are places somebody visits once.
 */
function Quick({
  label,
  onClick,
  children,
}: {
  label: string;
  onClick: () => void;
  children: ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      className="flex items-center gap-1.5 rounded-control px-2 py-1 text-[11px] text-ink-2 transition-colors duration-150 hover:bg-raise hover:text-ink"
    >
      <svg viewBox="0 0 16 16" className="size-3" fill="none" stroke="currentColor" strokeWidth={1.5} strokeLinejoin="round">
        {children}
      </svg>
      {label}
    </button>
  );
}


/** A keycap: small, monospaced, faintly ringed -- the shape of a key, not a badge. */

const stroke = {
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 1.6,
  strokeLinecap: "round" as const,
  strokeLinejoin: "round" as const,
};



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
