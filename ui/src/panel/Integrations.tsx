import { useMemo, useState } from "react";

type Service = { name: string; tint: string; mark: string; dark?: boolean; blurb: string };

/**
 * The integrations browser.
 *
 * UI only for now -- nothing here connects to anything, and `Connect` is inert.
 *
 * The real tool servers used to sit at the top of this page, which put working
 * plumbing above a catalogue of things that do not work yet. They belong with the
 * agents that use them, and that is where they went.
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
