import { useEffect, useRef, useState } from "react";

/** Typed fallback. Speaking is the primary path; this is for quiet rooms. */
export function AskInput({ onSubmit }: { onSubmit: (goal: string) => void }) {
  const [value, setValue] = useState("");
  const ref = useRef<HTMLInputElement>(null);

  // The window only accepts clicks while this is mounted, so grab focus at once
  // or the first keystrokes fall through to the app underneath.
  useEffect(() => ref.current?.focus(), []);

  return (
    <form
      onSubmit={(e) => {
        e.preventDefault();
        const goal = value.trim();
        if (goal) onSubmit(goal);
      }}
      // While asking, the native window is shrunk to just this box -- so it fills
      // the window rather than positioning itself against a full screen.
      className={[
        "fixed inset-0 flex items-center rounded-2xl px-5",
        "bg-neutral-900/95 backdrop-blur-xl shadow-[0_12px_40px_#00000088]",
        "inset-ring-1 inset-ring-white/12",
      ].join(" ")}
    >
      <input
        ref={ref}
        value={value}
        onChange={(e) => setValue(e.target.value)}
        autoComplete="off"
        spellCheck={false}
        aria-label="What do you want to do?"
        placeholder="Hold the hotkey to speak, or type here"
        className="w-full bg-transparent text-[17px] text-white outline-none placeholder:text-white/40"
      />
    </form>
  );
}
