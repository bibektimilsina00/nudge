import { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import { type Agent } from "../Agent";

/**
 * The home screen: a place to say what you want.
 *
 * What was here before was a hub -- "Add skills", a keyboard reference, "Add an
 * integration". All setup, arranged as though the thing somebody opens this for
 * is configuring it. Nobody opens an assistant to configure it; the first screen
 * should be the one thing it is for, and everything else is a door off it.
 *
 * So: one field, and it has focus. Typing is the other half of the hotkey --
 * holding a key and talking is faster when your hands are free and impossible
 * when they are not, or when somebody is in the room, or when the task has a
 * path in it that no dictation will spell.
 *
 * The suggestions underneath are the part worth being careful about. Generic
 * examples ("summarise this document") teach nothing, because the interesting
 * question is never what an assistant *can* do -- it is what *yours* can reach.
 * So they are built from what is actually connected: no mail connector, no mail
 * suggestion. An example you can click and watch work is worth a page of
 * description, and one that fails because you never connected Gmail is worse
 * than none.
 */
export function Ask({
  hold,
  onOpenIntegrations,
  onOpenAgents,
}: {
  hold: string;
  onOpenIntegrations: () => void;
  onOpenAgents: () => void;
}) {
  const [text, setText] = useState("");
  const [sending, setSending] = useState(false);
  const [have, setHave] = useState<{ key: string; name: string }[]>([]);
  const [agents, setAgents] = useState<Agent[]>([]);
  const field = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    // Focus on arrival: the panel opens because somebody wants something, and a
    // field they have to click first is a field that asks them to say so twice.
    field.current?.focus();
    void invoke<{ key: string; name: string; connected: boolean }[]>("connections")
      .then((all) => setHave(all.filter((c) => c.connected).map(({ key, name }) => ({ key, name }))))
      .catch(() => {});
    void invoke<Agent[]>("agents").then(setAgents).catch(() => {});
    const sub = listen<Agent[]>("agents", (e) => setAgents(e.payload));
    return () => void sub.then((un) => un());
  }, []);

  // Anything live first, then the most recent finished ones. Three, because the
  // field and its hint own the bottom half and a list that fills the rest is a
  // list nobody reads the end of.
  const recent = useMemo(() => {
    const live = agents.filter((a) => a.state === "running" || a.state === "waiting");
    const past = agents
      .filter((a) => a.state !== "running" && a.state !== "waiting")
      .sort((a, b) => b.started - a.started);
    return [...live, ...past].slice(0, 3);
  }, [agents]);

  const nothing = have.length === 0;
  // Their own names, not keys. Trimmed to three so the sentence stays a
  // sentence -- nineteen connectors listed is a wall, not an invitation.
  const reach = useMemo(() => {
    const names = have.map((h) => h.name).slice(0, 3);
    return names.length > 1
      ? `${names.slice(0, -1).join(", ")} and ${names[names.length - 1]}`
      : (names[0] ?? "");
  }, [have]);

  const send = () => {
    const goal = text.trim();
    if (!goal || sending) return;
    setSending(true);
    void invoke("start", { goal })
      .then(() => setText(""))
      .finally(() => setSending(false));
  };

  return (
    <div className="flex flex-1 flex-col px-3.5 pt-2.5">
      {/* Above the field, because that is where an answer appears. What was
          here was a row of example tasks -- a tutorial, shown forever, to
          somebody who has used the thing a hundred times. This is what actually
          happened instead: the run in progress, or the last few that finished.
          A home screen that shows work beats one that only accepts it. */}
      <div className="min-h-0 flex-1 overflow-y-auto">
        {recent.length > 0 ? (
          <div className="space-y-1">
            {recent.map((a) => (
              <button
                key={a.id}
                onClick={onOpenAgents}
                className="flex w-full items-start gap-2 rounded-lg px-1.5 py-[6px] text-left transition-colors duration-150 hover:bg-raise"
              >
                <span
                  className={[
                    "mt-[5px] size-1.5 shrink-0 rounded-full",
                    a.state === "running" || a.state === "waiting"
                      ? "animate-pulse bg-blue"
                      : a.state === "failed"
                        ? "bg-[#ff5f57]"
                        : "bg-[#30d158]",
                  ].join(" ")}
                />
                <span className="min-w-0 flex-1">
                  <span className="block truncate text-[11.5px] text-ink">{a.title || a.goal}</span>
                  {/* Its own words about what it is doing, or what it ended up
                      saying. Not a state name -- "Done" tells nobody anything
                      they did not already know from the green dot. */}
                  {/* Two lines, clamped. One line threw away the half that
                      matters -- a refusal cut off at "It can be turned on under
                      Allowed to in setti…" has dropped the sentence somebody
                      needed and kept the one they already knew. */}
                  <span className="line-clamp-2 text-[10.5px] leading-snug text-ink-3">
                    {a.state === "failed" ? a.why : a.status}
                  </span>
                </span>
              </button>
            ))}
          </div>
        ) : nothing ? (
          // Nothing connected: the honest empty state is the reason it is empty
          // and the way out of it, not a shrug.
          <button
            onClick={onOpenIntegrations}
            className="w-full rounded-xl bg-raise px-3 py-2.5 text-left transition-colors duration-150 hover:bg-raise-hi hairline"
          >
            <p className="text-[11.5px] text-ink">Nothing is connected yet</p>
            <p className="mt-0.5 text-[10.5px] text-ink-3">
              Nudge can see your screen and run things. Connect your mail, calendar or
              code and it can reach those too.
            </p>
          </button>
        ) : (
          // Connected, and nothing has been asked yet. Says what it can reach,
          // because that is the useful fact and it is different on every machine.
          <p className="px-1 py-1 text-[10.5px] leading-relaxed text-ink-3">
            Ask for anything on your screen, or in {reach}.
          </p>
        )}
      </div>

      {/* The field sits at the foot, where the thing you type into lives in
          every other window that takes a sentence. Above it is where what comes
          back will go. */}
      <div className="pt-2.5 pb-3">
        <div className="relative">
          <textarea
            ref={field}
            value={text}
            onChange={(e) => setText(e.target.value)}
            onKeyDown={(e) => {
              // Enter sends; shift-enter is a newline. A task is usually one
              // line and occasionally several, and the common case should not
              // need a modifier.
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                send();
              }
            }}
            rows={2}
            spellCheck={false}
            placeholder="What do you want done?"
            className="w-full resize-none rounded-xl bg-black/35 px-3 py-2.5 pr-11 text-[13px] leading-snug text-white outline-none hairline placeholder:text-ink-3 focus:inset-ring-[#0a84ff]"
          />
          <button
            onClick={send}
            disabled={!text.trim() || sending}
            aria-label="Start"
            // Sits in the field rather than beside it: the button is the same
            // action as the key, and putting it anywhere else implies otherwise.
            className="absolute right-2 bottom-2 grid size-[26px] place-items-center rounded-lg bg-blue text-white transition-[background-color,opacity] duration-150 hover:bg-blue-hi disabled:bg-raise disabled:text-ink-3"
          >
            <svg viewBox="0 0 14 14" className="size-3.5" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
              <path d="M7 11V3M3.5 6.5 7 3l3.5 3.5" />
            </svg>
          </button>
        </div>

        {/* The other way in, stated once and quietly. */}
        <p className="mt-1.5 text-[10.5px] text-ink-3">
          {hold ? (
            <>
              or hold <Key>{hold}</Key> and say it
            </>
          ) : (
            "Press ⏎ to start"
          )}
        </p>
      </div>
    </div>
  );
}

function Key({ children }: { children: React.ReactNode }) {
  return (
    <kbd className="rounded-[4px] bg-raise px-1 py-px font-sans text-[10px] text-ink-2">
      {children}
    </kbd>
  );
}
