import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

/** Mirrors `Listed` in `app/commands/connect.rs`. */
type Listed = {
  key: string;
  name: string;
  about: string;
  /** What connecting grants. Shown before consent, never after. */
  access: string;
  needs_token: boolean;
  where_from: string | null;
  /** For the ones signed into rather than pasted. */
  setup: string | null;
  needs_folder: boolean;
  connected: boolean;
  /** What the server itself said it could do. */
  tools: string[];
  /** Which of those may be used. `null` means all of them -- never reviewed. */
  allowed: string[] | null;
  /** Which were looked at and turned down, as opposed to never seen. */
  declined: string[];
};

/**
 * The integrations browser.
 *
 * It used to be a catalogue of logos with an inert Connect button -- a list of
 * things that did not work, which is worse than a short list of things that do.
 * What is here now is whatever `core::connect` offers, and connecting one
 * actually starts its server.
 *
 * **Access before consent.** Every card states what connecting grants, in the
 * same words whether or not anybody clicks. That is the rule OpenWorker's
 * catalogue enforces with a test, and this one enforces it too -- an offer with
 * nothing to say about its access does not compile.
 *
 * Monogram tiles rather than the real brand marks: shipping other companies'
 * logos into a binary is a licensing question, and a coloured initial carries
 * the same recognition in a 22px square.
 */
const TINT: Record<string, string> = {
  files: "#8a8f98",
  google: "#1a73e8",
  github: "#24292f",
  slack: "#4a154b",
};

export function Integrations({ onBack }: { onBack: () => void }) {
  const [query, setQuery] = useState("");
  const [all, setAll] = useState<Listed[]>([]);

  const load = () => void invoke<Listed[]>("connections").then(setAll).catch(() => {});
  useEffect(load, []);

  const shown = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return all;
    return all.filter(
      (s) => s.name.toLowerCase().includes(q) || s.about.toLowerCase().includes(q),
    );
  }, [query, all]);

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <header className="flex items-center gap-2.5 px-3 pt-2.5 pb-3">
        <button
          onClick={onBack}
          aria-label="Back"
          className="grid size-[21px] shrink-0 place-items-center rounded-full bg-raise text-ink-2 transition-colors duration-150 hover:bg-raise-hi hover:text-white"
        >
          <svg viewBox="0 0 16 16" className="size-3" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
            <path d="M9.5 3.5 5 8l4.5 4.5" />
          </svg>
        </button>
        <h2 className="text-[13px] font-semibold tracking-tight">Integrations</h2>
      </header>

      <div className="px-3.5 pb-2.5">
        <div className="flex h-[30px] items-center gap-2 rounded-[10px] bg-raise px-2.5 hairline">
          <svg viewBox="0 0 16 16" className="size-3.5 shrink-0 text-ink-3" fill="none" stroke="currentColor" strokeWidth={1.7} strokeLinecap="round">
            <circle cx="7.2" cy="7.2" r="4.2" />
            <path d="m10.4 10.4 2.6 2.6" />
          </svg>
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search integrations"
            spellCheck={false}
            className="w-full bg-transparent text-[11.5px] text-white outline-none placeholder:text-ink-3"
          />
        </div>
      </div>

      <div className="min-h-0 flex-1 space-y-2 overflow-y-auto px-3 pb-3">
        {shown.map((s) => (
          <Card key={s.key} it={s} onChanged={load} />
        ))}
        {shown.length === 0 && (
          <p className="pt-6 text-center text-[12px] text-ink-3">Nothing matches “{query}”.</p>
        )}
      </div>
    </div>
  );
}

