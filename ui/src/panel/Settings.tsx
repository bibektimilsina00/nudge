import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Choice, Page, Row, Section, Toggle } from "./parts";
import * as I from "./icons";
import { LOOKS, DEFAULT_LOOK } from "../companions";
import { Companion } from "../components/Companion";

type VoiceMode = "off" | "system" | "gemini";
type Allowed = { key: string; label: string; about: string; on: boolean };
type Pick = { key: string; label: string; about: string };
type Brain = {
  provider: string;
  model: string;
  think: string;
  workspace: string;
  providers: Pick[];
  thinks: Pick[];
};

/** Which page of settings is open. */
type Where = "root" | "allowed" | "model" | "voice" | "tools" | "screen";

/**
 * The settings sheet.
 *
 * Two things were wrong with it and they had the same cause.
 *
 * It was flat: every section of every setting on one scroll, so the three grants
 * that can let a model run any command on your machine sat in the same visual
 * register as which microphone is selected. A list that long is one nobody reads
 * to the end, and the things worth reading are the ones that get skipped.
 *
 * And nothing in it could be *chosen*. A row either showed a value with no way to
 * change it, or changed it by cycling on click -- which is the worst of both,
 * because you cannot see the alternatives, cannot go back without going all the
 * way round, and cannot tell a three-state setting from a button.
 *
 * So it is pages now, and the pages are made of pickers. The root says what each
 * page currently holds, so the common case -- checking rather than changing -- is
 * answered without opening anything.
 *
 * One page per thing somebody would go looking for, which is not the same as one
 * page per part of the code. Which model answers and which microphone it hears
 * you through were together because both are "the model" from the inside; from
 * the outside they are unrelated, and nobody adjusting their microphone wants to
 * scroll past a choice that costs money.
 */
