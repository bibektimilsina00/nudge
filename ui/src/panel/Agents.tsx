import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Artifacts, Commands, Plan, Steps, hue, type Agent } from "../Agent";
import { Section } from "./parts";
import { Face } from "../components/Face";

/**
 * The Agents tab: what Nudge is working on, or an explanation of what would be
 * here. The empty state is still correct -- it is just no longer the only thing.
 */
export function Agents() {
  const [agents, setAgents] = useState<Agent[]>([]);
  // Which past run is open. One at a time: two open tiles in a two-column grid
  // leaves the column heights fighting, and nobody compares two histories.
  const [opened, setOpened] = useState<number | null>(null);

  useEffect(() => {
    void invoke<Agent[]>("agents").then(setAgents);
    const sub = listen<Agent[]>("agents", (e) => setAgents(e.payload));
    return () => void sub.then((un) => un());
  }, []);

  // Filtered by state rather than by excluding the other list -- `includes` on a
  // discriminated union loses the narrowing and the card cannot read `why`.
  const live = agents.filter((a) => a.state === "running" || a.state === "waiting");
  const past = agents
    .filter((a) => a.state !== "running" && a.state !== "waiting")
    // Newest first. A history is read from the end that just happened.
    .sort((a, b) => b.started - a.started);

  // Grouped by the day it ran, because that is how somebody looks for one: they
  // remember roughly when, never what it was called.
  const days = new Map<string, Agent[]>();
  for (const a of past) {
    const key = whichDay(a.started);
    const bucket = days.get(key);
    if (bucket) bucket.push(a);
    else days.set(key, [a]);
  }

  return (
    <div className="flex-1 overflow-y-auto px-3 pb-3">
      {live.length > 0 ? (
        <Section title="Running">
          {live.map((a) => (
            <Card key={a.id} agent={a} open />
          ))}
        </Section>
      ) : (
        // Nothing running, and that is worth saying even when there is history
        // underneath. A list of finished work with no explanation reads as the
        // whole of what this tab is; the empty state is what says the tab is
        // about something that happens, not something that happened.
        <Empty compact={days.size > 0} />
      )}
      {[...days].map(([day, runs]) => (
        <Section key={day} title={day} grid>
          {runs.map((a) => (
            <Tile key={a.id} agent={a} open={opened === a.id} onOpen={() =>
              setOpened((o) => (o === a.id ? null : a.id))
            } />
          ))}
        </Section>
      ))}
    </div>
  );
}

/**
 * Which day a run belongs under.
 *
 * Today and Yesterday by name, because that is what people say, and a date after
 * that. The year is left off inside the current one -- "3 March" is how somebody
 * reads a date they already know the year of.
 *
 * A run from before timestamps were kept has none, and is filed as Earlier
 * rather than under the first second of 1970.
 */
function whichDay(at: number) {
  if (!at) return "Earlier";
  const when = new Date(at);
  const midnight = (d: Date) => new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
  const days = Math.round((midnight(new Date()) - midnight(when)) / 86_400_000);
  if (days <= 0) return "Earlier today";
  if (days === 1) return "Yesterday";
  return when.toLocaleDateString(undefined, {
    day: "numeric",
    month: "long",
    ...(when.getFullYear() === new Date().getFullYear() ? {} : { year: "numeric" }),
  });
}

/**
 * A finished run, as a tile.
 *
 * Two across rather than a list, because a past run is recognised by its shape
 * and its colour long before anybody reads the title -- and a column of
 * full-width rows makes every run look equally like the one being searched for.
 *
 * Opening one spans it across both columns rather than growing it in place. A
 * tile that gets taller than its neighbour drags the grid around it; one that
 * takes the whole row simply becomes a card for as long as it is open.
 */
function Tile({
  agent,
  open,
  onOpen,
}: {
  agent: Agent;
  open: boolean;
  onOpen: () => void;
}) {
  const detail =
    agent.plan.length > 0 ||
    agent.made.length > 0 ||
    agent.ran.length > 0 ||
    agent.history.length > 0;

  if (open) {
    return (
      <div className="col-span-2 rounded-card bg-raise p-2.5 hairline">
        <button
          onClick={onOpen}
          className="flex w-full items-center gap-2 text-left"
          aria-expanded
        >
          <Face state={agent.state} step={agent.step} size={18} hue={hue(agent.id)} />
          <h3 className="min-w-0 flex-1 truncate text-[12px] font-semibold">{agent.title}</h3>
          <span className="shrink-0 text-[9.5px] text-ink-3">{at(agent.started)}</span>
          <svg viewBox="0 0 12 12" className="size-2.5 shrink-0 rotate-90 text-ink-3" fill="none" stroke="currentColor" strokeWidth={1.6} strokeLinecap="round" strokeLinejoin="round">
            <path d="M4.5 2.5 8 6l-3.5 3.5" />
          </svg>
        </button>
        <p className="mt-1 text-[10.5px] leading-snug text-ink-2">
          {agent.state === "failed" ? agent.why : agent.status}
        </p>
        <Plan plan={agent.plan} />
        <Artifacts made={agent.made} />
        <Steps history={agent.history} />
        <Commands ran={agent.ran} />
      </div>
    );
  }

  return (
    <button
      onClick={() => detail && onOpen()}
      className="flex h-[92px] flex-col items-start rounded-card bg-raise p-2.5 text-left transition-colors duration-150 hairline hover:bg-raise-hi"
    >
      <Face state={agent.state} step={agent.step} size={22} hue={hue(agent.id)} />
      {/* Two lines then clipped, so every tile is the same height whatever it was
          asked to do -- a grid of ragged boxes is harder to scan than a list. */}
      <h3 className="mt-1.5 line-clamp-2 w-full text-[11.5px] leading-tight font-semibold">
        {agent.title}
      </h3>
      <span className="mt-auto flex w-full items-center gap-1.5 text-[9.5px] text-ink-3">
        {at(agent.started)}
        <span className="capitalize">· {agent.state}</span>
        {agent.made.length > 0 && (
          <span className="ml-auto shrink-0">
            {agent.made.length} file{agent.made.length === 1 ? "" : "s"}
          </span>
        )}
      </span>
    </button>
  );
}

