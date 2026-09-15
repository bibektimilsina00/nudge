import { useEffect, useRef, useState, type ReactNode } from "react";
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
  running: { label: "Running", dot: "bg-[#0a84ff]" },
  waiting: { label: "Needs you", dot: "bg-[#e8b027]" },
  done: { label: "Done", dot: "bg-[#30d158]" },
  failed: { label: "Failed", dot: "bg-[#ff5f57]" },
  stopped: { label: "Stopped", dot: "bg-white/30" },
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
      <span className="mt-[4px] size-2 shrink-0 rounded-full bg-[#0a84ff] ring-2 ring-[#0a84ff]/25" />
    );
  }
  return <span className="mt-[4px] size-2 shrink-0 rounded-full ring-1 ring-white/25" />;
}

/**
 * Every step it has taken, in order.
 *
 * `history` has always been collected and never shown, so the card answered
 * "what is it doing" and never "what has it done" -- which is the question
 * somebody actually has when they come back to it after two minutes away.
 *
 * Newest first, because the interesting end of a list of forty is the end that
 * just happened. Open on a click, like the commands: this is the detail, and
 * detail that is always open is noise until the moment it is not.
 */
export function Steps({ history }: { history: string[] }) {
  if (history.length === 0) return null;
  return (
    <Fold count={history.length} label={`step${history.length === 1 ? "" : "s"} taken`}>
      <ol className="mt-1.5 max-h-44 space-y-1 overflow-y-auto">
        {[...history].reverse().map((line, i) => (
          <li key={i} className="flex items-start gap-1.5 text-[10.5px] leading-snug text-white/55">
            {/* Numbered from the real position, not from the top of a reversed
                list -- otherwise the newest step is called number one. */}
            <span className="mt-[1px] w-4 shrink-0 text-right font-mono text-[9px] text-white/25">
              {history.length - i}
            </span>
            <span className="min-w-0">{line}</span>
          </li>
        ))}
      </ol>
    </Fold>
  );
}

/**
 * The disclosure both lists use.
 *
 * `<details>` rather than state: the browser already knows how to do this,
 * remembers it per element, and is keyboard-accessible without being told.
 */
