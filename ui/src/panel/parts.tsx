import type { ReactNode } from "react";

/** A labelled group of rows.
 *
 * Sentence case, not letter-spaced capitals. AppKit's section headers are quiet
 * and ordinary; uppercase with tracking is a web habit that reads as a banner
 * rather than as a way of grouping the things underneath it. */
export function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="pt-4 first:pt-2">
      <h2 className="mb-1.5 pl-1 text-[11px] font-medium text-ink-3">
        {title}
      </h2>
      <div className="space-y-2">{children}</div>
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
