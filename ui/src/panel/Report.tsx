import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export type Kind = "bug" | "idea";

const COPY: Record<Kind, { title: string; hint: string; placeholder: string }> = {
  bug: {
    title: "Report a bug",
    hint: "What happened?",
    // Two examples rather than instructions. A placeholder that says "please
    // include steps to reproduce" is a form asking for homework; showing what a
    // one-line report looks like gets one-line reports.
    placeholder: "The companion vanished when I opened Mission Control…",
  },
  idea: {
    title: "Request a feature",
    hint: "What would you want it to do?",
    placeholder: "Let me pin an agent's result so it survives a restart…",
  },
};

/**
 * The compose sheet, which is the entire feature.
 *
 * Everything here is in service of one number: the share of people who notice
 * something and go on to say so. Every step between those two loses some of them
 * -- opening a browser, signing in, finding the repository, picking a template --
 * and what survives is reports from people invested enough to run the gauntlet,
 * which is the opposite of who is most worth hearing from.
 *
 * So: a box, and a Send. No title field, no category, no severity, no minimum
 * length. The version and the OS are attached without being asked for, because
 * they are the two things nobody thinks to include and no report is actionable
 * without.
 *
 * The only optional extra is a picture, because a bug report with a screenshot is
 * worth about five without one -- and it is offered three ways (take one, paste
 * one, drag one in) since the cheapest one differs per person and per bug.
 */
export function Report({ kind, onBack }: { kind: Kind; onBack: () => void }) {
  const [text, setText] = useState("");
  const [image, setImage] = useState<string | null>(null);
  const [state, setState] = useState<"writing" | "sending" | "sent">("writing");
  const [problem, setProblem] = useState<string | null>(null);
  const [shooting, setShooting] = useState(false);
  const box = useRef<HTMLTextAreaElement>(null);

  // Focused on arrival. They pressed a button that says what this is for; making
  // them click once more to start typing is a step that buys nothing.
  useEffect(() => box.current?.focus(), []);

  const send = async () => {
    if (state !== "writing" || !text.trim()) return;
    setState("sending");
    setProblem(null);
    try {
      await invoke("send_report", { kind, text, image });
      setState("sent");
      // Long enough to read, short enough that nobody is left looking at a tick.
      window.setTimeout(onBack, 1400);
    } catch (e) {
      setState("writing");
      setProblem(String(e));
    }
  };

  const shoot = async () => {
    setShooting(true);
    try {
      setImage(await invoke<string>("shot_for_report"));
    } catch (e) {
      setProblem(String(e));
    } finally {
      setShooting(false);
    }
  };

  // Paste is how most people already move a screenshot, so it should simply work
  // wherever they happen to be focused when they try it.
  const onPaste = (e: React.ClipboardEvent) => {
    const file = [...e.clipboardData.files].find((f) => f.type.startsWith("image/"));
    if (file) {
      e.preventDefault();
      read(file);
    }
  };

  const read = (file: File) => {
    const r = new FileReader();
    r.onload = () => setImage(String(r.result));
    r.readAsDataURL(file);
  };

  if (state === "sent") return <Sent kind={kind} />;

  return (
    <div className="flex min-h-0 flex-1 flex-col" onPaste={onPaste}>
      <header className="flex items-center gap-2.5 px-3 pt-2.5 pb-2">
        <button
          onClick={onBack}
          aria-label="Back"
          className="grid size-[21px] shrink-0 place-items-center rounded-full bg-raise text-ink-2 transition-colors duration-150 hover:bg-raise-hi hover:text-white"
        >
          <svg viewBox="0 0 16 16" className="size-3" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
            <path d="M9.5 3.5 5 8l4.5 4.5" />
          </svg>
        </button>
        <h2 className="text-[13px] font-semibold tracking-tight">{COPY[kind].title}</h2>
      </header>

      <div className="min-h-0 flex-1 overflow-y-auto px-3 pb-3">
        <p className="mb-1.5 px-0.5 text-[10.5px] text-ink-3">{COPY[kind].hint}</p>

        <textarea
          ref={box}
          value={text}
          onChange={(e) => setText(e.target.value)}
          onKeyDown={(e) => {
            // Send on ⌘↵, because this is a box people will type one line into
            // and reaching for the mouse afterwards is the last bit of friction
            // left. Plain Enter stays a newline -- a report cut off mid-sentence
            // by a stray keystroke is worse than no shortcut.
            if (e.key === "Enter" && e.metaKey) void send();
            if (e.key === "Escape") onBack();
          }}
          placeholder={COPY[kind].placeholder}
          spellCheck
          className="h-[104px] w-full resize-none rounded-card bg-raise p-2.5 text-[12px] leading-relaxed text-white outline-none hairline placeholder:text-ink-3 focus:bg-raise-hi"
        />

        {image ? (
          <Attached src={image} onDrop={() => setImage(null)} />
        ) : (
          <div className="mt-2 flex items-center gap-2">
            <Small onClick={shoot} busy={shooting}>
              <Camera />
              {shooting ? "Taking it…" : "Take a screenshot"}
            </Small>
            <label className="cursor-pointer">
              <input
                type="file"
                accept="image/*"
                className="hidden"
                onChange={(e) => e.target.files?.[0] && read(e.target.files[0])}
              />
              <Small as="span">
                <Clip />
                Choose an image
              </Small>
            </label>
          </div>
        )}
        {!image && (
          <p className="mt-1.5 px-0.5 text-[10px] text-ink-3">
            Or paste one — Nudge steps out of the shot when it takes its own.
          </p>
        )}

        {problem && (
          <p className="mt-2 rounded-card bg-[#ff453a]/12 p-2 text-[10.5px] leading-snug text-[#ff8a80]">
            {problem}
          </p>
        )}
      </div>

      <footer className="flex items-center gap-2 border-t border-line px-3 py-2.5">
        {/* Said plainly and up front. People are more willing to send something
            when they can see exactly what goes with it, and less willing when
            they have to guess. */}
        <p className="min-w-0 flex-1 text-[10px] leading-snug text-ink-3">
          Sends your message, your version and your OS. Nothing else.
        </p>
        <button
          onClick={send}
          disabled={!text.trim() || state === "sending"}
          className="h-[26px] shrink-0 rounded-control bg-blue px-3.5 text-[12px] font-medium text-white transition-colors duration-150 hover:bg-blue-hi active:scale-[0.98] disabled:bg-raise disabled:text-ink-3"
        >
          {state === "sending" ? "Sending…" : "Send"}
        </button>
      </footer>
    </div>
  );
}

