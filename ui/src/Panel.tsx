import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Companion } from "./components/Companion";
import { Ask } from "./panel/Ask";
import { Agents } from "./panel/Agents";
import { StatusPill, type Status } from "./panel/Status";
import { Integrations } from "./panel/Integrations";
import { Skills } from "./panel/Skills";
import { Report, type Kind } from "./panel/Report";
import { Settings } from "./panel/Settings";
import { SignIn, type Account } from "./panel/SignIn";
import { Allow, skippedPermissions } from "./panel/Allow";

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
  home: 286,
  agents: 320,
  settings: 640,
  integrations: 640,
  // The same as the other browsers. A list that grows as folders are added needs
  // the room whether or not it is using it today.
  skills: 640,
  report: 400,
  // Taller than Home. The companion is the same size, but underneath it there
  // are two buttons and a line of small print that has to be readable rather
  // than merely present.
  //
  // Not as tall as it was. The block inside centres itself, so extra height
  // does not make the page more generous -- it just widens the gap between the
  // buttons and the small print until the two stop looking related.
  signin: 366,
  // Taller than signing in: four rows, each with a sentence saying what is
  // lost without it, and the list must not be the part that scrolls.
  allow: 430,
} as const;

export default function Panel() {
  const [open, setOpen] = useState(false);
  // Three places to be, so a name rather than a pile of booleans that can all be
  // true at once.
  const [view, setView] = useState<
    "home" | "agents" | "settings" | "integrations" | "skills" | "report"
  >("home");
  const [reporting, setReporting] = useState<Kind>("bug");
  // Integrations opens from two places, so "back" has to mean the one you left
  // rather than a fixed destination -- entering from Home and landing in Settings
  // is the kind of small wrongness that makes a panel feel untrustworthy.
  const [cameFrom, setCameFrom] = useState<"home" | "settings">("home");
  const [status, setStatus] = useState<Status>("idle");
  // Three states, and the third one matters: `undefined` is "not asked yet".
  // Collapsing it into `null` would flash the sign-in page at somebody who is
  // signed in, every single time the app starts.
  const [account, setAccount] = useState<Account | null | undefined>(undefined);
  // Asked once, after signing in, and only while there is something to ask for.
  // Skipping is remembered, so "not now" does not mean "every time".
  const [allowing, setAllowing] = useState(!skippedPermissions());
  // A new version, found about twenty seconds after launch. Kept here rather
  // than in Settings alone, because a notice nobody opens is not a notice.
  const [newer, setNewer] = useState(false);

  useEffect(() => {
    const sub = listen("update", () => setNewer(true));
    return () => {
      void sub.then((off) => off());
    };
  }, []);

  useEffect(() => {
    // The local answer first, because it is instant and is what the first paint
    // needs. Then the server's, which is the only thing that can tell us the
    // session was signed out from somewhere else -- and which is allowed to
    // take as long as the network does, because by then there is already
    // something on the screen.
    void invoke<Account | null>("account")
      .then((a) => setAccount(a ?? null))
      .catch(() => setAccount(null));
    void invoke("check_account").catch(() => {});

    const sub = listen<Account | null>("account", (e) => setAccount(e.payload ?? null));
    return () => {
      void sub.then((off) => off());
    };
  }, []);

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
  // Heading this way, but not here yet. Its own ring, wider than the dock's --
  // see `is_near` in notch.rs for why this cannot be a delay before opening.
  const [near, setNear] = useState(false);
  // What to hold, read rather than hardcoded: the key is configurable, and a
  // hint naming the wrong one is worse than no hint.
  const [hold, setHold] = useState("");

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
      listen<boolean>("near", (e) => setNear(e.payload)),
    ];
    return () => subs.forEach((s) => void s.then((un) => un()));
  }, []);

  useEffect(() => {
    // The glyph only, not "⌃ control" -- the settings page has room to spell it
    // out and a strip in the notch does not.
    void invoke<{ id: string; keys: string[] }[]>("shortcuts")
      .then((all) => setHold(all.find((s) => s.id === "talk")?.keys[0]?.split(" ")[0] ?? ""))
      .catch(() => {});
  }, []);

  // The panel keeps its place when it closes. It used to go back to Home, so as
  // not to be parked three screens deep next time -- but it closes every time you
  // go and do the thing it just asked you to do (approve a sign-in, copy a token,
  // look something up), and starting over on every return is the opposite of
  // helpful. The tabs are one click away if Home is where you wanted.

  // Busy takes over the closed pill; the open panel keeps its own header.
  // Which height to be. Signed out is a shape of its own rather than a view,
  // because it is not somewhere you can navigate to or away from.
  const shape: keyof typeof HEIGHT =
    account === null ? "signin" : account && allowing ? "allow" : view;

  // Keep the hover region the same shape as what is on screen. Held at the
  // tallest view's size, the panel stayed open far below anything visible.
  //
  // From `HEIGHT[shape]` -- the number the panel is actually drawn at -- and not
  // a second list beside it. There was one, keyed on `view` alone, and it did not
  // know about the two shapes that are not views: signing in renders 366 and was
  // reported as 300, permissions renders 430 and was reported as 300. Both put
  // their buttons in the 60 to 130 points below where the pointer still counted
  // as being on the panel, so reaching for one closed the thing it was on.
  //
  // The true size, with no padding added here -- how much room to leave around it
  // is one decision and it lives in notch.rs, next to the reasoning for it.
  useEffect(() => {
    const [w, h] = open ? [540, HEIGHT[shape]] : [248, 33];
    void invoke("set_open_size", { w, h });
  }, [open, shape]);

  // The docked pill is the notch, so it is sized by the hardware rather than by a
  // number we picked. Measured once -- the notch does not change while we run.
  useEffect(() => {
    void invoke<number>("notch_height").then((h) =>
      document.documentElement.style.setProperty("--notch-h", `${h}px`),
    );
  }, []);

  const busy = status !== "idle";
  // Only at rest. Busy already says what is happening and open has the whole
  // panel to say it, so a hint over either would be a second caption. Not
  // conditioned on `docked`: that decides where the companion lives, and the
  // hint is about what the notch does, which is true either way.
  const hint = near && !open && !busy && hold !== "";

  return (
    <div className="flex w-full justify-center">
      <div
        className={[
          "overflow-hidden text-ink select-none",
          // Black while it is a pill, a dark gradient once it is a panel.
          //
          // The pill has to read as part of the notch: the same black, the same
          // corners. Open, it used to be vibrancy -- the material every macOS
          // window is made of -- and over a bright wallpaper that stopped being
          // a window and became a smudge of one, the lake coming through as a
          // haze behind the text. The panel makes its own depth now: near black
          // at the notch so the seam is invisible, lifting toward the foot.
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
            ? "w-[540px]"
            : busy
              // Both pills are the notch's own height, so the strip reads as the
              // hardware getting wider rather than as a bar hanging below it.
              ? "h-(--notch-h) w-[300px]"
              // At rest it holds only the companion, so it needs to be barely
              // wider than the notch rather than a bar parked across the menu
              // bar. The hint goes *inside* this, not beside it: the strip is
              // already almost entirely empty black -- that emptiness is the
              // notch -- and growing it to make room pushed the companion out
              // past the hardware onto the menu bar, with a slab of nothing
              // where the width had gone.
              : "h-(--notch-h) w-[220px]",
        ].join(" ")}
        // Open, the height is the number; closed, the classes above own it, and
        // an inline height would win over them.
        style={open ? { height: HEIGHT[shape] } : undefined}
      >
        {busy && !open ? <StatusPill status={status} /> : <Pill open={open} hint={hint ? hold : ""} />}

        <div
          className={[
            "flex flex-col transition-opacity duration-200",
            open ? "opacity-100 delay-150" : "pointer-events-none opacity-0",
          ].join(" ")}
          style={{ height: HEIGHT[shape] }}
        >
          {account === null ? (
            // No rail, no sections. There is nowhere else to be yet, and
            // drawing the furniture of an app somebody cannot use is an
            // invitation to press things that will not work.
            <SignIn />
          ) : account === undefined ? null : allowing ? (
            // After the account, before anything else. Permissions are what the
            // app needs to work; an account is what it needs to know who you
            // are, and asking for the screen before somebody has decided to be
            // here at all is how a permission gets dismissed forever.
            <Allow onDone={() => setAllowing(false)} />
          ) : (
          <div className="flex min-h-0 flex-1">
          <Rail
            view={view}
            onGo={setView}
            newer={newer}
            docked={docked}
            onDock={() => dock(!docked)}
          />
          <div className="flex min-w-0 flex-1 flex-col">
          {view === "agents" ? (
            <Agents />
          ) : view === "integrations" ? (
            <Integrations onBack={() => setView(cameFrom)} />
          ) : view === "skills" ? (
            <Skills onBack={() => setView(cameFrom)} />
          ) : view === "report" ? (
            <Report kind={reporting} onBack={() => setView("settings")} />
          ) : view === "settings" ? (
            <Settings
              onIntegrations={() => openIntegrations("settings")}
              onSkills={() => openSkills("settings")}
              onReport={(k) => {
                setReporting(k);
                setView("report");
              }}
            />
          ) : (
          <Ask hold={hold} />
          )}
          </div>
          </div>
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
 * Cursor" -- you can see where it went. The socket stays either way, because an
 * empty spot that something belongs in is the half that carries the meaning.
 *
 * The companion cannot literally fly between two windows, so the illusion is
 * scale: releasing it grows and fades it out of the socket, calling it back drops
 * it in from larger with a small overshoot, like something landing.
 */
function Perch({ docked, onToggle }: { docked: boolean; onToggle: () => void }) {
  // Just the socket. The label it used to carry ("Call back" / "Release") made
  // this a pill wider than the rail it now lives in, so it was clipped down the
  // middle and its own text read as a tooltip stranded over the content. The
  // name is in the tooltip with every other one.
  return (
    <button
      onClick={onToggle}
      aria-pressed={docked}
      aria-label={docked ? "Release the companion" : "Call the companion back"}
      className="grid size-[30px] shrink-0 place-items-center rounded-full bg-raise transition-colors duration-150 hover:bg-raise-hi hairline"
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
 * Collapsed state: what sits in the notch.
 *
 * No label -- the point of living in the notch is reading as part of the hardware,
 * and a word beside the Apple menu reads as an app announcing itself.
 */
function Pill({ open, hint }: { open: boolean; hint: string }) {
  return (
    <div
      // One height class, not two. Listing `h-[38px]` and `h-0` together lets CSS
      // source order decide the winner rather than the condition -- the pill kept
      // its 38px while open, pushed the panel down, and clipped exactly that much
      // off the bottom.
      className={[
        "flex items-center justify-end gap-2 pr-3 transition-opacity duration-200",
        open ? "pointer-events-none h-0 opacity-0" : "h-(--notch-h) opacity-100 delay-150",
      ].join(" ")}
    >
      {/* What the notch is for, said only while somebody is on their way to it.
          Nothing here at rest: the whole point of living in the notch is being
          ignorable, and a permanent caption would be a toolbar.

          Delayed slightly rather than arriving with the strip -- text that lands
          before the room it sits in reads as the box growing around it. */}
      <span
        aria-hidden={!hint}
        className={[
          "flex items-center gap-1.5 overflow-hidden text-[10.5px] whitespace-nowrap text-ink-2",
          "transition-opacity duration-200 ease-out",
          hint ? "opacity-100 delay-100" : "opacity-0",
        ].join(" ")}
      >
        Hold
        <kbd className="rounded-[4px] bg-white/12 px-1.5 py-px font-sans text-[10px] text-white/90">
          {hint || "\u00a0"}
        </kbd>
        to ask
      </span>

      {/* Nudged up and in from the corner: sitting hard against the right edge
          it reads as clipped by the pill rather than resting in it. */}
      <div className="-translate-y-[6px] scale-[0.5]">
        <Companion mode="idle" anchored />
      </div>
    </div>
  );
}


type View = "home" | "agents" | "settings" | "integrations" | "skills" | "report";

/**
 * The sections, down the right-hand edge.
 *
 * They used to be two tabs and a gear in a header, with Integrations and Skills
 * reachable only from inside Home or Settings -- so the two things somebody
 * sets up most often were the two hardest to find, and getting back meant
 * remembering which door you came through.
 *
 * On the left. The argument for the right was that the pointer arrives from the
 * menu bar at the top-right, so that edge is nearest -- true, and worth less
 * than it sounds: the rail is read before it is clicked, and reading starts at
 * the left. Every list of sections anybody has used sits there, and being
 * cleverer than that costs more than the few pixels it saves.
 *
 * There is no Automation section because there is no such feature. Agents is
 * the automation: a task handed over and left to run. A tab for something that
 * does not exist is worse than its absence -- it is a promise the app then
 * breaks.
 *
 * Settings sits at the foot rather than in the run of sections, because it is
 * not one of the places you are going -- it is where you go to change how the
 * others behave. Every rail people already use puts it there, under the same
 * reasoning, which makes the bottom of the rail the first place anyone looks.
 */
function Rail({
  view,
  onGo,
  docked,
  onDock,
  newer,
}: {
  view: View;
  onGo: (v: View) => void;
  docked: boolean;
  onDock: () => void;
  /** A version is waiting, and the way to it is through Settings. */
  newer?: boolean;
}) {
  // Settings is missing from this list on purpose -- it is rendered at the foot.
  const items: [View, string, React.ReactNode][] = [
    ["home", "Home", <Home key="h" />],
    ["agents", "Agents", <Sparkle key="a" />],
    ["integrations", "Integrations", <Plug key="i" />],
    ["skills", "Skills", <Stack key="s" />],
  ];

  const tab = ([key, label, icon]: [View, string, React.ReactNode]) => {
    // Report is a page off Settings, so Settings stays lit while you are in
    // it -- otherwise the rail says you are nowhere.
    const on = view === key || (key === "settings" && view === "report");
    return (
      <Hint key={key} label={label}>
        <button
          onClick={() => onGo(key)}
          aria-label={label}
          aria-current={on ? "page" : undefined}
          className={[
            "grid size-[30px] place-items-center rounded-[9px]",
            "transition-colors duration-150",
            on ? "bg-raise-hi text-white" : "text-ink-2 hover:bg-raise hover:text-white/85",
          ].join(" ")}
        >
          {icon}
        </button>
      </Hint>
    );
  };

  return (
    <nav className="flex w-[46px] shrink-0 flex-col items-center gap-1 border-r border-line py-2.5">
      {items.map(tab)}

      <div className="mt-auto flex flex-col items-center gap-1.5">
        {/* A dot on the way in. The update itself lives in Settings, and a row
            in a page nobody has opened is not a notice -- this is the only
            thing on screen that says there is something to open it for. */}
        <span className="relative">
          {tab(["settings", "Settings", <Gear key="g" />])}
          {newer && (
            <span className="pointer-events-none absolute -top-px -right-px size-[7px] rounded-full bg-blue ring-2 ring-[#141417]" />
          )}
        </span>
        {/* The companion's socket, at the foot. Not a section, so it sits apart
            from them rather than reading as a fifth. */}
        <Hint label={docked ? "Release" : "Call back"}>
          <Perch docked={docked} onToggle={onDock} />
        </Hint>
      </div>
    </nav>
  );
}

/**
 * A label for an icon that has none.
 *
 * Icons alone are a memory test, and the rail is used a few times a day -- often
 * enough to learn, rarely enough to forget. The tooltip is what makes the trade
 * fair: the width goes back to the content, and the name is a hover away.
 *
 * Its own element rather than `title`, which the system draws after a delay it
 * chooses, in a style nobody picked, outside the window -- so on a panel that
 * sits over the menu bar it arrives late and looks like it belongs to something
 * else.
 */
function Hint({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <span className="group relative flex">
      {children}
      <span
        role="tooltip"
        className={[
          "pointer-events-none absolute top-1/2 left-[calc(100%+8px)] z-10 -translate-y-1/2",
          "rounded-md bg-[#2b2b2e] px-2 py-1 text-[11px] whitespace-nowrap text-white",
          "shadow-[0_4px_14px_rgba(0,0,0,0.5)] hairline",
          // Fades and slides a little, from the side it belongs to. Fast,
          // because this is read on the way to clicking something else.
          "origin-left scale-95 opacity-0 transition-[opacity,transform] duration-150 ease-out",
          "group-hover:scale-100 group-hover:opacity-100",
        ].join(" ")}
      >
        {label}
      </span>
    </span>
  );
}


/** A plug, for the things Nudge is joined to. */
function Plug() {
  return (
    <svg viewBox="0 0 16 16" className="size-[13px]" {...stroke}>
      <path d="M6 2.2v3.4M10 2.2v3.4M4 5.6h8v2.2a4 4 0 0 1-4 4 4 4 0 0 1-4-4Z" />
      <path d="M8 11.8v2" />
    </svg>
  );
}

/** Stacked layers: things saved to be used again. */
function Stack() {
  return (
    <svg viewBox="0 0 16 16" className="size-[13px]" {...stroke}>
      <path d="M8 2.4 14 5.4 8 8.4 2 5.4Z" />
      <path d="M2 8.2 8 11.2l6-3" />
      <path d="M2 10.9 8 13.9l6-3" />
    </svg>
  );
}

function Home() {
  return (
    <svg viewBox="0 0 16 16" className="size-3" {...stroke}>
      <path d="M2.5 7 8 2.5 13.5 7v6a.8.8 0 0 1-.8.8H3.3a.8.8 0 0 1-.8-.8Z" />
    </svg>
  );
}

/** An agent: a head with an antenna and two eyes. A sparkle means "something
 *  clever happens here", which is what every icon in every product means. */
function Sparkle() {
  return (
    <svg viewBox="0 0 16 16" className="size-[13px]" {...stroke}>
      <rect x="2.6" y="5.2" width="10.8" height="8" rx="2.4" />
      <path d="M8 5.2V2.8" />
      <circle cx="6" cy="9.2" r="0.9" fill="currentColor" stroke="none" />
      <circle cx="10" cy="9.2" r="0.9" fill="currentColor" stroke="none" />
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
