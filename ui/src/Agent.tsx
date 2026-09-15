import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Face } from "./components/Face";

export type AgentState =
  | { state: "running" }
  | { state: "waiting"; question: string }
  | { state: "done" }
  | { state: "failed"; why: string }
  | { state: "stopped" };

/** A command an agent ran, and what came back. */
export type Ran = { command: string; output: string };
/** A file it produced. */
export type Made = { path: string };
/** One line of its plan. */
export type Todo = { text: string; status: "pending" | "active" | "done" };

export type Agent = {
  id: number;
  goal: string;
  title: string;
  status: string;
  step: number;
  history: string[];
  ran: Ran[];
  made: Made[];
  plan: Todo[];
} & AgentState;

const TONE = {
  running: { label: "RUNNING", pill: "bg-[#0a84ff] text-white" },
  waiting: { label: "NEEDS YOU", pill: "bg-[#e8b027] text-black" },
  done: { label: "DONE", pill: "bg-white/15 text-white/70" },
  failed: { label: "FAILED", pill: "bg-[#ff5f57]/85 text-white" },
  stopped: { label: "STOPPED", pill: "bg-white/15 text-white/60" },
} as const;

/**
 * The plan, when there is one.
 *
 * Worth showing above everything else: it is the only thing on the card that
 * answers "how much is left" honestly. The progress bar underneath counts the
 * same list, so the two can never disagree.
 */
export function Plan({ plan }: { plan: Todo[] }) {
  if (plan.length === 0) return null;
  return (
    <ol className="mt-2.5 space-y-1">
      {plan.map((t, i) => (
        <li key={i} className="flex items-start gap-1.5 text-[11px] leading-snug">
          <Mark status={t.status} />
          <span
            className={
              t.status === "done"
                ? "text-white/30 line-through decoration-white/20"
                : t.status === "active"
                  ? "text-white"
                  : "text-white/55"
            }
          >
            {t.text}
          </span>
        </li>
      ))}
    </ol>
  );
}

/**
 * Three states, three shapes -- not three colours.
 *
 * Colour alone would be invisible to anyone who cannot separate these hues, and
 * the card already uses colour to mean which agent this is.
 */
function Mark({ status }: { status: Todo["status"] }) {
  if (status === "done") {
    return (
      <svg viewBox="0 0 12 12" className="mt-[3px] size-2.5 shrink-0 text-[#30d158]" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
        <path d="M2.5 6.3 4.8 8.6 9.5 3.9" />
      </svg>
    );
  }
  if (status === "active") {
    return (
      <span className="relative mt-[4px] grid size-2 shrink-0 place-items-center">
        {/* Behind the dot, so the dot itself does not move on a line of text. */}
        <span aria-hidden className="absolute size-2 rounded-full bg-[#0a84ff] motion-safe:animate-halo" />
        <span className="relative size-2 rounded-full bg-[#0a84ff]" />
      </span>
    );
  }
  return <span className="mt-[4px] size-2 shrink-0 rounded-full ring-1 ring-white/25" />;
}

/**
 * What an agent made. The point of the whole thing, so it sits above the log.
 *
 * A row of files rather than a list of writes: asked for a landing page, what
 * you want afterwards is the page, not a transcript of the making of it.
 * Clicking opens it in whatever owns that kind of file.
 */
export function Artifacts({ made }: { made: Made[] }) {
  if (made.length === 0) return null;
  return (
    <div className="mt-2 flex flex-wrap gap-1.5">
      {made.map((m) => (
        <button
          key={m.path}
          onClick={() => void invoke("open_artifact", { path: m.path })}
          title={m.path}
          className="flex max-w-full items-center gap-1.5 rounded-lg bg-white/[0.07] py-1 pr-2 pl-1.5 text-[10.5px] text-white/75 transition-colors duration-150 hover:bg-white/[0.13] hover:text-white active:scale-[0.97]"
        >
          <svg viewBox="0 0 16 16" className="size-3 shrink-0 text-white/45" fill="none" stroke="currentColor" strokeWidth={1.5} strokeLinecap="round" strokeLinejoin="round">
            <path d="M9 1.5H4.5A1.5 1.5 0 0 0 3 3v10a1.5 1.5 0 0 0 1.5 1.5h7A1.5 1.5 0 0 0 13 13V5.5Z" />
            <path d="M9 1.5V5.5H13" />
          </svg>
          <span className="truncate">{m.path.split("/").pop()}</span>
        </button>
      ))}
    </div>
  );
}

/**
 * What the agent ran on your machine.
 *
 * Collapsed by default and open on a click, because both are true: the commands
 * are the most important thing here when something has gone wrong, and noise
 * every other time. `<details>` rather than state -- the browser already knows
 * how to do this, remembers it per element, and is keyboard-accessible without
 * being told.
 */