function Card({ it, onChanged }: { it: Listed; onChanged: () => void }) {
  const [open, setOpen] = useState(false);
  const [token, setToken] = useState("");
  const [folder, setFolder] = useState("");
  const [busy, setBusy] = useState(false);
  const [failed, setFailed] = useState("");
  const [tools, setTools] = useState(false);

  const go = () => {
    setBusy(true);
    setFailed("");
    void invoke<string[]>("connect", {
      key: it.key,
      token: token || null,
      folder: folder || null,
    })
      .then(() => {
        setOpen(false);
        setToken("");
        onChanged();
      })
      .catch((e) => setFailed(String(e)))
      .finally(() => setBusy(false));
  };

  return (
    <div className="rounded-xl bg-raise p-2.5 hairline">
      <div className="flex items-start gap-2.5">
        <span
          className="grid size-5 shrink-0 place-items-center rounded-[6px] text-[9px] font-bold text-white"
          style={{ backgroundColor: TINT[it.key] ?? "#555" }}
        >
          {it.name.slice(0, 2).toUpperCase()}
        </span>

        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <h3 className="text-[12px] font-semibold">{it.name}</h3>
            {it.connected && (
              <span className="rounded bg-[#30d158]/15 px-1.5 py-[1px] text-[9px] text-[#30d158]">
                Connected
              </span>
            )}
          </div>
          <p className="mt-1 text-[10.5px] leading-snug text-ink-2">{it.about}</p>
          {/* Always, not only while deciding. What something can reach is not a
              detail that stops mattering once it is connected. */}
          <p className="mt-1 text-[10px] leading-snug text-ink-3">
            <span className="text-ink-2">Gets:</span> {it.access}
          </p>
          {it.connected && it.tools.length > 0 && (
            // What the server said, not what the catalogue claimed -- and the
            // way in to changing it, because a count nobody can act on is
            // decoration.
            <button
              onClick={() => setTools((t) => !t)}
              className="mt-1 text-[10px] text-ink-3 transition-colors duration-150 hover:text-ink-2"
            >
              {it.allowed ? `${it.allowed.length} of ${it.tools.length}` : it.tools.length} tools
              allowed · {tools ? "hide" : "choose"}
            </button>
          )}
        </div>

        {it.connected ? (
          <button
            onClick={() => void invoke("disconnect", { key: it.key }).then(onChanged)}
            className="shrink-0 rounded-full bg-raise px-2.5 py-[5px] text-[11px] font-medium text-ink-2 transition-colors duration-150 hover:bg-[#ff5f57] hover:text-white"
          >
            Disconnect
          </button>
        ) : (
          <button
            onClick={() =>
              it.needs_token || it.needs_folder || it.setup ? setOpen((o) => !o) : go()
            }
            disabled={busy}
            className="shrink-0 rounded-full bg-blue px-2.5 py-[5px] text-[11px] font-medium text-white transition-colors duration-150 hover:bg-blue-hi disabled:opacity-50"
          >
            {busy ? "Checking…" : "Connect"}
          </button>
        )}
      </div>

      {open && !it.connected && (
        <div className="mt-2.5 space-y-2 border-t border-line pt-2.5">
          {it.needs_folder && (
            <input
              value={folder}
              onChange={(e) => setFolder(e.target.value)}
              placeholder="Which folder? e.g. /Users/you/Work"
              spellCheck={false}
              className="w-full rounded-lg bg-black/40 px-2.5 py-1.5 text-[11px] text-white outline-none hairline placeholder:text-ink-3 focus:inset-ring-[#0a84ff]"
            />
          )}
          {/* Not a field. Signing in happens in a browser with their own
              account, on the provider's own consent screen -- so what is shown
              is what to go and do, and the Connect button below checks whether
              they did it. */}
          {it.setup && (
            <p className="text-[10.5px] leading-snug whitespace-pre-line text-ink-2">
              {it.setup}
            </p>
          )}
          {it.needs_token && (
            <>
              <input
                value={token}
                onChange={(e) => setToken(e.target.value)}
                type="password"
                placeholder="Paste the token"
                spellCheck={false}
                className="w-full rounded-lg bg-black/40 px-2.5 py-1.5 text-[11px] text-white outline-none hairline placeholder:text-ink-3 focus:inset-ring-[#0a84ff]"
              />
              {/* Where to get one. "Paste your token" is not an instruction
                  anybody can follow, and hunting for it is where people stop. */}
              {it.where_from && (
                <p className="text-[10px] leading-snug text-ink-3">{it.where_from}</p>
              )}
            </>
          )}
          {failed && <p className="text-[10px] leading-snug text-[#ff8a80]">{failed}</p>}
          <button
            onClick={go}
            disabled={busy}
            className="w-full rounded-lg bg-blue py-1.5 text-[11px] font-medium text-white transition-colors duration-150 hover:bg-blue-hi disabled:opacity-50"
          >
            {busy ? "Starting it to check…" : it.setup ? "I've done that — check" : "Connect"}
          </button>
        </div>
      )}

      {tools && it.connected && <Tools it={it} onChanged={onChanged} />}
    </div>
  );
}

