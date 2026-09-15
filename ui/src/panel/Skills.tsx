import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type Skill = { name: string; about: string; folder: string };

/**
 * The skills browser.
 *
 * Unlike Integrations, everything here is real: the list is read off disk and
 * `Add a skill` opens the folder. That is the whole of installing one -- a skill
 * is a directory with a `SKILL.md` in it, which is the shape the rest of the
 * agent world already uses, so a skill somebody already has works here.
 *
 * Read on every open rather than cached. Somebody who has just dropped a folder
 * in expects to see it, and a list that needs the app restarted to refresh is the
 * kind of thing people assume is broken.
 */
export function Skills({ onBack }: { onBack: () => void }) {
  const [skills, setSkills] = useState<Skill[] | null>(null);
  const [query, setQuery] = useState("");

  const load = () => {
    invoke<Skill[]>("skills")
      .then(setSkills)
      .catch(() => setSkills([]));
  };

  useEffect(() => {
    load();
    // Looking again when the window comes back is what makes "drop a folder in"
    // feel finished: you add the folder, click back into Nudge, and it is there.
    window.addEventListener("focus", load);
    return () => window.removeEventListener("focus", load);
  }, []);

  const shown = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!skills) return [];
    if (!q) return skills;
    return skills.filter(
      (s) => s.name.toLowerCase().includes(q) || s.about.toLowerCase().includes(q),
    );
  }, [skills, query]);

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <header className="flex items-center gap-2.5 px-3 pt-2.5 pb-3">
        <button
          onClick={onBack}
          aria-label="Back"
          className="grid size-[21px] shrink-0 place-items-center rounded-full bg-[#1e1e1e] text-white/70 transition-colors duration-150 hover:bg-[#262626] hover:text-white"
        >
          <svg viewBox="0 0 16 16" className="size-3" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
            <path d="M9.5 3.5 5 8l4.5 4.5" />
          </svg>
        </button>
        <h2 className="text-[13px] font-semibold tracking-tight">Skills</h2>
        {skills !== null && skills.length > 0 && (
          <span className="ml-auto text-[10.5px] text-white/35">
            {skills.length} installed
          </span>
        )}
      </header>

      {/* No search box until there is enough to search. A filter over three items
          is furniture. */}
      {skills !== null && skills.length > 3 && (
        <div className="px-3.5 pb-2.5">
          <div className="flex h-[30px] items-center gap-2 rounded-[10px] bg-[#1e1e1e] px-2.5 inset-ring-1 inset-ring-white/[0.09]">
            <svg viewBox="0 0 16 16" className="size-3.5 shrink-0 text-white/35" fill="none" stroke="currentColor" strokeWidth={1.7} strokeLinecap="round">
              <circle cx="7.2" cy="7.2" r="4.2" />
              <path d="m10.4 10.4 2.6 2.6" />
            </svg>
            <input
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="Search skills"
              spellCheck={false}
              className="w-full bg-transparent text-[11.5px] text-white outline-none placeholder:text-white/30"
            />
          </div>
        </div>
      )}

      <div className="min-h-0 flex-1 space-y-2 overflow-y-auto px-3 pb-3">
        {skills === null ? null : skills.length === 0 ? (
          <Empty onAdd={() => invoke("open_skills_folder").catch(() => {})} />
        ) : (
          <>
            {shown.map((s) => (
              <Card key={s.folder} skill={s} />
            ))}
            {shown.length === 0 && (
              <p className="pt-6 text-center text-[12px] text-white/30">
                Nothing matches “{query}”.
              </p>
            )}
            <button
              onClick={() => invoke("open_skills_folder").catch(() => {})}
              className="flex w-full items-center justify-center gap-1.5 rounded-xl border border-dashed border-white/[0.14] py-2.5 text-[11px] text-white/45 transition-colors duration-150 hover:border-white/25 hover:text-white/70"
            >
              <Plus />
              Add a skill
            </button>
          </>
        )}
      </div>
    </div>
  );
}

/**
 * What somebody sees before they have any.
 *
 * It says what a skill *is*, because nobody can install one without knowing that,
 * and the answer is short enough to fit: a folder with a SKILL.md in it.
 */
function Empty({ onAdd }: { onAdd: () => void }) {
  return (
    <div className="flex flex-col items-center px-4 pt-7 text-center">
      <span className="grid size-10 place-items-center rounded-xl bg-[#1e1e1e] text-white/45 inset-ring-1 inset-ring-white/[0.09]">
        <Bolt />
      </span>
      <h3 className="mt-3 text-[12.5px] font-semibold">No skills yet</h3>
      <p className="mt-1.5 max-w-[250px] text-[10.5px] leading-relaxed text-white/45">
        A skill is a folder with a <code className="text-white/60">SKILL.md</code> in
        it — a name, a line about what it does, and the steps. Nudge reads the line
        on every turn and the steps only when it needs them.
      </p>
      <button
        onClick={onAdd}
        className="mt-3.5 flex items-center gap-1.5 rounded-full bg-[#0a84ff] px-3 py-[6px] text-[11px] font-medium text-white transition-colors duration-150 hover:bg-[#0a7ae8]"
      >
        <Plus />
        Open the skills folder
      </button>
    </div>
  );
}

function Card({ skill }: { skill: Skill }) {
  return (
    <div className="flex items-start gap-2.5 rounded-xl bg-[#1e1e1e] p-2.5 inset-ring-1 inset-ring-white/[0.09]">
      <span className="grid size-5 shrink-0 place-items-center rounded-[6px] bg-white/[0.09] text-white/55">
        <Bolt small />
      </span>

      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <h3 className="truncate text-[12px] font-semibold">{skill.name}</h3>
          <span className="shrink-0 rounded bg-[#0a84ff]/15 px-1.5 py-[1px] text-[9px] text-[#4da3ff]">
            Ready
          </span>
        </div>
        {/* A skill with no description is still usable; saying so beats an empty
            line that looks like something failed to load. */}
        <p className="mt-1 line-clamp-2 text-[10.5px] leading-snug text-white/45">
          {skill.about || "No description in its SKILL.md."}
        </p>
      </div>
    </div>
  );
}

function Plus() {
  return (
    <svg viewBox="0 0 16 16" className="size-3" fill="none" stroke="currentColor" strokeWidth={1.9} strokeLinecap="round">
      <path d="M8 3.5v9M3.5 8h9" />
    </svg>
  );
}

function Bolt({ small }: { small?: boolean }) {
  return (
    <svg viewBox="0 0 16 16" className={small ? "size-3" : "size-[18px]"} fill="none" stroke="currentColor" strokeWidth={1.6} strokeLinejoin="round">
      <path d="M8.8 1.8 3.6 9.1h3.4l-.8 5.1 5.2-7.3H8l.8-5.1Z" />
    </svg>
  );
}
