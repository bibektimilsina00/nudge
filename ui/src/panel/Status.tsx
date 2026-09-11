import { useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";

export type Status = "idle" | "listening" | "thinking" | "speaking" | "agent";

/**
 * Each state gets its own colour, so the notch is readable from the corner of
 * your eye without reading the word.
 */
const TONE: Record<Exclude<Status, "idle">, { label: string; glow: string; ink: string }> = {
  agent: { label: "Working", glow: "#0a84ff", ink: "#8ec6ff" },
  listening: { label: "Listening", glow: "#2ec8c8", ink: "#7fe9e9" },
  thinking: { label: "Thinking", glow: "#a855f7", ink: "#d8b4fe" },
  speaking: { label: "Speaking", glow: "#e8913a", ink: "#f5c48a" },
};

/**
 * The notch pill while Nudge is busy.
 *
 * The glow is anchored to the right edge and bleeds off it, so the pill reads as
 * lit from inside the hardware rather than as a coloured rectangle stuck on. It
 * sits under the label rather than beside it, which is what keeps the left side
 * plain black and legible.
 */
export function StatusPill({ status }: { status: Exclude<Status, "idle"> }) {
  const tone = TONE[status];
  return (
    <div className="relative flex h-[38px] items-center overflow-hidden px-4">
      <span className="z-10 text-[13px] font-semibold tracking-tight">{tone.label}</span>
      <span className="flex-1" />
      <span
        aria-hidden
        className="pointer-events-none absolute inset-y-0 right-0 w-[190px]"
        style={{
          background: `radial-gradient(120% 140% at 100% 50%, ${tone.glow}cc 0%, ${tone.glow}55 38%, transparent 72%)`,
        }}
      />
      <span className="z-10 flex items-center" style={{ color: tone.ink }}>
        {status === "thinking" || status === "agent" ? (
          <Dots />
        ) : (
          <Bars live={status === "listening"} />
        )}
      </span>
    </div>
  );
}

/** Three dots, staggered -- the shape of waiting. */
function Dots() {
  return (
    <span className="flex gap-[5px]">
      {[0, 1, 2].map((i) => (
        <span
          key={i}
          className="size-[5px] rounded-full bg-current motion-safe:animate-think"
          style={{ animationDelay: `${i * 140}ms` }}
        />
      ))}
    </span>
  );
}

/**
 * A level meter. While listening it follows the real microphone; while speaking
 * there is nothing to measure, so it animates on its own -- honest either way,
 * because a meter that pretends to track audio it cannot hear is worse than one
 * that is plainly decorative.
 */
function Bars({ live }: { live: boolean }) {
  const bars = useRef<(HTMLSpanElement | null)[]>([]);

  useEffect(() => {
    const level = { raw: 0, shown: 0 };
    let frame = 0;
    let phase = 0;

    const sub = live
      ? listen<number>("level", (e) => {
          level.raw = Math.min(1, Math.max(0, e.payload));
        })
      : null;

    const tick = () => {
      frame = requestAnimationFrame(tick);
      phase += 0.16;
      // Rises fast, falls slowly -- how a real meter behaves.
      const up = level.raw > level.shown;
      level.shown += (level.raw - level.shown) * (up ? 0.35 : 0.08);

      bars.current.forEach((bar, i) => {
        if (!bar) return;
        const wave = 0.5 + 0.5 * Math.sin(phase + i * 1.1);
        const amount = live ? 0.25 + level.shown * wave * 1.4 : 0.3 + wave * 0.7;
        bar.style.transform = `scaleY(${Math.min(1, amount).toFixed(3)})`;
      });
    };
    frame = requestAnimationFrame(tick);

    return () => {
      cancelAnimationFrame(frame);
      void sub?.then((un) => un());
    };
  }, [live]);

  return (
    <span className="flex h-[15px] items-center gap-[3px]">
      {[0, 1, 2, 3].map((i) => (
        <span
          key={i}
          ref={(el) => {
            bars.current[i] = el;
          }}
          className="h-full w-[2.5px] origin-center rounded-full bg-current"
        />
      ))}
    </span>
  );
}
