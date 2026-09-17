import { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import { type Agent } from "../Agent";
import { Companion } from "../components/Companion";

/**
 * Home: a greeting, what has happened, and the two ways to ask for more.
 *
 * The previous version was correct and characterless -- three sizes of grey
 * text, a plain box to type in, and the hold-to-talk key mentioned in a footnote
 * under it. Everything the same weight, so nothing was the thing you had come
 * for, and nothing anywhere said that this is a companion rather than a
 * settings sheet with a text field.
 *
 * Three decisions carry this one.
 *
 * **The talk pill is the hero.** Holding a key and speaking is the fastest way
 * to use Nudge and the reason the hotkey exists; it had been demoted to grey
 * small print while a text box nobody asked for took the space. It is a filled
 * control now, with the key drawn on it, and typing is the quieter alternative
 * beside it -- which is the true order of those two.
 *
 * **The companion sits on it.** Nudge is a cat that lives in the notch, and
 * until now it appeared on this screen as a 24px thumbnail in a socket. Resting
 * its paws on the control you hold is the one moment of character the panel
 * gets, and it costs nothing: the artwork already exists and already idles.
 *
 * **The greeting is the only large type.** A panel with no typographic high
 * point reads as a list of settings however it is coloured. One line at 17px
 * establishes that something is addressing you, and everything under it can
 * then be quiet without the whole thing going flat.
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
  const [typing, setTyping] = useState(false);
  const [sending, setSending] = useState(false);
  const [have, setHave] = useState<{ key: string; name: string }[]>([]);
  const [agents, setAgents] = useState<Agent[]>([]);
  const field = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    void invoke<{ key: string; name: string; connected: boolean }[]>("connections")
      .then((all) => setHave(all.filter((c) => c.connected).map(({ key, name }) => ({ key, name }))))
      .catch(() => {});
    void invoke<Agent[]>("agents").then(setAgents).catch(() => {});
    const sub = listen<Agent[]>("agents", (e) => setAgents(e.payload));
    return () => void sub.then((un) => un());
  }, []);

  useEffect(() => {
    if (typing) field.current?.focus();
  }, [typing]);

  const send = () => {
    const goal = text.trim();
    if (!goal || sending) return;
    setSending(true);
    void invoke("start", { goal })
      .then(() => {
        setText("");
        setTyping(false);
      })
      .finally(() => setSending(false));
  };

  // Anything live first, then what finished most recently. Two, not three: the
  // greeting and the control own the ends of the panel, and a list that fills
  // whatever is left is a list nobody reads the end of.
  const recent = useMemo(() => {
    const live = agents.filter((a) => a.state === "running" || a.state === "waiting");
    const past = agents
      .filter((a) => a.state !== "running" && a.state !== "waiting")
      .sort((a, b) => b.started - a.started);
    return [...live, ...past].slice(0, 2);
  }, [agents]);

  const reach = useMemo(() => {
    const names = have.map((h) => h.name).slice(0, 3);
    return names.length > 1
      ? `${names.slice(0, -1).join(", ")} and ${names[names.length - 1]}`
      : (names[0] ?? "");
  }, [have]);

  const busy = agents.some((a) => a.state === "running");

  return (
    <div className="flex flex-1 flex-col px-4 pt-3">
      <h2 className="text-[17px] leading-tight font-semibold tracking-[-0.01em] text-ink">
        {partOfDay()}.
      </h2>
      <p className="mt-0.5 text-[11.5px] leading-snug text-ink-3">
        {busy
          ? "Working on it."
          : reach
            ? `I can see your screen, and reach ${reach}.`
            : "I can see your screen. Tell me what to do with it."}
      </p>

      <div className="mt-2.5 min-h-0 flex-1 space-y-px overflow-y-auto">
        {recent.map((a) => (
          <Ran key={a.id} agent={a} onOpen={onOpenAgents} />
        ))}
        {recent.length === 0 && have.length === 0 && (
          <button
            onClick={onOpenIntegrations}
            className="mt-1 w-full rounded-xl bg-raise px-3 py-2.5 text-left transition-colors duration-150 hover:bg-raise-hi hairline"
          >
            <p className="text-[11.5px] text-ink">Nothing is connected yet</p>
            <p className="mt-0.5 text-[10.5px] leading-snug text-ink-3">
              Connect your mail, calendar or code and I can reach those too.
            </p>
          </button>
        )}
      </div>

      <Dock
        hold={hold}
        typing={typing}
        text={text}
        sending={sending}
        field={field}
        onType={() => setTyping(true)}
        onText={setText}
        onSend={send}
        onStopTyping={() => {
          setTyping(false);
          setText("");
        }}
      />
    </div>
  );
}

/** One thing that ran, or is running. */
function Ran({ agent, onOpen }: { agent: Agent; onOpen: () => void }) {
  const live = agent.state === "running" || agent.state === "waiting";
  return (
    <button
      onClick={onOpen}
      className="flex w-full items-start gap-2.5 rounded-lg px-2 py-[7px] text-left transition-colors duration-150 hover:bg-raise"
    >
      <span
        className={[
          "mt-[5px] size-[7px] shrink-0 rounded-full",
          live
            ? "animate-pulse bg-blue"
            : agent.state === "failed"
              ? "bg-[#ff5f57]"
              : "bg-[#30d158]",
        ].join(" ")}
      />
      <span className="min-w-0 flex-1">
        {/* Weight, not just colour. Three greys at one size is why the old list
            read as a paragraph rather than as rows. */}
        <span className="block truncate text-[12px] font-medium text-ink">
          {agent.title || agent.goal}
        </span>
        <span className="line-clamp-2 text-[11px] leading-snug text-ink-3">
          {agent.state === "failed" ? agent.why : agent.status}
        </span>
      </span>
    </button>
  );
}

/**
 * The two ways in, and the cat sitting on them.
 *
 * Talking is primary and typing is the alternative, so one is a filled control
 * and the other is a quiet button that becomes a field when it is wanted. The
 * field replacing the pill rather than sitting beside it keeps one clear action
 * at the foot instead of two competing ones.
 */
function Dock({
  hold,
  typing,
  text,
  sending,
  field,
  onType,
  onText,
  onSend,
  onStopTyping,
}: {
  hold: string;
  typing: boolean;
  text: string;
  sending: boolean;
  field: React.RefObject<HTMLTextAreaElement | null>;
  onType: () => void;
  onText: (s: string) => void;
  onSend: () => void;
  onStopTyping: () => void;
}) {
  return (
    <div className="relative pt-7 pb-3.5">
      {/* Paws over the control. Behind it in the stack, so the pill's edge cuts
          across the body and it reads as leaning on the thing rather than
          floating in front of it. */}
      {!typing && (
        <span className="pointer-events-none absolute bottom-[42px] left-1/2 z-0 -translate-x-1/2 scale-[0.58] opacity-95">
          <Companion mode="idle" anchored />
        </span>
      )}

      {typing ? (
        <div className="relative">
          <textarea
            ref={field}
            value={text}
            onChange={(e) => onText(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                onSend();
              }
              // Escape goes back to the pill rather than closing the panel,
              // because the thing in front of you is the field.
              if (e.key === "Escape") {
                e.preventDefault();
                e.stopPropagation();
                onStopTyping();
              }
            }}
            rows={2}
            spellCheck={false}
            placeholder="What do you want done?"
            className="w-full resize-none rounded-xl bg-black/45 px-3 py-2.5 pr-11 text-[12.5px] leading-snug text-white outline-none hairline placeholder:text-ink-3 focus:inset-ring-[#0a84ff]"
          />
          <button
            onClick={onSend}
            disabled={!text.trim() || sending}
            aria-label="Start"
            className="absolute right-2 bottom-2 grid size-[26px] place-items-center rounded-lg bg-blue text-white transition-colors duration-150 hover:bg-blue-hi disabled:bg-raise disabled:text-ink-3"
          >
            <svg viewBox="0 0 14 14" className="size-3.5" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
              <path d="M7 11V3M3.5 6.5 7 3l3.5 3.5" />
            </svg>
          </button>
        </div>
      ) : (
        <div className="relative z-10 flex items-center gap-2">
          <div className="flex h-[38px] flex-1 items-center justify-center gap-2 rounded-full bg-raise-hi text-[12px] font-medium text-ink hairline">
            <Mic />
            {hold ? (
              <span>
                Hold <Key>{hold}</Key> to talk
              </span>
            ) : (
              <span>Hold your hotkey to talk</span>
            )}
          </div>
          <button
            onClick={onType}
            aria-label="Type instead"
            className="grid h-[38px] w-[44px] shrink-0 place-items-center rounded-full bg-raise text-ink-2 transition-colors duration-150 hover:bg-raise-hi hover:text-ink"
          >
            <Keyboard />
          </button>
        </div>
      )}
    </div>
  );
}