export function Settings({
  onIntegrations,
  onSkills,
  onReport,
}: {
  onIntegrations: () => void;
  onSkills: () => void;
  onReport: (kind: "bug" | "idea") => void;
}) {
  const [where, setWhere] = useState<Where>("root");
  const [voice, setVoice] = useState<VoiceMode>("system");
  const [mic, setMic] = useState("…");
  const [version, setVersion] = useState("");
  const [allowed, setAllowed] = useState<Allowed[]>([]);
  const [brain, setBrain] = useState<Brain | null>(null);
  const [servers, setServers] = useState<[string, string][]>([]);
  const [problem, setProblem] = useState<string | null>(null);
  const [look, setLook] = useState(DEFAULT_LOOK);

  useEffect(() => {
    void invoke<VoiceMode>("voice_mode").then(setVoice);
    void invoke<string>("microphone").then(setMic);
    void invoke<string>("version").then(setVersion);
    void invoke<Allowed[]>("reach").then(setAllowed);
    void invoke<Brain>("brain").then(setBrain);
    void invoke<string>("look").then(setLook);
    // Servers connect in the background long after this mounts -- `npx` can spend
    // a minute fetching one it has never run -- so this looks again rather than
    // showing "starting…" forever to somebody who opened settings early.
    const look = () => void invoke<[string, string][]>("servers").then(setServers);
    look();
    const again = window.setInterval(look, 2000);
    return () => window.clearInterval(again);
  }, []);

  const allow = (key: string, on: boolean) => {
    setAllowed((a) => a.map((g) => (g.key === key ? { ...g, on } : g)));
    void invoke("set_reach", { key, on });
  };

  // Asked for, then confirmed. A provider that cannot start -- a missing key is
  // the usual one -- must leave the working one in place and say why, so the
  // picker only moves once Rust agrees it moved.
  const retune = (change: { provider?: string; think?: string }) => {
    setProblem(null);
    void invoke("retune", change)
      .then(() => invoke<Brain>("brain").then(setBrain))
      .catch((e) => setProblem(String(e)));
  };

  const open = allowed.filter((g) => g.on).length;

  if (where === "allowed") {
    return (
      <Page title="Allowed to" onBack={() => setWhere("root")}>
        <p className="px-0.5 pt-1 pb-2.5 text-[10.5px] leading-snug text-ink-3">
          Off, Nudge reads the screen, runs commands that only look, and fetches
          pages. Each of these lifts one of those limits for as long as it is on.
        </p>
        <div className="space-y-2">
          {allowed.map((g) => (
            <Row
              key={g.key}
              icon={<I.Bolt />}
              label={g.label}
              sub={g.about}
              trailing={<Toggle on={g.on} onChange={(v) => allow(g.key, v)} />}
            />
          ))}
        </div>
        {brain && (
          <Section title="Works inside">
            <Row
              icon={<I.Grid />}
              label={short(brain.workspace)}
              sub="What it runs and writes stays here. Set in config.toml."
            />
          </Section>
        )}
      </Page>
    );
  }

  if (where === "model" && brain) {
    return (
      <Page title="Model" onBack={() => setWhere("root")}>
        {problem && (
          <p className="mt-1 mb-2 rounded-xl bg-[#ff453a]/12 p-2 text-[10.5px] leading-snug text-[#ff8a80]">
            {problem}
          </p>
        )}
        <Section title="Model">
          <Choice
            options={brain.providers}
            value={brain.provider}
            onChange={(provider) => retune({ provider })}
          />
          <p className="px-0.5 pt-1.5 text-[10px] text-ink-3">
            Using {brain.model}. Name a different one in config.toml.
          </p>
        </Section>

        <Section title="Thinking before answering">
          <Choice
            options={brain.thinks}
            value={brain.think}
            onChange={(think) => retune({ think })}
          />
          {/* The one number worth printing in a settings screen: thinking bills
              at the output rate, five times input, so this outweighs the model. */}
          <p className="px-0.5 pt-1.5 text-[10px] leading-snug text-ink-3">
            Thinking is charged at five times the rate of what you send it.
          </p>
        </Section>

      </Page>
    );
  }

  if (where === "voice") {
    return (
      <Page title="Voice" onBack={() => setWhere("root")}>
        <Section title="Speaks back">
          <Choice
            options={[
              { key: "off", label: "Off", about: "Nothing aloud." },
              { key: "system", label: "System", about: "macOS say. Free, offline, instant." },
              { key: "gemini", label: "Natural", about: "Much better. A round trip and a charge per step." },
            ]}
            value={voice}
            onChange={(mode) => {
              setVoice(mode as VoiceMode);
              void invoke("set_voice_mode", { mode });
            }}
          />
        </Section>

        <Section title="Listens with">
          {/* macOS owns this one. Offering a picker here would be a second place
              to set it that the system can overrule at any moment. */}
          <Row
            icon={<I.MicIcon />}
            label={mic}
            sub="Whichever input macOS has selected. Change it in Sound settings."
          />
        </Section>
      </Page>
    );
  }

  if (where === "tools") {
    return (
      <Page title="Tools" onBack={() => setWhere("root")}>
        <div className="space-y-2 pt-1">
          <Row
            icon={<I.Grid />}
            label="Integrations"
            sub="Services Nudge can reach"
            chevron
            onClick={onIntegrations}
          />
          <Row
            icon={<I.Bolt />}
            label="Skills"
            sub="Folders of instructions it can follow"
            chevron
            onClick={onSkills}
          />
        </div>
        <Section title="Servers">
          {servers.length === 0 ? (
            <p className="px-0.5 text-[10.5px] leading-snug text-ink-3">
              None yet. Each one is three lines in config.toml and brings its own
              tools.
            </p>
          ) : (
            // A count rather than a tick: a server you only know the name of is
            // one you have to trust.
            servers.map(([name, said]) => (
              <Row key={name} icon={<I.Dot className="bg-white/40" />} label={name} value={said} />
            ))
          )}
        </Section>
      </Page>
    );
  }

  if (where === "screen") {
    return (
      <Page title="Companion" onBack={() => setWhere("root")}>
        <p className="px-0.5 pt-1 pb-2.5 text-[10.5px] leading-snug text-ink-3">
          Who follows your cursor.
        </p>
        {/* Each one running, not a picture of it. What separates these is how
            they move -- a still frame of a thing that idles and blinks is a
            sticker, and picking between stickers tells you nothing about what
            will be beside your pointer all day. */}
        <div className="grid grid-cols-2 gap-2">
          {LOOKS.map((l) => {
            const on = l.key === look;
            return (
              <button
                key={l.key}
                onClick={() => {
                  setLook(l.key);
                  void invoke("set_look", { key: l.key });
                }}
                aria-pressed={on}
                className={[
                  "relative flex flex-col items-center rounded-xl bg-raise px-2 pt-3 pb-2.5 text-center",
                  "transition-colors duration-150 hover:bg-raise-hi",
                  on ? "inset-ring-1 inset-ring-blue" : "hairline",
                ].join(" ")}
              >
                {on && (
                  <span className="absolute top-1.5 right-1.5 grid size-[15px] place-items-center text-blue">
                    <svg viewBox="0 0 16 16" className="size-[13px]" fill="none" stroke="currentColor" strokeWidth={2.2} strokeLinecap="round" strokeLinejoin="round">
                      <path d="m3 8.5 3.5 3.5L13 5" />
                    </svg>
                  </span>
                )}
                <span className="grid h-[72px] place-items-center">
                  <span className="scale-[0.92]">
                    <Companion mode="idle" anchored />
                  </span>
                </span>
                <span className={`text-[12px] ${on ? "font-medium text-white" : "text-ink-2"}`}>
                  {l.name}
                </span>
                <span className="mt-px text-[10px] leading-snug text-ink-3">{l.about}</span>
              </button>
            );
          })}
        </div>
        {LOOKS.length === 1 && (
          <p className="px-0.5 pt-2.5 text-[10px] leading-snug text-ink-3">
            One so far. More are a file each — the list is in{" "}
            <code className="text-ink-2">companions.ts</code>.
          </p>
        )}
      </Page>
    );
  }

  return (
    <div className="flex-1 overflow-y-auto px-3 pb-3">
      {/* Each row says what is inside it, so checking a setting -- which is most
          of why anybody opens this -- needs no clicks at all. */}
      <Section title="Settings">
        <Row
          icon={<I.Bolt />}
          label="Allowed to"
          sub="What Nudge may do to your machine"
          value={open === 0 ? "nothing extra" : `${open} of ${allowed.length}`}
          chevron
          onClick={() => setWhere("allowed")}
        />
        <Row
          icon={<I.Spark />}
          label="Model"
          sub="Who answers, and how hard it thinks"
          value={brain?.provider ?? "…"}
          chevron
          onClick={() => setWhere("model")}
        />
        <Row
          icon={<I.Wave />}
          label="Voice"
          sub="Speaking and listening"
          value={{ off: "silent", system: "System", gemini: "Natural" }[voice]}
          chevron
          onClick={() => setWhere("voice")}
        />
        <Row
          icon={<I.Grid />}
          label="Tools"
          sub="Integrations, skills and servers"
          value={servers.length ? `${servers.length} server${servers.length > 1 ? "s" : ""}` : undefined}
          chevron
          onClick={() => setWhere("tools")}
        />
        <Row
          icon={<I.Arrow />}
          label="Companion"
          sub="Which one follows your cursor"
          value={LOOKS.find((l) => l.key === look)?.name ?? look}
          chevron
          onClick={() => setWhere("screen")}
        />
      </Section>

      <Section title="Tell us">
        <Row
          icon={<I.Bug />}
          label="Report a bug"
          sub="A sentence is enough. Screenshot optional."
          chevron
          onClick={() => onReport("bug")}
        />
        <Row
          icon={<I.Bulb />}
          label="Request a feature"
          sub="Say what you wanted it to do."
          chevron
          onClick={() => onReport("idea")}
        />
      </Section>

      <Section title="Nudge">
        <Row icon={<I.Power />} label="Quit" danger onClick={() => void invoke("quit")} />
      </Section>

      <p className="pt-3 pl-1 font-mono text-[10px] text-ink-3">v{version}</p>
    </div>
  );
}

/** A home-relative path, because the full one is mostly somebody's user name. */
function short(path: string) {
  const home = "/Users/";
  if (!path.startsWith(home)) return path;
  const rest = path.slice(home.length);
  const cut = rest.indexOf("/");
  return cut === -1 ? "~" : `~${rest.slice(cut)}`;
}
