import type { ReactNode } from "react";

/** The one surface that ever shows text: status, transcript, or error. */
type Tone = "normal" | "unsure" | "error";


const TONES: Record<Tone, string> = {
  normal: "bg-neutral-900/95 inset-ring-1 inset-ring-white/12",
  unsure: "bg-amber-950/90 inset-ring-1 inset-ring-amber-400/30",
  error: "bg-red-950/85 inset-ring-1 inset-ring-red-400/30",
};

export function Bubble({
  tone = "normal",
  heard,
  typing,
  children,
}: {
  tone?: Tone;
  /** What Nudge thought you said. Kept on screen so a misheard goal is obvious. */
  heard?: string;
  /** Text to enter. Shown verbatim -- in guide mode this is what you copy. */
  typing?: string | null;
  children: ReactNode;
}) {
  return (
    <div
      role="status"
      aria-live="polite"
      className={[
        "fixed bottom-14 left-1/2 max-w-[620px] -translate-x-1/2",
        "rounded-2xl px-5 py-3.5 text-[15px] leading-normal text-white",
        "backdrop-blur-xl shadow-[0_12px_40px_#00000088]",
        TONES[tone],
      ].join(" ")}
    >
      {heard && <p className="mb-1 text-[13px] text-white/45">heard “{heard}”</p>}
      {children}
      {typing && (
        <p className="mt-2 rounded-lg bg-black/45 px-3 py-2 font-mono text-[13px] break-all text-white/95 inset-ring-1 inset-ring-white/10">
          {typing}
        </p>
      )}
    </div>
  );
}
