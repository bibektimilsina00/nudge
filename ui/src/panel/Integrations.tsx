import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type Service = { name: string; tint: string; mark: string; dark?: boolean; blurb: string };
type Server = { name: string; about: string; said: string; failed: boolean };

/**
 * The integrations browser.
 *
 * Two halves, and only the first is real. The tool servers at the top are
 * whatever the config named and are actually connected and actually working; the
 * catalogue underneath is a list of services Nudge does not reach yet, and
 * `Connect` on those is inert.
 *
 * The real ones go first and say so. A page that opens on a wall of famous logos
 * that do nothing reads as a product that does nothing -- while the thing that
 * does work, and is doing it right now, was not on this page at all.
 *
 * Monogram tiles rather than the real brand marks: shipping other companies'
 * logos into a binary is a licensing question, and a coloured initial carries the
 * same recognition in a 22px square.
 */
const SERVICES: Service[] = [
  {
    name: "Notion",
    tint: "#ffffff",
    mark: "N",
    dark: true,
    blurb:
      "Search Notion pages and databases, read content, then create, update, comment on, organize, or attach uploaded files to pages.",
  },
  {
    name: "Linear",
    tint: "#5e6ad2",
    mark: "L",
    blurb:
      "Search Linear issues, teams, projects, cycles, labels, and users, then create or update issues, comments, projects, and milestones.",
  },
  {
    name: "GitHub",
    tint: "#e6e6e6",
    mark: "G",
    dark: true,
    blurb:
      "Search repositories, inspect issues, PRs, commits, releases, and Actions, then create or update repo work with approval.",
  },
  {
    name: "Google Docs",
    tint: "#4285f4",
    mark: "D",
    blurb:
      "Search Docs, create documents, read or export content, insert text, images, and tables, and update document sections.",
  },
  {
    name: "Google Calendar",
    tint: "#1a73e8",
    mark: "31",
    blurb:
      "List calendars and events, find free time, then create, move, update, or delete Google Calendar events.",
  },
  {
    name: "Slack",
    tint: "#4a154b",
    mark: "S",
    blurb:
      "Search channels and threads, read recent messages, then post, reply, or react on your behalf.",
  },
  {
    name: "Gmail",
    tint: "#ea4335",
    mark: "M",
    blurb:
      "Search mail, read threads and attachments, then draft, reply, label, or archive with approval.",
  },
];

export function Integrations({ onBack }: { onBack: () => void }) {
  const [query, setQuery] = useState("");
  const [servers, setServers] = useState<Server[]>([]);

  // They connect in the background long after this mounts -- `npx` can spend a
  // minute fetching a server it has never run -- so this looks again rather than
  // saying "starting…" forever.
  useEffect(() => {
    const look = () => void invoke<Server[]>("servers").then(setServers);
    look();
    const again = window.setInterval(look, 2000);
    return () => window.clearInterval(again);
  }, []);

  // Filtering is real even though connecting is not -- a search box that does
  // nothing is more confusing than no search box.
  const shown = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return SERVICES;
    return SERVICES.filter(
      (s) => s.name.toLowerCase().includes(q) || s.blurb.toLowerCase().includes(q),
    );
  }, [query]);

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
        {servers.length > 0 && !query && (
          <>
            <p className="px-0.5 pt-0.5 text-[10.5px] text-ink-3">
              Connected — tool servers from your config
            </p>
            {servers.map((s) => (
              <div key={s.name} className="flex items-start gap-2.5 rounded-xl bg-raise p-2.5 hairline">
                <span
                  className={`grid size-5 shrink-0 place-items-center rounded-[6px] ${
                    s.failed ? "bg-[#ff453a]/20 text-[#ff8a80]" : "bg-blue/20 text-blue"
                  }`}
                >
                  <svg viewBox="0 0 16 16" className="size-3" fill="none" stroke="currentColor" strokeWidth={1.8} strokeLinecap="round">
                    <path d="M9 2 4 9h3.5L7 14l5-7H8.5Z" strokeLinejoin="round" />
                  </svg>
                </span>
                <div className="min-w-0 flex-1">
                  <div className="flex items-baseline gap-2">
                    <h3 className="min-w-0 truncate text-[12px] font-semibold">{s.name}</h3>
                    {/* A count, not a tick. A server you only know the name of is
                        one you have to trust; one that says it brought fourteen
                        tools is one you can weigh. */}
                    <span
                      className={`ml-auto shrink-0 text-[10.5px] ${
                        s.failed ? "text-[#ff8a80]" : "text-ink-3"
                      }`}
                    >
                      {s.said}
                    </span>
                  </div>
                  {/* The name is whatever the config called it, and "files" is a
                      reasonable name that means nothing to a reader. This is what
                      is actually running. */}
                  {s.about && (
                    <p className="mt-px truncate font-mono text-[10px] text-ink-3">{s.about}</p>
                  )}
                </div>
              </div>
            ))}
            <p className="px-0.5 pt-2 text-[10.5px] text-ink-3">Not yet reachable</p>
          </>
        )}
        {shown.map((s) => (
          <Card key={s.name} service={s} />
        ))}
        {shown.length === 0 && (
          <p className="pt-6 text-center text-[12px] text-ink-3">Nothing matches “{query}”.</p>
        )}
      </div>
    </div>
  );
}

function Card({ service }: { service: Service }) {
  return (
    <div className="flex items-start gap-2.5 rounded-xl bg-raise p-2.5 hairline">
      <span
        className="grid size-5 shrink-0 place-items-center rounded-[6px] text-[9px] font-bold"
        style={{
          backgroundColor: service.tint,
          color: service.dark ? "#111" : "#fff",
        }}
      >
        {service.mark}
      </span>

      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <h3 className="text-[12px] font-semibold">{service.name}</h3>
          <span className="rounded bg-raise px-1.5 py-[1px] text-[9px] text-ink-3">
            Not connected
          </span>
        </div>
        {/* Two lines, then clipped -- the same shape for every card keeps the list
            scannable no matter how much the service has to say about itself. */}
        <p className="mt-1 line-clamp-2 text-[10.5px] leading-snug text-ink-2">
          {service.blurb}
        </p>
      </div>

      <button className="flex shrink-0 items-center gap-1.5 rounded-full bg-blue px-2.5 py-[5px] text-[11px] font-medium text-white transition-colors duration-150 hover:bg-blue-hi">
        <svg viewBox="0 0 16 16" className="size-3" fill="none" stroke="currentColor" strokeWidth={1.8} strokeLinecap="round">
          <path d="M6.6 9.4a2.8 2.8 0 0 0 4 0l2-2a2.8 2.8 0 1 0-4-4l-.6.6" />
          <path d="M9.4 6.6a2.8 2.8 0 0 0-4 0l-2 2a2.8 2.8 0 1 0 4 4l.6-.6" />
        </svg>
        Connect
      </button>
    </div>
  );
}