/**
 * Choosing which of a server's tools may be used.
 *
 * **An unchecked tool is not blocked, it is absent.** It is never collected when
 * the server starts, so the model is never shown its name or its schema and has
 * nothing to call, argue with, or be talked into. Taken from OpenWorker, which
 * calls this the existence lever and is right to.
 *
 * Three states, not two, and the third is the one that is easy to miss: a tool
 * that is allowed, a tool that was looked at and turned down, and a tool nobody
 * has ever seen because the server grew it after the last review. Without the
 * middle one, declining something makes it come back wearing a `new` badge every
 * time the list is opened, which teaches people to ignore the badge.
 */
function Tools({ it, onChanged }: { it: Listed; onChanged: () => void }) {
  const [checked, setChecked] = useState<Set<string>>(
    // No list yet means everything, which is what an older connection looks like.
    () => new Set(it.allowed ?? it.tools),
  );
  const [query, setQuery] = useState("");
  const [saving, setSaving] = useState(false);

  const shown = useMemo(() => {
    const q = query.trim().toLowerCase();
    return q ? it.tools.filter((t) => t.toLowerCase().includes(q)) : it.tools;
  }, [query, it.tools]);

  const isNew = (t: string) => it.allowed !== null && !it.allowed.includes(t) && !it.declined.includes(t);

  const flip = (t: string) =>
    setChecked((was) => {
      const next = new Set(was);
      next.has(t) ? next.delete(t) : next.add(t);
      return next;
    });

  const save = () => {
    setSaving(true);
    void invoke("choose_tools", { key: it.key, allowed: [...checked] })
      .then(onChanged)
      .finally(() => setSaving(false));
  };

  return (
    <div className="mt-2.5 space-y-2 border-t border-line pt-2.5">
      <div className="flex items-center gap-2">
        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder={`Search ${it.tools.length} tools`}
          spellCheck={false}
          className="min-w-0 flex-1 rounded-lg bg-black/40 px-2.5 py-1.5 text-[11px] text-white outline-none hairline placeholder:text-ink-3 focus:inset-ring-[#0a84ff]"
        />
        <button
          onClick={() => setChecked(new Set(it.tools))}
          className="shrink-0 text-[10px] text-ink-3 transition-colors duration-150 hover:text-ink-2"
        >
          All
        </button>
        <button
          onClick={() => setChecked(new Set())}
          className="shrink-0 text-[10px] text-ink-3 transition-colors duration-150 hover:text-ink-2"
        >
          None
        </button>
      </div>

      <div className="max-h-56 space-y-px overflow-y-auto">
        {shown.map((t) => (
          <label
            key={t}
            className="flex cursor-pointer items-center gap-2 rounded px-1 py-[3px] hover:bg-white/5"
          >
            <input
              type="checkbox"
              checked={checked.has(t)}
              onChange={() => flip(t)}
              className="size-3 shrink-0 accent-[#0a84ff]"
            />
            <span className="min-w-0 flex-1 truncate font-mono text-[10px] text-ink-2">{t}</span>
            {isNew(t) && (
              // Only ever a name neither list has seen -- so it means "the server
              // added this since you looked", and nothing else.
              <span className="shrink-0 rounded bg-[#ff9f0a]/15 px-1 py-[1px] text-[9px] text-[#ff9f0a]">
                new
              </span>
            )}
          </label>
        ))}
        {shown.length === 0 && (
          <p className="px-1 py-2 text-[10px] text-ink-3">Nothing matches that.</p>
        )}
      </div>

      <p className="text-[10px] leading-snug text-ink-3">
        {checked.size} of {it.tools.length} allowed. Unchecked tools are not hidden from the
        assistant, they are absent — it is never told they exist. Takes effect next time this
        server starts.
      </p>

      <button
        onClick={save}
        disabled={saving}
        className="w-full rounded-lg bg-blue py-1.5 text-[11px] font-medium text-white transition-colors duration-150 hover:bg-blue-hi disabled:opacity-50"
      >
        {saving ? "Saving…" : "Save"}
      </button>
    </div>
  );
}