export function Commands({ ran }: { ran: Ran[] }) {
  if (ran.length === 0) return null;
  return (
    <details className="group mt-2">
      <summary className="flex cursor-default list-none items-center gap-1 text-[9.5px] tracking-wide text-white/30 uppercase transition-colors duration-150 hover:text-white/55">
        <svg
          viewBox="0 0 12 12"
          className="size-2.5 transition-transform duration-200 ease-out group-open:rotate-90"
          fill="none"
          stroke="currentColor"
          strokeWidth={1.6}
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <path d="M4.5 2.5 8 6l-3.5 3.5" />
        </svg>
        {ran.length} command{ran.length === 1 ? "" : "s"}
      </summary>
      {/* Capped and scrollable: a directory listing must not make the card
          taller than the thing it lives in. */}
      <div className="mt-1.5 max-h-44 space-y-1.5 overflow-y-auto">
        {ran.map((r, i) => (
          <div key={i} className="rounded-lg bg-black/40 p-1.5">
            <code className="block font-mono text-[10px] break-all text-[#8ec6ff]">
              <span className="text-white/25">$ </span>
              {r.command}
            </code>
            <pre className="mt-1 font-mono text-[9.5px] leading-snug whitespace-pre-wrap text-white/45">
              {r.output.trim()}
            </pre>
          </div>
        ))}
      </div>
    </details>
  );
}

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
  // Collapsed to start. A running agent should be visible, not in the way --
  // the tile says "still going" and the card is one click away. A question is
  // the exception and opens itself, below.
  const [collapsed, setCollapsed] = useState(true);

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

  // Finished work belongs in the Agents tab, not floating over the screen.
  const live = agents.filter((a) => a.state === "running" || a.state === "waiting");
  if (live.length === 0) return null;

  if (collapsed) {
    return (
      <div className="flex w-full flex-col items-end gap-1.5 p-1">
        {live.map((a) => (
          <Tile key={a.id} agent={a} onOpen={() => setCollapsed(false)} />
        ))}
      </div>
    );
  }

  return (
    <div className="flex w-full flex-col gap-1.5 p-1">
      {live.map((a) => (
        <Card key={a.id} agent={a} onCollapse={() => setCollapsed(true)} />
      ))}
    </div>
  );
}

/**
 * One agent, collapsed to a square.
 *
 * Colour is per agent rather than per state, so two running at once are
 * distinguishable at a glance -- which is the only reason to show several
 * squares instead of a count. State is carried by the halo and the dot, not by
 * the colour, or the two meanings would fight.
 */
function Tile({ agent, onOpen }: { agent: Agent; onOpen: () => void }) {
  const c = colour(agent.id);
  return (
    <button
      onClick={onOpen}
      aria-label={`${agent.title} — expand`}
      title={agent.title}
      style={{ backgroundColor: c.bg }}
      className="relative grid size-[38px] shrink-0 place-items-center rounded-[11px] text-white transition-transform duration-150 ease-[cubic-bezier(0.23,1,0.32,1)] hover:scale-105 active:scale-[0.97]"
    >
      {/* The tile is what is on screen almost all the time -- the card is
          collapsed unless someone opens it -- so this is where being visibly
          alive actually counts. It was a drawing of a robot, which looks the
          same whether the agent is working or has been dead for a minute. */}
      {/* Sized here, not by the wrapper. `Face` draws at its own `size` and
          defaults to 22, so a larger box around it only moved the canvas off
          centre -- which is exactly what it looked like. */}
      <Face state={agent.state} step={agent.step} size={32} />
      {agent.state === "waiting" && (
        <span className="absolute -top-0.5 -right-0.5 size-2.5 rounded-full bg-[#e8b027] ring-2 ring-black/60" />
      )}
    </button>
  );
}

/**
 * Six, cycled by id. Enough that two agents are never the same colour in
 * practice, few enough that each one stays recognisable.
 */
function colour(id: number) {
  const palette = [
    { bg: "#0a84ff", glow: "rgba(10,132,255,0.45)" },
    { bg: "#a855f7", glow: "rgba(168,85,247,0.45)" },
    { bg: "#2ec8c8", glow: "rgba(46,200,200,0.45)" },
    { bg: "#ff9f0a", glow: "rgba(255,159,10,0.45)" },
    { bg: "#ff2d55", glow: "rgba(255,45,85,0.45)" },
    { bg: "#30d158", glow: "rgba(48,209,88,0.45)" },
  ];
  return palette[Math.abs(id) % palette.length];
}

function Card({ agent, onCollapse }: { agent: Agent; onCollapse: () => void }) {
  return (
    <div className="w-full">
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
          <Face state={agent.state} step={agent.step} />
          <h1 className="min-w-0 flex-1 truncate text-[13px] font-semibold">{agent.title}</h1>
          <span
            className={`rounded-full px-2 py-[2px] text-[9px] font-bold tracking-wide ${TONE[agent.state].pill}`}
          >
            {TONE[agent.state].label}
          </span>
          <button
            onClick={onCollapse}
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

        <Plan plan={agent.plan} />
        <Artifacts made={agent.made} />
        <Commands ran={agent.ran} />

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
