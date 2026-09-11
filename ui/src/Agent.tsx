import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type AgentState =
  | { state: "running" }
  | { state: "waiting"; question: string }
  | { state: "done" }
  | { state: "failed"; why: string }
  | { state: "stopped" };

export type Agent = {
  id: number;
  goal: string;
  title: string;
  status: string;
  step: number;
  history: string[];
} & AgentState;

const TONE = {
  running: { label: "RUNNING", pill: "bg-[#0a84ff] text-white" },
  waiting: { label: "NEEDS YOU", pill: "bg-[#e8b027] text-black" },
  done: { label: "DONE", pill: "bg-white/15 text-white/70" },
  failed: { label: "FAILED", pill: "bg-[#ff5f57]/85 text-white" },
  stopped: { label: "STOPPED", pill: "bg-white/15 text-white/60" },
} as const;

/**
 * The floating card: what an agent is doing, and how to stop it.
 *
 * It is deliberately loud while running. An agent owns the real cursor and
 * keyboard for as long as it works, so the one thing that must never be hard to
 * find is the way out.
 *
 * The window stays one size and the content collapses, the same approach the
 * notch uses -- animating a window resize tears on a Retina display.
 */
export default function AgentCard() {
  const [agents, setAgents] = useState<Agent[]>([]);
  const [collapsed, setCollapsed] = useState(false);

  useEffect(() => {
    void invoke<Agent[]>("agents").then(setAgents);
    const sub = listen<Agent[]>("agents", (e) => setAgents(e.payload));
    return () => void sub.then((un) => un());
  }, []);

  // Something new to say is a reason to come back out.
  const waiting = agents.some((a) => a.state === "waiting");
  useEffect(() => {
    if (waiting) setCollapsed(false);
  }, [waiting]);

  const agent = agents[agents.length - 1];
  if (!agent) return null;

  if (collapsed) {
    return (
      <div className="flex w-full justify-end p-1">
        <button
          onClick={() => setCollapsed(false)}
          aria-label={`${agent.title} — expand`}
          className="relative grid size-[38px] place-items-center rounded-full bg-[#0a84ff] text-white shadow-[0_6px_20px_rgba(10,132,255,0.45)] transition-transform duration-150 hover:scale-105"
        >
          <svg viewBox="0 0 12 12" className="size-3" fill="currentColor">
            <path d="M3 1.8 10 6l-7 4.2Z" />
          </svg>
          {agent.state !== "running" && (
            <span className="absolute -top-0.5 -right-0.5 size-2.5 rounded-full bg-[#e8b027] ring-2 ring-black/60" />
          )}
        </button>
      </div>
    );
  }

  return (
    <div className="w-full p-1">
      <div
        className={[
          "rounded-2xl bg-[#141824] p-3.5 text-white backdrop-blur-xl",
          "inset-ring-1 inset-ring-white/[0.12]",
          agent.state === "running"
            ? "shadow-[0_8px_28px_rgba(10,132,255,0.35)]"
            : "shadow-[0_8px_28px_rgba(0,0,0,0.5)]",
        ].join(" ")}
      >
        <header className="flex items-center gap-2">
          <h1 className="min-w-0 flex-1 truncate text-[13px] font-semibold">{agent.title}</h1>
          <span
            className={`rounded-full px-2 py-[2px] text-[9px] font-bold tracking-wide ${TONE[agent.state].pill}`}
          >
            {TONE[agent.state].label}
          </span>
          <button
            onClick={() => setCollapsed(true)}
            aria-label="Collapse"
            className="grid size-[20px] place-items-center rounded-full text-white/40 transition-colors duration-150 hover:bg-white/10 hover:text-white"
          >
            <svg viewBox="0 0 12 12" className="size-2.5" fill="none" stroke="currentColor" strokeWidth={1.8} strokeLinecap="round">
              <path d="M2.5 6h7" />
            </svg>
          </button>
          <button
            onClick={() => void invoke("dismiss_agent", { id: agent.id })}
            aria-label="Dismiss"
            className="grid size-[20px] place-items-center rounded-full text-white/40 transition-colors duration-150 hover:bg-white/10 hover:text-white"
          >
            <svg viewBox="0 0 12 12" className="size-2.5" fill="none" stroke="currentColor" strokeWidth={1.8} strokeLinecap="round">
              <path d="M3 3l6 6M9 3l-6 6" />
            </svg>
          </button>
        </header>

        {agent.state === "waiting" ? (
          <Question id={agent.id} question={agent.question} />
        ) : (
          <p className="mt-2 line-clamp-3 text-[11.5px] leading-snug text-white/70">
            {agent.state === "failed" ? agent.why : agent.status}
          </p>
        )}

        <div className="mt-3 h-[3px] overflow-hidden rounded-full bg-white/10">
          <div
            className="h-full rounded-full bg-[#0a84ff] transition-[width] duration-500 ease-out"
            style={{ width: `${progress(agent) * 100}%` }}
          />
        </div>

        {agent.state === "running" && (
          <div className="mt-2.5 flex items-center justify-between">
            <span className="text-[10px] text-white/30">
              step {agent.step} · your cursor is in use
            </span>
            <button
              onClick={() => void invoke("stop_agent", { id: agent.id })}
              className="rounded-lg bg-white/10 px-2.5 py-1 text-[11px] font-medium transition-colors duration-150 hover:bg-[#ff5f57] hover:text-white"
            >
              Stop
            </button>
          </div>
        )}
      </div>
    </div>
  );
}

/** Mirrors `Agent::progress` in Rust: steps against the budget, never backwards. */
function progress(agent: Agent) {
  if (agent.state === "done") return 1;
  return Math.min(0.97, agent.step / 40);
}

function Question({ id, question }: { id: number; question: string }) {
  const [text, setText] = useState("");
  const field = useRef<HTMLInputElement>(null);

  // It is blocked until this is answered, so do not make anyone go looking for
  // the field.
  useEffect(() => field.current?.focus(), []);

  return (
    <form
      onSubmit={(e) => {
        e.preventDefault();
        const answer = text.trim();
        if (!answer) return;
        void invoke("answer_agent", { id, text: answer });
        setText("");
      }}
    >
      <p className="mt-2 text-[11.5px] leading-snug text-white/85">{question}</p>
      <input
        ref={field}
        value={text}
        onChange={(e) => setText(e.target.value)}
        placeholder="Type your answer…"
        spellCheck={false}
        className="mt-2 w-full rounded-lg bg-black/40 px-2.5 py-1.5 text-[11.5px] text-white outline-none inset-ring-1 inset-ring-white/[0.12] placeholder:text-white/25 focus:inset-ring-[#0a84ff]"
      />
    </form>
  );
}
