import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Artifacts, Commands, Plan, type Agent } from "../Agent";
import { Section } from "./parts";
import { Face } from "../components/Face";

/**
 * The Agents tab: what Nudge is working on, or an explanation of what would be
 * here. The empty state is still correct -- it is just no longer the only thing.
 */
export function Agents() {
  const [agents, setAgents] = useState<Agent[]>([]);

  useEffect(() => {
    void invoke<Agent[]>("agents").then(setAgents);
    const sub = listen<Agent[]>("agents", (e) => setAgents(e.payload));
    return () => void sub.then((un) => un());
  }, []);

  if (agents.length === 0) return <Empty />;

  // Filtered by state rather than by excluding the other list -- `includes` on a
  // discriminated union loses the narrowing and the card cannot read `why`.
  const live = agents.filter((a) => a.state === "running" || a.state === "waiting");
  const past = agents.filter((a) => a.state !== "running" && a.state !== "waiting");

  return (
    <div className="flex-1 overflow-y-auto px-3 pb-3">
      {live.length > 0 && (
        <Section title="Running">
          {live.map((a) => (
            <Card key={a.id} agent={a} />
          ))}
        </Section>
      )}
      {past.length > 0 && (
        <Section title="Finished">
          {past.map((a) => (
            <Card key={a.id} agent={a} />
          ))}
        </Section>
      )}
    </div>
  );
}

function Card({ agent }: { agent: Agent }) {
  const running = agent.state === "running" || agent.state === "waiting";
  return (
    <div
      className={[
        "rounded-xl p-2.5 inset-ring-1",
        running
          ? "bg-[#141824] inset-ring-white/[0.12]"
          : "bg-[#1e1e1e] inset-ring-white/[0.09]",
      ].join(" ")}
    >
      <div className="flex items-center gap-2">
        <h3 className="min-w-0 flex-1 truncate text-[12px] font-semibold">{agent.title}</h3>
        <span className="shrink-0 text-[9px] tracking-wide text-white/35 uppercase">
          {agent.state}
        </span>
        <button
          onClick={() => void invoke("dismiss_agent", { id: agent.id })}
          aria-label="Dismiss"
          className="shrink-0 text-[11px] text-white/30 transition-colors duration-150 hover:text-white/70"
        >
          ×
        </button>
      </div>
      <p className="mt-1 line-clamp-2 text-[10.5px] leading-snug text-white/45">
        {agent.state === "failed" ? agent.why : agent.state === "waiting" ? agent.question : agent.status}
      </p>
      {running && (
        <div className="mt-2 h-[2px] overflow-hidden rounded-full bg-white/10">
          <div
            className="h-full rounded-full bg-[#0a84ff] transition-[width] duration-500 ease-out"
            style={{ width: `${progress(agent) * 100}%` }}
          />
        </div>
      )}
      <Plan plan={agent.plan} />
      <Artifacts made={agent.made} />
      <Commands ran={agent.ran} />
    </div>
  );
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

function Empty() {
  return (
    <div className="flex flex-1 flex-col items-center px-6 pt-6 text-center">
      {/* The thing itself, idling. A drawing of a sparkle said "agents" the way
          a road sign says "deer" -- this says it by being one, sitting there
          waiting for something to do, which is exactly the state being
          described. */}
      <Face state="done" step={0} size={64} />
      <h2 className="mt-2.5 text-[12.5px] font-medium">No agents yet</h2>
      <p className="mt-1 max-w-[300px] text-[10.5px] leading-snug text-white/35">
        Ask for a whole task — “play a song on YouTube” — and Nudge will go and do
        it, reporting back as it works.
      </p>
    </div>
  );
}