/** The thank-you, which is short because the job is done. */
function Sent({ kind }: { kind: Kind }) {
  return (
    <div className="flex flex-1 flex-col items-center justify-center px-6 text-center">
      <span className="grid size-10 place-items-center rounded-full bg-blue/15 text-blue">
        <svg viewBox="0 0 16 16" className="size-[18px]" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
          <path d="m3.5 8.5 3 3 6-6.5" />
        </svg>
      </span>
      <h3 className="mt-2.5 text-[12.5px] font-semibold">Sent — thank you</h3>
      <p className="mt-1 max-w-[240px] text-[10.5px] leading-snug text-ink-2">
        {kind === "bug"
          ? "Someone will actually read it."
          : "Someone will actually read it. The good ones get built."}
      </p>
    </div>
  );
}

/** The picture, with the one control it needs. */
function Attached({ src, onDrop }: { src: string; onDrop: () => void }) {
  return (
    <div className="group relative mt-2 overflow-hidden rounded-card hairline">
      <img src={src} alt="Attached screenshot" className="block max-h-[150px] w-full object-cover object-top" />
      <button
        onClick={onDrop}
        aria-label="Remove image"
        className="absolute top-1.5 right-1.5 grid size-[22px] place-items-center rounded-full bg-black/65 text-white/85 opacity-0 backdrop-blur-sm transition-opacity duration-150 group-hover:opacity-100 hover:text-white focus-visible:opacity-100"
      >
        <svg viewBox="0 0 16 16" className="size-[11px]" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round">
          <path d="M4 4l8 8M12 4l-8 8" />
        </svg>
      </button>
    </div>
  );
}

function Small({
  children,
  onClick,
  busy,
  as: Tag = "button",
}: {
  children: React.ReactNode;
  onClick?: () => void;
  busy?: boolean;
  as?: "button" | "span";
}) {
  return (
    <Tag
      onClick={onClick}
      className={[
        "inline-flex h-[26px] items-center gap-1.5 rounded-control bg-raise px-2.5 text-[11.5px] text-ink-2",
        "transition-colors duration-150 hover:bg-raise-hi hover:text-white active:scale-[0.98] hairline",
        busy ? "pointer-events-none opacity-60" : "",
      ].join(" ")}
    >
      {children}
    </Tag>
  );
}

function Camera() {
  return (
    <svg viewBox="0 0 16 16" className="size-[13px]" fill="none" stroke="currentColor" strokeWidth={1.5} strokeLinejoin="round">
      <path d="M2 5.5h2.2l1-1.6h5.6l1 1.6H14v7H2z" />
      <circle cx="8" cy="9" r="2.3" />
    </svg>
  );
}

function Clip() {
  return (
    <svg viewBox="0 0 16 16" className="size-[13px]" fill="none" stroke="currentColor" strokeWidth={1.5} strokeLinecap="round" strokeLinejoin="round">
      <path d="M11.5 7.5 7 12a2.5 2.5 0 0 1-3.5-3.5l5-5a1.7 1.7 0 0 1 2.4 2.4l-5 5" />
    </svg>
  );
}