function partOfDay() {
  const h = new Date().getHours();
  if (h < 5) return "Late one";
  if (h < 12) return "Morning";
  if (h < 18) return "Afternoon";
  return "Evening";
}

function Key({ children }: { children: React.ReactNode }) {
  return (
    <kbd className="rounded-[4px] bg-black/35 px-[5px] py-px font-sans text-[10.5px] text-ink-2">
      {children}
    </kbd>
  );
}

function Mic() {
  return (
    <svg viewBox="0 0 16 16" className="size-[13px]" fill="none" stroke="currentColor" strokeWidth={1.5} strokeLinecap="round" strokeLinejoin="round">
      <rect x="6" y="2" width="4" height="7" rx="2" />
      <path d="M4 7.5a4 4 0 0 0 8 0M8 11.5V14" />
    </svg>
  );
}

function Keyboard() {
  return (
    <svg viewBox="0 0 18 16" className="size-[15px]" fill="none" stroke="currentColor" strokeWidth={1.4} strokeLinecap="round" strokeLinejoin="round">
      <rect x="1.5" y="3.5" width="15" height="9" rx="2" />
      <path d="M4.5 6.5h0M7 6.5h0M9.5 6.5h0M12 6.5h0M5.5 9.5h7" />
    </svg>
  );
}
