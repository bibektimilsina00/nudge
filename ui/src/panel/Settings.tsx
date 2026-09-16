import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Choice, Page, Row, Section, Toggle } from "./parts";
import * as I from "./icons";
import { LOOKS, DEFAULT_LOOK } from "../companions";
import { Companion } from "../components/Companion";

type VoiceMode = "off" | "system" | "gemini";
type Allowed = { key: string; label: string; about: string; on: boolean };
type Pick = { key: string; label: string; about: string };
type Shortcut = {
  id: string;
  name: string;
  about: string;
  keys: string[];
  fixed: string | null;
};
type Setup = { workspace: string; suggesting: boolean; steps: number };
type Server = { name: string; about: string; said: string; failed: boolean };
type PermitState = "granted" | "denied" | "unasked";
type Permit = {
  key: string;
  name: string;
  without: string;
  state: PermitState;
  essential: boolean;
};
type Brain = {
  provider: string;
  model: string;
  think: string;
  workspace: string;
  providers: Pick[];
  thinks: Pick[];
};

/** Which page of settings is open. */
type Where =
  | "root"
  | "allowed"
  | "model"
  | "voice"
  | "screen"
  | "permissions"
  | "keys"
  | "agents";

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
  const [update, setUpdate] = useState<{ version: string; notes: string } | null>(null);
  const [taking, setTaking] = useState(false);
  const [allowed, setAllowed] = useState<Allowed[]>([]);
  const [brain, setBrain] = useState<Brain | null>(null);
  const [servers, setServers] = useState<Server[]>([]);
  const [setup, setSetup] = useState<Setup | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [look, setLook] = useState(DEFAULT_LOOK);
  const [permits, setPermits] = useState<Permit[]>([]);
  const [keys, setKeys] = useState<Shortcut[]>([]);

  useEffect(() => {
    void invoke<VoiceMode>("voice_mode").then(setVoice);
    void invoke<string>("microphone").then(setMic);
    void invoke<string>("version").then(setVersion);
    // Pushed rather than polled: the check happens once, twenty seconds after
    // launch, and this page is usually not open when the answer arrives.
    const found = listen<{ version: string; notes: string }>("update", (e) =>
      setUpdate(e.payload),
    );
    void invoke<Allowed[]>("reach").then(setAllowed);
    void invoke<Brain>("brain").then(setBrain);
    void invoke<string>("look").then(setLook);
    // Polled, not asked once. The entire shape of granting one of these is that
    // somebody leaves for System Settings and comes back, and an answer cached
    // before they left would still say "not granted" over a grant already
    // working. A second is faster than anybody can tick a box and switch back.
    void invoke<Shortcut[]>("shortcuts").then(setKeys);
    void invoke<Setup>("agent_setup").then(setSetup);
    const grants = () => void invoke<Permit[]>("permits").then(setPermits);
    grants();
    const watching = window.setInterval(grants, 1000);
    // Servers connect in the background long after this mounts -- `npx` can spend
    // a minute fetching one it has never run -- so this looks again rather than
    // showing "starting…" forever to somebody who opened settings early.
    const look = () => void invoke<Server[]>("servers").then(setServers);
    look();
    const again = window.setInterval(look, 2000);
    return () => {
      window.clearInterval(again);
      window.clearInterval(watching);
      void found.then((un) => un());
    };
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
  // Only the ones Nudge cannot work without. The microphone being off is a
  // smaller app, not a broken one, and warning about it the same way would teach
  // people to ignore the warning.
  const short_of = permits.filter((p) => p.essential && p.state !== "granted");

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

  if (where === "agents" && setup) {
    return (
      <Page title="Agents" onBack={() => setWhere("root")}>
        <p className="px-0.5 pt-1 pb-2.5 text-[10.5px] leading-snug text-ink-3">
          Where agents work, and how much rope they get.
        </p>

        <Section title="Works inside">
          <button
            onClick={() =>
              void invoke<string | null>("pick_workspace")
                .then((picked) => {
                  // Nothing picked is a cancel, not a failure.
                  if (picked) void invoke<Setup>("agent_setup").then(setSetup);
                })
                .catch((e) => setProblem(String(e)))
            }
            className="flex w-full items-center gap-2.5 rounded-xl bg-raise p-2.5 text-left transition-colors duration-150 hover:bg-raise-hi hairline"
          >
            <span className="text-ink-2">
              <I.Grid />
            </span>
            <span className="min-w-0 flex-1">
              <span className="block truncate text-[12px] font-medium">
                {short(setup.workspace)}
              </span>
              {/* The boundary that makes shell access and file writing acceptable
                  at all, so it says what it is rather than "Folder". */}
              <span className="mt-px block text-[10.5px] leading-snug text-ink-3">
                Everything an agent runs and writes stays in here.
              </span>
            </span>
            <span className="shrink-0 text-[11.5px] text-blue">Change</span>
          </button>
          <p className="px-0.5 pt-1.5 text-[10px] leading-snug text-ink-3">
            Until you quit. Set <code className="text-ink-2">workspace</code> in
            config.toml to keep it — Nudge will not rewrite that file behind you.
          </p>
        </Section>

        <Section title="On its own">
          <Row
            icon={<I.Bulb />}
            label="Suggest things"
            sub="Let Nudge raise an idea now and then without being asked."
            trailing={
              <Toggle
                on={setup.suggesting}
                onChange={(on) => {
                  setSetup({ ...setup, suggesting: on });
                  void invoke("set_suggesting", { on });
                }}
              />
            }
          />
          {/* Stated rather than offered as a dial. The number exists so a model
              that never finishes gives up instead of clicking forever, and it is
              not a preference -- but hiding it entirely leaves people guessing
              why an agent stopped. */}
          <Row
            icon={<I.Agent />}
            label="Gives up after"
            sub="Long enough for a real task, short enough to stop a loop."
            value={`${setup.steps} steps`}
          />
          <Row
            icon={<I.Power />}
            label="Escape stops it"
            sub="From anywhere, whatever it is in the middle of."
          />
        </Section>

        <Section title="Tools they can use">
          {servers.length === 0 ? (
            <p className="px-0.5 text-[10.5px] leading-snug text-ink-3">
              None yet. Each one is three lines in config.toml and brings its own
              tools.
            </p>
          ) : (
            servers.map((s) => (
              <div key={s.name} className="flex items-start gap-2.5 rounded-xl bg-raise p-2.5 hairline">
                <span
                  className={`mt-px grid size-5 shrink-0 place-items-center rounded-[6px] ${
                    s.failed ? "bg-[#ff453a]/20 text-[#ff8a80]" : "bg-blue/20 text-blue"
                  }`}
                >
                  <I.Bolt />
                </span>
                <div className="min-w-0 flex-1">
                  <div className="flex items-baseline gap-2">
                    <h3 className="min-w-0 truncate text-[12px] font-medium">{s.name}</h3>
                    <span
                      className={`ml-auto shrink-0 text-[10.5px] ${
                        s.failed ? "text-[#ff8a80]" : "text-ink-3"
                      }`}
                    >
                      {s.said}
                    </span>
                  </div>
                  {/* The name is whatever the config called it. This is what is
                      actually running, and for a filesystem server the folder is
                      the whole question. */}
                  {s.about && (
                    <p className="mt-px truncate font-mono text-[10px] text-ink-3">{s.about}</p>
                  )}
                </div>
              </div>
            ))
          )}
        </Section>
      </Page>
    );
  }

  if (where === "keys") {
    return (
      <Page title="Shortcuts" onBack={() => setWhere("root")}>
        <p className="px-0.5 pt-1 pb-2.5 text-[10.5px] leading-snug text-ink-3">
          The keys that summon Nudge.
        </p>
        <div className="space-y-2">
          {keys.map((s) => (
            <Key
              key={s.id}
              shortcut={s}
              onChange={(combo) =>
                invoke("set_shortcut", { keys: combo })
                  .then(() => invoke<Shortcut[]>("shortcuts").then(setKeys))
                  .catch((e) => setProblem(String(e)))
              }
            />
          ))}
        </div>
        {problem && (
          <p className="mt-2 rounded-xl bg-[#ff453a]/12 p-2 text-[10.5px] leading-snug text-[#ff8a80]">
            {problem}
          </p>
        )}
      </Page>
    );
  }

  if (where === "permissions") {
    return (
      <Page title="Permissions" onBack={() => setWhere("root")}>
        <p className="px-0.5 pt-1 pb-2.5 text-[10.5px] leading-snug text-ink-3">
          Nudge looks at your screen and moves your cursor, so macOS makes you say
          so. Granting one takes effect immediately — no restart.
        </p>
        <div className="space-y-2">
          {permits.map((p) => (
            <Grant key={p.key} permit={p} />
          ))}
        </div>
      </Page>
    );
  }

  return (
    <div className="flex-1 overflow-y-auto px-3 pb-3">
      {/* Above everything, and only when it is true. Nudge cannot do the thing it
          is for without these, so a missing one is not a setting -- it is the
          reason nothing works, and it belongs where somebody looking for that
          reason will land. */}
      {short_of.length > 0 && (
        <button
          onClick={() => setWhere("permissions")}
          className="mt-2 flex w-full items-center gap-2.5 rounded-xl bg-[#ffd60a]/10 px-2.5 py-2 text-left transition-colors duration-150 hover:bg-[#ffd60a]/15 hairline"
        >
          <span className="text-[#ffd60a]">
            <svg viewBox="0 0 16 16" className="size-[15px]" fill="none" stroke="currentColor" strokeWidth={1.7} strokeLinecap="round">
              <path d="M8 1.8 15 14H1z" strokeLinejoin="round" />
              <path d="M8 6.4v3.2M8 11.6v.1" />
            </svg>
          </span>
          <span className="min-w-0 flex-1">
            <span className="block text-[12px] font-medium text-[#ffd60a]">
              {short_of.length === 1
                ? `${short_of[0].name} is off`
                : `${short_of.length} permissions are off`}
            </span>
            <span className="mt-px block text-[10.5px] leading-snug text-ink-2">
              {short_of[0].without}
            </span>
          </span>
          <svg viewBox="0 0 16 16" className="size-3 shrink-0 text-ink-3" fill="none" stroke="currentColor" strokeWidth={1.8} strokeLinecap="round" strokeLinejoin="round">
            <path d="M6.5 3.5 11 8l-4.5 4.5" />
          </svg>
        </button>
      )}
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
          icon={<I.Agent />}
          label="Agents"
          sub="Where they work, and how much rope"
          value={setup ? short(setup.workspace) : undefined}
          chevron
          onClick={() => {
            setProblem(null);
            setWhere("agents");
          }}
        />
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
        <Row
          icon={<I.Arrow />}
          label="Companion"
          sub="Which one follows your cursor"
          value={LOOKS.find((l) => l.key === look)?.name ?? look}
          chevron
          onClick={() => setWhere("screen")}
        />
      </Section>

      <Section title="System">
        <Row
          icon={<I.Keyboard />}
          label="Shortcuts"
          sub="The keys that summon Nudge"
          value={keys[0]?.keys.join(" ")}
          chevron
          onClick={() => {
            setProblem(null);
            setWhere("keys");
          }}
        />
        <Row
          icon={<I.Shield />}
          label="Permissions"
          sub="What macOS lets Nudge do"
          value={
            short_of.length > 0
              ? `${short_of.length} off`
              : permits.length
                ? "all granted"
                : undefined
          }
          chevron
          onClick={() => setWhere("permissions")}
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
        {/* Only when there is one. A row that says "no updates available" is a
            row that is wrong the moment it is right, and it asks somebody to
            care about maintenance on every visit to this page. */}
        {update && (
          <Row
            icon={<I.Refresh />}
            label={taking ? "Installing…" : `Update to ${update.version}`}
            sub={taking ? "Nudge will restart when it is done." : update.notes || undefined}
            onClick={() => {
              if (taking) return;
              setTaking(true);
              void invoke("take_update").catch(() => setTaking(false));
            }}
          />
        )}
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

/**
 * One grant, and the one thing to do about it.
 *
 * The button changes with the state because the routes are genuinely different.
 * macOS shows each prompt exactly once: before that, Allow works and is one
 * click; after a refusal -- or a dismissal, which it records as the same thing --
 * the API is silent forever and the only way through is the pane. Offering Allow
 * to somebody who already said no is a button that does nothing, which is how an
 * app teaches people its buttons are decorative.
 */
function Grant({ permit }: { permit: Permit }) {
  const [tried, setTried] = useState(false);
  const granted = permit.state === "granted";
  // After a refusal, and after an Allow that produced no prompt, the pane is the
  // only remaining route.
  const pane = permit.state === "denied" || tried;

  return (
    <div
      className={`flex items-start gap-2.5 rounded-xl p-2.5 hairline ${
        granted ? "bg-raise" : "bg-raise-hi"
      }`}
    >
      <span
        className={`mt-px grid size-[17px] shrink-0 place-items-center rounded-full ${
          granted ? "text-blue" : "text-ink-3"
        }`}
      >
        {granted ? (
          <svg viewBox="0 0 16 16" className="size-[15px]" fill="none" stroke="currentColor" strokeWidth={2.2} strokeLinecap="round" strokeLinejoin="round">
            <path d="m3 8.5 3.5 3.5L13 5" />
          </svg>
        ) : (
          <span className="size-[9px] rounded-full inset-ring-1 inset-ring-current" />
        )}
      </span>

      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-1.5">
          <h3 className="text-[12px] font-medium">{permit.name}</h3>
          {permit.essential && !granted && (
            <span className="rounded bg-[#ffd60a]/15 px-1 py-px text-[9px] font-semibold text-[#ffd60a]">
              NEEDED
            </span>
          )}
        </div>
        <p className="mt-px text-[10.5px] leading-snug text-ink-3">{permit.without}</p>
      </div>

      {!granted && (
        <button
          onClick={() => {
            if (pane) {
              void invoke("open_permit", { key: permit.key });
              return;
            }
            // `false` means macOS will not prompt -- already answered. The row
            // switches to the pane rather than leaving a dead button behind.
            void invoke<boolean>("ask_permit", { key: permit.key }).then((asked) => {
              if (!asked) void invoke("open_permit", { key: permit.key });
              setTried(true);
            });
          }}
          className="h-[24px] shrink-0 rounded-control bg-blue px-2.5 text-[11.5px] font-medium text-white transition-colors duration-150 hover:bg-blue-hi active:scale-[0.98]"
        >
          {pane ? "Open Settings" : "Allow"}
        </button>
      )}
    </div>
  );
}

/** The caps a shortcut is drawn as. */
function Caps({ keys }: { keys: string[] }) {
  return (
    <span className="flex shrink-0 items-center gap-1">
      {keys.map((k, i) => (
        <kbd
          key={`${k}-${i}`}
          className="rounded-[5px] bg-raise-on px-1.5 py-[2px] font-mono text-[10px] whitespace-nowrap text-ink"
        >
          {k}
        </kbd>
      ))}
    </span>
  );
}

/**
 * One shortcut, and a way to change it by pressing it.
 *
 * Recorded, not typed. The stored form is `Ctrl+Shift+Space`, which is what the OS
 * wants and not something anybody should have to know -- a text field here would
 * be asking people to spell an accelerator correctly and giving them an error when
 * they guess wrong.
 *
 * A bare modifier is a real answer, and the awkward one. Nudge's default is
 * Control on its own, which the OS has no notion of as a shortcut and which never
 * completes the usual "modifiers plus a key" shape -- so holding a modifier and
 * letting go is taken as meaning it, and anything pressed while it is held wins
 * instead.
 */
function Key({
  shortcut,
  onChange,
}: {
  shortcut: Shortcut;
  onChange: (combo: string) => void;
}) {
  const [listening, setListening] = useState(false);
  const [held, setHeld] = useState<string[]>([]);

  useEffect(() => {
    if (!listening) return;

    const mods = (e: KeyboardEvent) => {
      const out: string[] = [];
      if (e.ctrlKey) out.push("Ctrl");
      if (e.shiftKey) out.push("Shift");
      if (e.altKey) out.push("Alt");
      if (e.metaKey) out.push("Cmd");
      return out;
    };

    const down = (e: KeyboardEvent) => {
      e.preventDefault();
      if (e.key === "Escape") {
        setListening(false);
        setHeld([]);
        return;
      }
      const m = mods(e);
      // A modifier on its own is not finished yet -- it is either the whole
      // answer or the start of one, and only letting go says which.
      if (["Control", "Shift", "Alt", "Meta"].includes(e.key)) {
        setHeld(m);
        return;
      }
      const named = e.key === " " ? "Space" : e.key.length === 1 ? e.key.toUpperCase() : e.key;
      setListening(false);
      setHeld([]);
      onChange([...m, named].join("+"));
    };

    const up = (e: KeyboardEvent) => {
      if (!["Control", "Shift", "Alt", "Meta"].includes(e.key)) return;
      // Released with nothing else pressed: they meant the modifier itself. Only
      // Control is watchable that way, so anything else is not offered.
      if (e.key === "Control" && held.length === 1 && held[0] === "Ctrl") {
        setListening(false);
        setHeld([]);
        onChange("ctrl");
        return;
      }
      setHeld([]);
    };

    window.addEventListener("keydown", down, true);
    window.addEventListener("keyup", up, true);
    return () => {
      window.removeEventListener("keydown", down, true);
      window.removeEventListener("keyup", up, true);
    };
  }, [listening, held, onChange]);

  return (
    <div className="flex items-start gap-2.5 rounded-xl bg-raise p-2.5 hairline">
      <div className="min-w-0 flex-1">
        <h3 className="text-[12px] font-medium">{shortcut.name}</h3>
        <p className="mt-px text-[10.5px] leading-snug text-ink-3">
          {listening ? "Press the keys you want. Escape to cancel." : shortcut.about}
        </p>
      </div>

      {listening ? (
        <span className="flex shrink-0 items-center gap-1.5">
          {held.length > 0 ? (
            <Caps keys={held} />
          ) : (
            <span className="text-[11px] text-blue">Listening…</span>
          )}
        </span>
      ) : (
        <Caps keys={shortcut.keys} />
      )}

      {shortcut.fixed ? (
        <span
          title={shortcut.fixed}
          className="grid size-[24px] shrink-0 place-items-center text-ink-3"
        >
          <svg viewBox="0 0 16 16" className="size-[13px]" fill="none" stroke="currentColor" strokeWidth={1.6}>
            <rect x="3.5" y="7" width="9" height="6" rx="1.5" />
            <path d="M5.8 7V5.2a2.2 2.2 0 0 1 4.4 0V7" strokeLinecap="round" />
          </svg>
        </span>
      ) : (
        <button
          onClick={() => setListening((l) => !l)}
          className={`h-[24px] shrink-0 rounded-control px-2.5 text-[11.5px] font-medium transition-colors duration-150 active:scale-[0.98] ${
            listening ? "bg-raise-on text-ink" : "bg-blue text-white hover:bg-blue-hi"
          }`}
        >
          {listening ? "Cancel" : "Change"}
        </button>
      )}
    </div>
  );
}
