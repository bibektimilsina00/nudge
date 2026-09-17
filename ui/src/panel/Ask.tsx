import { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

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
export function Ask({ hold, onOpenIntegrations }: { hold: string; onOpenIntegrations: () => void }) {
  const [text, setText] = useState("");
  const [sending, setSending] = useState(false);
  const [have, setHave] = useState<string[]>([]);
  const field = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    // Focus on arrival: the panel opens because somebody wants something, and a
    // field they have to click first is a field that asks them to say so twice.
    field.current?.focus();
    void invoke<{ key: string; connected: boolean }[]>("connections")
      .then((all) => setHave(all.filter((c) => c.connected).map((c) => c.key)))
      .catch(() => {});
  }, []);

  const send = () => {
    const goal = text.trim();
    if (!goal || sending) return;
    setSending(true);
    void invoke("start", { goal })
      .then(() => setText(""))
      .finally(() => setSending(false));
  };

  const tries = useMemo(() => suggestions(have), [have]);

  return (
    <div className="flex flex-1 flex-col px-3.5 pt-2">
      <div className="relative">
        <textarea
          ref={field}
          value={text}
          onChange={(e) => setText(e.target.value)}
          onKeyDown={(e) => {
            // Enter sends; shift-enter is a newline. A task is usually one line
            // and occasionally several, and the common case should not need a
            // modifier.
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
      {hold ? (
        <p className="mt-1.5 text-[10.5px] text-ink-3">
          or hold <Key>{hold}</Key> and say it
        </p>
      ) : (
        <p className="mt-1.5 text-[10.5px] text-ink-3">Press ⏎ to start</p>
      )}

      <div className="mt-auto pt-3 pb-3.5">
        {tries.length > 0 ? (
          <>
            <p className="mb-1.5 text-[10.5px] text-ink-3">Try</p>
            <div className="flex flex-wrap gap-1.5">
              {tries.map((t) => (
                <button
                  key={t}
                  onClick={() => {
                    setText(t);
                    field.current?.focus();
                  }}
                  // Filled in rather than sent. A suggestion is a starting point
                  // and most of them want a word changed before they are what
                  // somebody meant.
                  className="rounded-full bg-raise px-2.5 py-[5px] text-[10.5px] text-ink-2 transition-colors duration-150 hover:bg-raise-hi hover:text-white"
                >
                  {t}
                </button>
              ))}
            </div>
          </>
        ) : (
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
        )}
      </div>
    </div>
  );
}

/**
 * Examples drawn from what is connected, so every one of them works.
 *
 * Ordered by how ordinary the task is rather than how impressive, because the
 * first one somebody tries decides whether they try a second.
 */
function suggestions(have: string[]): string[] {
  const has = (...keys: string[]) => keys.some((k) => have.includes(k));
  const out: string[] = [];
  if (has("gmail", "google")) out.push("Summarise my unread mail");
  if (has("google_calendar", "google")) out.push("What's on my calendar today?");
  if (has("github")) out.push("What changed in my repo this week?");
  if (has("google_tasks")) out.push("What's on my task list?");
  if (has("slack")) out.push("Catch me up on Slack");
  if (has("google_sheets", "google")) out.push("Make a sheet of this week's spend");
  // Always last, and always available: the one thing that needs nothing
  // connected at all, because it is what Nudge was before any of this.
  out.push("Open the settings pane for my display");
  return out.slice(0, 4);
}

function Key({ children }: { children: React.ReactNode }) {
  return (
    <kbd className="rounded-[4px] bg-raise px-1 py-px font-sans text-[10px] text-ink-2">
      {children}
    </kbd>
  );
}
