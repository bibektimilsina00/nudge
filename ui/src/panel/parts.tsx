import type { ReactNode } from "react";

/**
 * A labelled group of rows.
 *
 * One container with hairlines between the rows, rather than a stack of
 * separately floating cards. A card says "this is its own object"; rows in a
 * group say "these belong together and you read them downward", which is what a
 * settings list actually is. It is also quieter: one border instead of six.
 */
export function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="pt-3.5 first:pt-1.5">
      <h2 className="mb-1 px-1.5 text-[9.5px] font-medium tracking-[0.09em] text-white/30 uppercase">
        {title}
      </h2>
      <div className="divide-y divide-hair overflow-hidden rounded-[10px] bg-raised inset-ring-1 inset-ring-hair">
        {children}
      </div>
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
        // Flat: the group around it carries the surface, so a row only has to
        // carry its own hover.
        "flex w-full items-center gap-2.5 px-2.5 text-left",
        compact ? "h-[34px]" : sub ? "py-[7px]" : "h-[36px]",
        onClick ? "transition-colors duration-150 hover:bg-hover" : "",
        danger ? "text-[#ff5f57]" : "",
      ].join(" ")}
    >
      {icon && <span className={danger ? "text-[#ff5f57]" : "text-white/45"}>{icon}</span>}
      <span className="min-w-0 flex-1">
        <span className="flex items-center gap-1.5">
          <span className="truncate text-[11.5px]">{label}</span>
          {badge && (
            <span className="rounded bg-accent px-1 py-px text-[8.5px] font-bold tracking-wide text-white">
              {badge}
            </span>
          )}
        </span>
        {sub && <span className="mt-0.5 block text-[10.5px] leading-snug text-white/35">{sub}</span>}
      </span>
      {value && <span className="max-w-[130px] shrink-0 truncate text-[11px] text-white/40">{value}</span>}
      {trailing}
      {chevron && <span className="shrink-0 text-[11px] text-white/25">›</span>}
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
        on ? "bg-[#0a84ff]" : "bg-white/15"
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
