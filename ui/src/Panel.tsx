import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Companion } from "./components/Companion";

const SHORTCUTS: [string, string][] = [
  ["Talk", "hold ⌘⇧Space"],
  ["Next step", "tap ⌘⇧Space"],
  ["Type instead", "tap, then type"],
  ["Stop", "Escape"],
];

/**
 * The menu-bar dropdown, and the companion's home while it is docked.
 *
 * Docking exists because an always-present companion is a commitment. Parking it
 * here gives the screen back without quitting, and undocking is a deliberate "go
 * on then" rather than something that happens at launch and never stops.
 */
export default function Panel() {
  const [docked, setDocked] = useState(false);
  const [auto, setAuto] = useState(false);
  const body = useRef<HTMLDivElement>(null);

  useEffect(() => {
    void invoke<boolean>("docked").then(setDocked);
    void invoke<boolean>("auto").then(setAuto);
    const sub = listen<boolean>("docked", (e) => setDocked(e.payload));
    return () => void sub.then((un) => un());
  }, []);

  // The panel is a dropdown, not a window: it should be exactly as tall as what is
  // in it, measured after layout rather than guessed in the window config.
  useLayoutEffect(() => {
    const h = body.current?.getBoundingClientRect().height;
    if (h) void invoke("fit_panel", { height: Math.ceil(h) });
  }, [docked]);

  const dock = (next: boolean) => {
    setDocked(next);
    void invoke("set_docked", { docked: next });
  };

  return (
    <div
      ref={body}
      className="w-full rounded-2xl bg-neutral-900/95 p-4 text-white backdrop-blur-2xl inset-ring-1 inset-ring-white/12"
    >
      <div className="flex items-baseline justify-between">
        <h1 className="text-[15px] font-semibold tracking-tight">Nudge</h1>
        <span className="text-[12px] text-white/40">
          {docked ? "parked" : "on your screen"}
        </span>
      </div>

      {/* The companion lives here while docked -- the same component that follows
          the cursor, so it is visibly the same creature either way. */}
      <div className="relative my-3 grid h-24 place-items-center rounded-xl bg-black/30 inset-ring-1 inset-ring-white/5">
        {docked ? (
          <Companion mode="idle" anchored />
        ) : (
          <p className="text-[12px] text-white/35">out there, following your cursor</p>
        )}
      </div>

      <button
        onClick={() => dock(!docked)}
        className="w-full rounded-xl bg-white/10 py-2 text-[13px] font-medium transition-colors duration-150 hover:bg-white/15 active:bg-white/20"
      >
        {docked ? "Undock cursor" : "Dock cursor"}
      </button>

      <Toggle
        label="Click for me"
        hint="Nudge does it instead of pointing"
        on={auto}
        // The command answers with what actually happened: asking for it without
        // Accessibility leaves the switch off rather than lying about it.
        onChange={(v) => void invoke<boolean>("set_auto", { on: v }).then(setAuto)}
      />

      <dl className="mt-3 space-y-1.5 border-t border-white/8 pt-3">
        {SHORTCUTS.map(([name, keys]) => (
          <div key={name} className="flex items-center justify-between">
            <dt className="text-[12px] text-white/55">{name}</dt>
            <dd className="rounded-md bg-white/8 px-2 py-0.5 font-mono text-[11px] text-white/75">
              {keys}
            </dd>
          </div>
        ))}
      </dl>
    </div>
  );
}

function Toggle({
  label,
  hint,
  on,
  onChange,
}: {
  label: string;
  hint: string;
  on: boolean;
  onChange: (on: boolean) => void;
}) {
  return (
    <label className="mt-3 flex cursor-default items-center justify-between gap-3">
      <span>
        <span className="block text-[13px]">{label}</span>
        <span className="block text-[11px] text-white/40">{hint}</span>
      </span>
      <button
        role="switch"
        aria-checked={on}
        onClick={() => onChange(!on)}
        className={`h-6 w-10 shrink-0 rounded-full p-0.5 transition-colors duration-200 ${
          on ? "bg-accent" : "bg-white/15"
        }`}
      >
        <span
          className={`block size-5 rounded-full bg-white transition-transform duration-200 ease-[cubic-bezier(0.23,1,0.32,1)] ${
            on ? "translate-x-4" : "translate-x-0"
          }`}
        />
      </button>
    </label>
  );
}