/**
 * One run.
 *
 * Shut by default once it is over, open while it is happening. A finished run is
 * a line in a list until somebody wants it -- what it made, what it ran, what it
 * did -- and showing all of that for every run turns a day's history into a wall.
 */
function Card({ agent, open: alwaysOpen }: { agent: Agent; open?: boolean }) {
  const [open, setOpen] = useState(alwaysOpen ?? false);
  const detail =
    agent.plan.length > 0 ||
    agent.made.length > 0 ||
    agent.ran.length > 0 ||
    agent.history.length > 0;
  return (
    <div className="rounded-card bg-raise p-2.5 hairline">
      {/* The whole row opens it, not a chevron somebody has to find. */}
      <div
        onClick={() => detail && !alwaysOpen && setOpen((o) => !o)}
        className="flex items-center gap-2"
      >
        {/* The same face, in the same colour it had while it was running. It is
            how somebody recognises a run they watched an hour ago -- the title is
            a sentence and the colour is a glance. */}
        <Face state={agent.state} step={agent.step} size={18} hue={hue(agent.id)} />
        <h3 className="min-w-0 flex-1 truncate text-[12px] font-semibold">{agent.title}</h3>
        <span className="shrink-0 text-[9px] text-ink-3">{at(agent.started)}</span>
        <span className="shrink-0 text-[9.5px] text-ink-3 capitalize">
          {agent.state}
        </span>
        {detail && !alwaysOpen && (
          <svg
            viewBox="0 0 12 12"
            className={`size-2.5 shrink-0 text-ink-3 transition-transform duration-200 ease-out ${open ? "rotate-90" : ""}`}
            fill="none"
            stroke="currentColor"
            strokeWidth={1.6}
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <path d="M4.5 2.5 8 6l-3.5 3.5" />
          </svg>
        )}
        <button
          onClick={(e) => {
            e.stopPropagation();
            void invoke("dismiss_agent", { id: agent.id });
          }}
          aria-label="Dismiss"
          className="shrink-0 text-[11px] text-ink-3 transition-colors duration-150 hover:text-ink-2"
        >
          ×
        </button>
      </div>
      <p className="mt-1 line-clamp-2 text-[10.5px] leading-snug text-ink-2">
        {agent.state === "failed" ? agent.why : agent.state === "waiting" ? agent.question : agent.status}
      </p>
      {alwaysOpen && (
        <div className="mt-2 h-[2px] overflow-hidden rounded-full bg-raise-hi">
          <div
            className="h-full rounded-full bg-blue transition-[width] duration-500 ease-out"
            style={{ width: `${progress(agent) * 100}%` }}
          />
        </div>
      )}
      {open && (
        <>
          <Plan plan={agent.plan} />
          <Artifacts made={agent.made} />
          <Steps history={agent.history} />
          <Commands ran={agent.ran} />
        </>
      )}
    </div>
  );
}

/** The time of day a run started, or nothing if it is from before these were kept. */
function at(started: number) {
  if (!started) return "";
  return new Date(started).toLocaleTimeString(undefined, {
    hour: "numeric",
    minute: "2-digit",
  });
}

/**
 * How far along, honestly.
 *
 * The plan when there is one; the step budget only as a fallback. A bar creeping
 * toward forty turns tells you how patient the runtime is, not how close the
 * task is -- and mirrors `Agent::progress` in Rust so the card and the tab agree.
 */
function progress(a: Agent) {
  if (a.state === "done") return 1;
  const done = a.plan.filter((t) => t.status === "done").length;
  if (a.plan.length > 0) return Math.min(0.97, done / a.plan.length);
  return Math.min(0.97, a.step / 40);
}

/** `compact` when there is history underneath it and this is a note rather than
 *  the whole page. */
function Empty({ compact }: { compact?: boolean }) {
  return (
    <div
      className={`flex flex-col items-center px-6 text-center ${compact ? "pt-7 pb-6" : "flex-1 pt-8"}`}
    >
      {/* The thing itself, idling. A sparkle said "agents" the way a road sign
          says "deer"; this says it by being one, waiting for something to do --
          which is exactly the state being described. */}
      <Face state="done" step={0} size={compact ? 34 : 64} />
      <h2 className={`font-medium ${compact ? "mt-1.5 text-[11.5px]" : "mt-2.5 text-[12.5px]"}`}>
        {compact ? "Nothing running" : "No agents yet"}
      </h2>
      {!compact && (
        <p className="mt-1 max-w-[300px] text-[10.5px] leading-snug text-ink-3">
          Ask for a whole task — “play a song on YouTube” — and Nudge will go and do
          it, reporting back as it works.
        </p>
      )}
    </div>
  );
}
