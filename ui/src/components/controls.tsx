import type { ReactNode } from "react";

/**
 * The controls, in one place.
 *
 * Every surface used to style its own buttons, which is how the interface ended
 * up with six greys and four button shapes -- each one reasonable beside what it
 * sat next to, and none of them the same as any other. A redesign laid over that
 * produces a redesign that looks half-applied, which is exactly what happened the
 * last time this was attempted.
 *
 * So there is one of each thing here and the surfaces use them. Changing how a
 * button looks is now a change in one file rather than a search.
 *
 * The shapes follow AppKit rather than the web: small, quiet, and only ever one
 * filled button in view -- macOS spends its accent colour on the single thing
 * you are most likely to want and leaves everything else as a surface.
 */

type ButtonKind = "plain" | "filled" | "danger";

/**
 * `sm` is AppKit's small control height and is what almost everything here
 * wants; `md` is for the one button a panel is actually about.
 */
export function Button({
  children,
  onClick,
  kind = "plain",
  size = "sm",
  icon,
  full,
  title,
}: {
  children: ReactNode;
  onClick?: () => void;
  kind?: ButtonKind;
  size?: "sm" | "md";
  icon?: ReactNode;
  full?: boolean;
  title?: string;
}) {
  const look =
    kind === "filled"
      ? "bg-blue text-white hover:bg-blue-hi"
      : kind === "danger"
        ? "bg-raise text-ink hover:bg-[#ff453a] hover:text-white"
        : "bg-raise text-ink hover:bg-raise-hi";
  return (
    <button
      onClick={onClick}
      title={title}
      className={[
        // `active:scale` rather than a colour change on press: a filled button
        // has nowhere darker to go, and the whole control moving is what reads
        // as a press on a touchpad.
        "inline-flex items-center justify-center gap-1.5 rounded-control font-medium",
        "transition-colors duration-150 active:scale-[0.98]",
        size === "sm" ? "h-[22px] px-2.5 text-[11.5px]" : "h-[28px] px-3.5 text-[12.5px]",
        full ? "w-full" : "",
        look,
      ].join(" ")}
    >
      {icon}
      {children}
    </button>
  );
}

/**
 * A button with no surface until it is pointed at.
 *
 * For the things every window has in its corner -- close, collapse, back. They
 * are always present and almost never the point, so they carry no weight until
 * somebody goes looking for them.
 */
export function Quiet({
  children,
  onClick,
  label,
}: {
  children: ReactNode;
  onClick?: () => void;
  label: string;
}) {
  return (
    <button
      onClick={onClick}
      aria-label={label}
      title={label}
      className="grid size-[20px] shrink-0 place-items-center rounded-md text-ink-3 transition-colors duration-150 hover:bg-raise-hi hover:text-ink"
    >
      {children}
    </button>
  );
}

/** A row of things on a card, with a hairline under all but the last. */
export function Rows({ children }: { children: ReactNode }) {
  return (
    <div className="overflow-hidden rounded-card bg-raise hairline [&>*+*]:border-t [&>*+*]:border-line">
      {children}
    </div>
  );
}

/**
 * The small uppercase label above a group.
 *
 * AppKit's section headers are sentence case and quiet; the shouting uppercase
 * label is a web habit. Kept small and low-contrast so it groups without
 * competing with the things it groups.
 */
export function Label({ children }: { children: ReactNode }) {
  return <p className="mb-1.5 px-0.5 text-[10.5px] text-ink-3">{children}</p>;
}

/** A hairline between two things that are not rows. */
export function Rule() {
  return <div className="my-2 h-px bg-line" />;
}
