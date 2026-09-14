import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Row, Section, Toggle } from "./parts";
import * as I from "./icons";

type VoiceMode = "off" | "system" | "gemini";
const VOICE_LABEL: Record<VoiceMode, string> = { off: "Off", system: "System", gemini: "Natural" };
const NEXT: Record<VoiceMode, VoiceMode> = { off: "system", system: "gemini", gemini: "off" };

/**
 * The settings sheet.
 *
 * No plan or usage section: theirs exists to sell an upgrade, and Nudge has
 * nothing to sell. A fictional quota in front of the user every time they open
 * settings would be a placeholder that actively misinforms.
 *
 * Most of it is inert. Community, Support, Connections and the account rows
 * are placeholders for features that do not exist -- Nudge has no accounts, no
 * plans and nothing to upgrade to -- and sit here so the shape is settled before
 * the behaviour arrives. Rows that actually do something are marked, and only
 * those are rendered as buttons, so nothing offers a hover state it cannot honour.
 */
export function Settings({
  docked,
  onDock,
  onIntegrations,
  onShortcuts,
}: {
  docked: boolean;
  onDock: (v: boolean) => void;
  onIntegrations: () => void;
  onShortcuts: () => void;
}) {
  const [voice, setVoice] = useState<VoiceMode>("system");
  const [mic, setMic] = useState("…");
  const [version, setVersion] = useState("");
  const [inDock, setInDock] = useState(false);
  const [inRecordings, setInRecordings] = useState(true);

  useEffect(() => {
    void invoke<VoiceMode>("voice_mode").then(setVoice);
    void invoke<string>("microphone").then(setMic);
    void invoke<string>("version").then(setVersion);
  }, []);

  return (
    <div className="flex-1 overflow-y-auto px-3 pb-3">
      <Section title="Community" columns>
        <Row compact icon={<I.Dot className="bg-[#5865f2]" />} label="Discord" />
        <Row compact icon={<I.Dot className="bg-white/70" />} label="GitHub" />
      </Section>

      <Section title="Support & updates" columns>
        <Row compact icon={<I.Bulb />} label="Request a feature" />
        <Row compact icon={<I.Bug />} label="Report a bug" />
        <Row compact icon={<I.Refresh />} label="Check for updates" />
        <Row compact icon={<I.Spark />} label="What's new" />
      </Section>

      <Section title="Connections">
        {/* Wired: opens the browser. */}
        <Row
          icon={<I.Grid />}
          label="Integrations"
          sub="Connect the apps you already use"
          chevron
          onClick={onIntegrations}
        />
        <Row icon={<I.Bolt />} label="Skills" sub="Power-ups that attach to Nudge" chevron />
      </Section>

      <Section title="Customization">
        <Row icon={<I.TextIcon />} label="Dictation" badge="NEW" value="Automatic" chevron />
        <Row icon={<I.Keyboard />} label="Shortcuts" sub="Every key Nudge listens for" chevron onClick={onShortcuts} />
        {/* Wired. */}
        <Row
          icon={<I.Arrow />}
          label="Cursor"
          value={docked ? "parked" : "following"}
          chevron
          onClick={() => onDock(!docked)}
        />
        {/* Wired: cycles Off → System → Natural. */}
        <Row
          icon={<I.Wave />}
          label="Voice"
          value={VOICE_LABEL[voice]}
          chevron
          onClick={() => {
            const next = NEXT[voice];
            setVoice(next);
            void invoke("set_voice_mode", { mode: next });
          }}
        />
        {/* Wired: the device recording will actually use. */}
        <Row icon={<I.MicIcon />} label="Microphone" value={mic} chevron />
        <Row icon={<I.Agent />} label="Agent" chevron />
        <Row
          icon={<I.DockIcon />}
          label="Show in Dock"
          sub="Turn off to keep Nudge notch only."
          trailing={<Toggle on={inDock} onChange={setInDock} />}
        />
        <Row
          icon={<I.Copy />}
          label="Show in screen recordings"
          sub="Let screen sharing and recording tools capture Nudge."
          trailing={<Toggle on={inRecordings} onChange={setInRecordings} />}
        />
      </Section>

      <Section title="Account actions">
        <Row icon={<I.Out />} label="Log Out" sub="Not signed in — Nudge has no accounts" />
        <Row icon={<I.Trash />} label="Delete Account" danger />
        {/* Wired. */}
        <Row icon={<I.Power />} label="Quit Nudge" danger onClick={() => void invoke("quit")} />
      </Section>

      <p className="pt-3 pl-1 font-mono text-[10px] text-white/25">v{version}</p>
    </div>
  );
}
