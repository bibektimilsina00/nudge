import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Row, Section, Toggle } from "./parts";
import * as I from "./icons";

type VoiceMode = "off" | "system" | "gemini";
const VOICE_LABEL: Record<VoiceMode, string> = { off: "Off", system: "System", gemini: "Natural" };
const NEXT: Record<VoiceMode, VoiceMode> = { off: "system", system: "gemini", gemini: "off" };

type Allowed = { key: string; label: string; about: string; on: boolean };
type Brain = { provider: string; model: string; think: string; workspace: string };

const PROVIDER: Record<string, string> = {
  ollama: "Ollama — local and free",
  gemini: "Gemini",
  anthropic: "Anthropic",
};

/**
 * The settings sheet.
 *
 * This used to be somebody else's settings sheet. It had a Community section
 * linking to a Discord nobody was in, a Dictation row for a feature that does not
 * exist here, and -- the tell -- Log Out and Delete Account, in an app with no
 * accounts to log out of. Those were not rows adapted from another product, they
 * were *its* rows, kept because the shape looked right.
 *
 * Meanwhile the settings Nudge actually has lived only in a TOML file and the menu
 * bar: which model is answering, what it is allowed to do to your machine, which
 * tool servers arrived, where it may write. The sheet was a stranger's feature
 * list sitting on an app whose own decisions were invisible.
 *
 * So it is built from those instead, under two rules.
 *
 * **Nothing inert.** Every row reads or changes something real. The old sheet was
 * mostly placeholders standing in for behaviour that might arrive later, which is
 * how a settings screen teaches people that its switches do nothing.
 *
 * **The dangerous things first.** "Allowed to" is at the top because it is the one
 * section where a wrong answer costs something, and a permission buried under four
 * rows of preferences is a permission nobody audits.
 */
export function Settings({
  docked,
  onDock,
  onIntegrations,
  onSkills,
  onReport,
}: {
  docked: boolean;
  onDock: (v: boolean) => void;
  onIntegrations: () => void;
  onSkills: () => void;
  onReport: (kind: "bug" | "idea") => void;
}) {
  const [voice, setVoice] = useState<VoiceMode>("system");
  const [mic, setMic] = useState("…");
  const [version, setVersion] = useState("");
  const [allowed, setAllowed] = useState<Allowed[]>([]);
  const [brain, setBrain] = useState<Brain | null>(null);
  const [servers, setServers] = useState<[string, string][]>([]);

  useEffect(() => {
    void invoke<VoiceMode>("voice_mode").then(setVoice);
    void invoke<string>("microphone").then(setMic);
    void invoke<string>("version").then(setVersion);
    void invoke<Allowed[]>("reach").then(setAllowed);
    void invoke<Brain>("brain").then(setBrain);
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

  return (
    <div className="flex-1 overflow-y-auto px-3 pb-3">
      {/* First, on purpose. The only section where being wrong costs anything,
          and a permission under four rows of preferences is one nobody looks at
          again. */}
      <Section title="Allowed to">
        {allowed.map((g) => (
          <Row
            key={g.key}
            icon={<I.Bolt />}
            label={g.label}
            sub={g.about}
            trailing={<Toggle on={g.on} onChange={(v) => allow(g.key, v)} />}
          />
        ))}
        {brain && (
          <Row
            icon={<I.Grid />}
            label="Works inside"
            sub="What it runs and writes stays here."
            value={short(brain.workspace)}
          />
        )}
      </Section>

      <Section title="Answering">
        {brain && (
          <>
            {/* Read, not edited. The config file is the feature; a panel that
                half-edits it is a second place to look with fewer answers. */}
            <Row
              icon={<I.Spark />}
              label={PROVIDER[brain.provider] ?? brain.provider}
              sub={brain.model}
            />
            <Row icon={<I.Bulb />} label="Thinking" value={brain.think} />
          </>
        )}
        <Row icon={<I.MicIcon />} label="Microphone" value={mic} />
        {/* Cycles Off → System → Natural. */}
        <Row
          icon={<I.Wave />}
          label="Speaks back"
          value={VOICE_LABEL[voice]}
          chevron
          onClick={() => {
            const next = NEXT[voice];
            setVoice(next);
            void invoke("set_voice_mode", { mode: next });
          }}
        />
      </Section>

      <Section title="Tools">
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
        {/* Whatever the config named. A count rather than a tick: a server you
            only know the name of is one you have to trust. */}
        {servers.map(([name, said]) => (
          <Row key={name} icon={<I.Dot className="bg-white/40" />} label={name} value={said} />
        ))}
      </Section>

      <Section title="On screen">
        <Row
          icon={<I.Arrow />}
          label="Companion"
          sub={docked ? "Parked in the panel." : "Following your cursor."}
          value={docked ? "parked" : "loose"}
          chevron
          onClick={() => onDock(!docked)}
        />
      </Section>

      {/* A full row each rather than a pair of tiles: these are the only way
          anything above ever finds out it is wrong. */}
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