function Fold({
  count,
  label,
  children,
}: {
  count: number;
  label: string;
  children: ReactNode;
}) {
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
        {count} {label}
      </summary>
      {children}
    </details>
  );
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
    <div className="mt-2.5">
      <p className="mb-1 text-[9.5px] tracking-wide text-white/30 uppercase">
        {made.length} file{made.length === 1 ? "" : "s"} — click to open
      </p>
      <div className="flex flex-wrap gap-1.5">
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
    <Fold count={ran.length} label={`command${ran.length === 1 ? "" : "s"} run`}>
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
    </Fold>
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
  // Which agent's card is open, by id. Nothing is open to start: a running agent
  // should be visible, not in the way.
  //
  // By id rather than a single `collapsed` flag, which is what it was: opening
  // one opened all four at once, because there was one piece of state for four
  // cards. A question opens itself, below.
  const [opened, setOpened] = useState<number | null>(null);
  // Tiles somebody has waved away. Hiding is not stopping -- the agent carries on
  // and the Agents tab still has it; this is only about the corner of the screen.
  const [hidden, setHidden] = useState<number[]>([]);

  useEffect(() => {
    void invoke<Agent[]>("agents").then(setAgents);
    const sub = listen<Agent[]>("agents", (e) => setAgents(e.payload));
    return () => void sub.then((un) => un());
  }, []);

  // Clicking anywhere else closes an open card, the way every panel and popover
  // on this machine does.
  //
  // It has to come from outside: a click that lands somewhere else goes to
  // whatever is there and this window hears nothing at all. The pointer loop
  // watches for it and says so; the decision is here, because this is the only
  // thing that knows whether a card is open -- clicking away from a column of
  // tiles should do nothing, and does.
  useEffect(() => {
    const sub = listen("away", () => setOpened(null));
    return () => void sub.then((un) => un());
  }, []);

  // A question is a reason to come out -- but only the agent that asked it.
  const asking = agents.find((a) => a.state === "waiting")?.id ?? null;
  useEffect(() => {
    if (asking !== null) setOpened(asking);
  }, [asking]);

  // Finished work belongs in the Agents tab, not floating over the screen.
  const live = agents
    .filter((a) => a.state === "running" || a.state === "waiting")
    // A question un-hides itself: it needs a person, and a hidden tile cannot ask.
    .filter((a) => a.state === "waiting" || !hidden.includes(a.id));

  // Tell the window how big to be.
  //
  // A transparent window is still a window: at its old fixed 360x520 it sat over
  // a swathe of desktop that could not be dragged on, clicked through or dropped
  // into -- an invisible hole whose only tenant was a 46px face. Only the browser
  // knows how big the tiles came out, so the browser says.
  const box = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const el = box.current;
    if (!el) return;
    const tell = () => {
      const r = el.getBoundingClientRect();
      void invoke("fit_agents", { width: r.width, height: r.height }).catch(() => {});
    };
    tell();
    const watch = new ResizeObserver(tell);
    watch.observe(el);
    return () => watch.disconnect();
  }, [live.length, opened]);

  if (live.length === 0) return null;

  const open = live.find((a) => a.id === opened);

  return (
    // `w-fit` so the measurement above means something: the element has to be the
    // size of what is in it, which is a column of tiles or one card.
    <div ref={box} className="flex w-fit flex-col items-end gap-2.5 p-2.5">
      {open ? (
        <Card agent={open} onCollapse={() => setOpened(null)} />
      ) : (
        live.map((a) => (
          <Tile
            key={a.id}
            agent={a}
            onOpen={() => setOpened(a.id)}
            onHide={() => setHidden((h) => [...h, a.id])}
          />
        ))
      )}
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
function Tile({
  agent,
  onOpen,
  onHide,
}: {
  agent: Agent;
  onOpen: () => void;
  onHide: () => void;
}) {
  return (
    // `group` so the controls appear on hovering anywhere on the tile, not only
    // on the eight pixels they occupy -- which at this size would be a game.
    <div className="group relative size-[46px] shrink-0">
      <button
        onClick={onOpen}
        aria-label={`${agent.title} — expand`}
        title={agent.title}
        className="grid size-full place-items-center text-white transition-transform duration-150 ease-[cubic-bezier(0.23,1,0.32,1)] hover:scale-105 active:scale-[0.97]"
      >
        <Face state={agent.state} step={agent.step} size={46} />
      </button>

      {/* A question outranks the controls: it is the one state that needs a
          person, and it should not be hidden behind a hover. It sits in the same
          corner the controls do and clears out of their way when they appear --
          the tiles are right-aligned, so that corner is the one nearest the
          pointer coming in from the screen. */}
      {agent.state === "waiting" && (
        <span className="pointer-events-none absolute top-0 right-0 size-2.5 rounded-full bg-[#e8b027] ring-2 ring-black/60 group-hover:opacity-0" />
      )}

      {/* Stop, and get out of the way. Two different things: one ends the work,
          the other only ends having to look at it, and an agent that keeps
          running is the ordinary reason to want the corner back. */}
      <div className="pointer-events-none absolute -top-1 -right-1 flex gap-[3px] opacity-0 transition-opacity duration-150 group-hover:pointer-events-auto group-hover:opacity-100">
        <Dot
          label={`Stop ${agent.title}`}
          tint="#ff5f57"
          onClick={() => void invoke("stop_agent", { id: agent.id }).catch(() => {})}
        >
          <path d="M4 4l6 6M10 4l-6 6" />
        </Dot>
        <Dot label={`Hide ${agent.title}`} tint="#febc2e" onClick={onHide}>
          <path d="M3.5 7h7" />
        </Dot>
      </div>
    </div>
  );
}

/**
 * One of the two little controls, sized like the traffic lights people already
 * know -- which is the whole reason they are round, coloured and in that order.
 */
function Dot({
  label,
  tint,
  onClick,
  children,
}: {
  label: string;
  tint: string;
  onClick: () => void;
  children: ReactNode;
}) {
  return (
    <button
      aria-label={label}
      title={label}
      onClick={onClick}
      style={{ backgroundColor: tint }}
      className="grid size-[13px] place-items-center rounded-full text-black/55 ring-1 ring-black/20 transition-transform duration-100 hover:scale-110 active:scale-95"
    >
      <svg viewBox="0 0 14 14" className="size-[9px]" fill="none" stroke="currentColor" strokeWidth={1.9} strokeLinecap="round">
        {children}
      </svg>
    </button>
  );
}

/* The six-colour palette went with the tile's background.
 *
 * It existed so two agents running at once were distinguishable at a glance --
 * which `Agents::start` has always made impossible: it refuses to start a second
 * while one is live, because there is one cursor. The colour was distinguishing
 * a case that cannot occur. */

function Card({ agent, onCollapse }: { agent: Agent; onCollapse: () => void }) {
  const tone = TONE[agent.state];
  return (
    // No shadow. It floats over whatever happens to be behind it, and a drop
    // shadow on a dark card over a dark screen is a smudge -- the ring is what
    // separates it from the desktop, and the ring is enough.
    //
    // The surface is the panel's, not one of its own: `#1e1e1e` with a hairline
    // inset ring is the language everything else in Nudge is already written in.
    <div className="w-[326px] rounded-2xl bg-[#1e1e1e] text-white inset-ring-1 inset-ring-white/[0.09]">
      <header className="flex items-center gap-2.5 px-3 pt-2.5 pb-2">
        <Face state={agent.state} step={agent.step} size={26} />
        <div className="min-w-0 flex-1">
          <h1 className="truncate text-[12.5px] font-semibold tracking-tight">{agent.title}</h1>
          {/* State and step on one quiet line rather than a loud pill beside the
              title. The pill was the largest thing on the card and said the least
              -- the face already says running, continuously. */}
          <p className="mt-[1px] flex items-center gap-1.5 text-[10px] text-white/35">
            <span className={`size-1.5 shrink-0 rounded-full ${tone.dot}`} />
            {tone.label}
            {agent.state === "running" && <> · step {agent.step}</>}
          </p>
        </div>
        <Ghost label="Collapse" onClick={onCollapse}>
          <path d="M2.5 6h7" />
        </Ghost>
        <Ghost
          label="Dismiss"
          onClick={() => void invoke("dismiss_agent", { id: agent.id }).catch(() => {})}
        >
          <path d="M3 3l6 6M9 3l-6 6" />
        </Ghost>
      </header>

      <div className="px-3 pb-2.5">
        {agent.state === "waiting" ? (
          <Question id={agent.id} question={agent.question} />
        ) : (
          <p className="text-[11.5px] leading-snug text-white/70">
            {agent.state === "failed" ? agent.why : agent.status}
          </p>
        )}

        <Plan plan={agent.plan} />
        <Artifacts made={agent.made} />
        <Steps history={agent.history} />
        <Commands ran={agent.ran} />
      </div>

      {/* The footer is separated by a line rather than by space. Stopping is the
          one thing that must never be hunted for, and a rule says "this is not
          part of the report" more cheaply than a gap does. */}
      {agent.state === "running" && (
        <div className="flex items-center gap-2.5 border-t border-white/[0.07] px-3 py-2">
          <div className="h-[3px] min-w-0 flex-1 overflow-hidden rounded-full bg-white/[0.08]">
            <div
              className="h-full rounded-full bg-[#0a84ff] transition-[width] duration-500 ease-out"
              // A floor, so the bar reads as a bar on the first step instead of as
              // an empty groove somebody forgot to fill.
              style={{ width: `${Math.max(4, progress(agent) * 100)}%` }}
            />
          </div>
          <span className="shrink-0 text-[9.5px] text-white/30">cursor in use</span>
          <button
            onClick={() => void invoke("stop_agent", { id: agent.id }).catch(() => {})}
            className="shrink-0 rounded-lg bg-white/[0.08] px-2.5 py-[3px] text-[11px] font-medium transition-colors duration-150 hover:bg-[#ff5f57] hover:text-white"
          >
            Stop
          </button>
        </div>
      )}
    </div>
  );
}

/** A header button: no chrome until it is pointed at. */
function Ghost({
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
      aria-label={label}
      title={label}
      className="grid size-[20px] shrink-0 place-items-center rounded-md text-white/35 transition-colors duration-150 hover:bg-white/[0.09] hover:text-white"
    >
      <svg viewBox="0 0 12 12" className="size-2.5" fill="none" stroke="currentColor" strokeWidth={1.8} strokeLinecap="round">
        {children}
      </svg>
    </button>
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
