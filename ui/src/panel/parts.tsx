import type { ReactNode } from "react";

/** A labelled group of rows.
 *
 * Sentence case, not letter-spaced capitals. AppKit's section headers are quiet
 * and ordinary; uppercase with tracking is a web habit that reads as a banner
 * rather than as a way of grouping the things underneath it. */
export function Section({
  title,
  children,
  grid,
}: {
  title: string;
  children: ReactNode;
  /** Two across instead of one down. For things that are glanced at rather than
   *  read -- a past run is recognised by its shape and colour long before
   *  anybody reads its title. */
  grid?: boolean;
}) {
  return (
    <section className="pt-4 first:pt-2">
      <h2 className="mb-1.5 pl-1 text-[11px] font-medium text-ink-3">{title}</h2>
      <div className={grid ? "grid grid-cols-2 gap-2" : "space-y-2"}>{children}</div>
    </section>
  );
}

/**
 * One settings row. A button when it does something, a plain div when it does not
 * -- so a placeholder never gets a hover state promising an action it will not
 * perform.
 */
export function Row({
  icon,
  label,
  sub,
  value,
  badge,
  chevron,
  danger,
  compact,
  trailing,
  onClick,
}: {
  icon?: ReactNode;
  label: string;
  sub?: string;
  value?: string;
  badge?: string;
  chevron?: boolean;
  danger?: boolean;
  compact?: boolean;
  trailing?: ReactNode;
  onClick?: () => void;
}) {
  const Tag = onClick ? "button" : "div";
  return (
    <Tag
      onClick={onClick}
      className={[
        "flex w-full items-center gap-2.5 rounded-xl bg-raise px-2.5 text-left",
        "hairline",
        compact ? "h-[38px]" : sub ? "py-2" : "h-[40px]",
        onClick ? "transition-colors duration-150 hover:bg-raise-hi" : "",
        danger ? "text-[#ff5f57]" : "",
      ].join(" ")}
    >
      {icon && <span className={danger ? "text-[#ff5f57]" : "text-ink-2"}>{icon}</span>}
      <span className="min-w-0 flex-1">
        <span className="flex items-center gap-1.5">
          <span className="truncate text-[12px]">{label}</span>
          {badge && (
            <span className="rounded bg-accent px-1 py-px text-[8.5px] font-bold tracking-wide text-black">
              {badge}
            </span>
          )}
        </span>
        {sub && <span className="mt-0.5 block text-[10.5px] leading-snug text-ink-3">{sub}</span>}
      </span>
      {value && <span className="max-w-[130px] shrink-0 truncate text-[11px] text-ink-3">{value}</span>}
      {trailing}
      {chevron && <span className="shrink-0 text-[11px] text-ink-3">›</span>}
    </Tag>
  );
}

export function Toggle({ on, onChange }: { on: boolean; onChange: (on: boolean) => void }) {
  return (
    <button
      role="switch"
      aria-checked={on}
      onClick={() => onChange(!on)}
      className={`h-[21px] w-[36px] shrink-0 rounded-full p-0.5 transition-colors duration-200 ${
        on ? "bg-blue" : "bg-white/15"
      }`}
    >
      <span
        className={`block size-[17px] rounded-full bg-white transition-transform duration-200 ease-[cubic-bezier(0.23,1,0.32,1)] ${
          on ? "translate-x-[15px]" : "translate-x-0"
        }`}
      />
    </button>
  );
}

/**
 * Pick one of a few.
 *
 * The rows this replaces either showed a value with no way to change it, or
 * changed it by cycling on click -- which is the worst of both: you cannot see
 * what the alternatives are, you cannot go back without going all the way round,
 * and there is no way to tell a setting with three states from a button.
 *
 * So every option is on screen with what it costs beside it. These are choices
 * between trades -- a slower free model against a fast paid one, no thinking
 * tokens against thousands -- and an option list that hides the trade is asking
 * people to guess.
 */
export function Choice<T extends string>({
  options,
  value,
  onChange,
}: {
  options: { key: T; label: string; about?: string }[];
  value: T;
  onChange: (v: T) => void;
}) {
  return (
    <div className="overflow-hidden rounded-xl bg-raise hairline [&>*+*]:border-t [&>*+*]:border-line">
      {options.map((o) => {
        const on = o.key === value;
        return (
          <button
            key={o.key}
            onClick={() => onChange(o.key)}
            aria-pressed={on}
            className="flex w-full items-start gap-2.5 px-2.5 py-2 text-left transition-colors duration-150 hover:bg-raise-hi"
          >
            {/* A tick, not a radio ring. macOS marks the chosen item in a list
                with a check and leaves the others blank, rather than drawing an
                empty control beside everything you did not pick. */}
            <span
              className={`mt-px grid size-[15px] shrink-0 place-items-center text-blue transition-opacity duration-150 ${
                on ? "opacity-100" : "opacity-0"
              }`}
            >
              <svg viewBox="0 0 16 16" className="size-[13px]" fill="none" stroke="currentColor" strokeWidth={2.2} strokeLinecap="round" strokeLinejoin="round">
                <path d="m3 8.5 3.5 3.5L13 5" />
              </svg>
            </span>
            <span className="min-w-0 flex-1">
              <span className={`block text-[12px] ${on ? "font-medium text-white" : "text-ink-2"}`}>
                {o.label}
              </span>
              {o.about && (
                <span className="mt-px block text-[10.5px] leading-snug text-ink-3">{o.about}</span>
              )}
            </span>
          </button>
        );
      })}
    </div>
  );
}

/** A page inside settings: a back arrow, a title, and whatever it is about. */
export function Page({
  title,
  onBack,
  children,
}: {
  title: string;
  onBack: () => void;
  children: ReactNode;
}) {
  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <header className="flex items-center gap-2.5 px-3 pt-2.5 pb-1">
        <button
          onClick={onBack}
          aria-label="Back"
          className="grid size-[21px] shrink-0 place-items-center rounded-full bg-raise text-ink-2 transition-colors duration-150 hover:bg-raise-hi hover:text-white"
        >
          <svg viewBox="0 0 16 16" className="size-3" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
            <path d="M9.5 3.5 5 8l4.5 4.5" />
          </svg>
        </button>
        <h2 className="text-[13px] font-semibold tracking-tight">{title}</h2>
      </header>
      <div className="min-h-0 flex-1 overflow-y-auto px-3 pb-3">{children}</div>
    </div>
  );
}
