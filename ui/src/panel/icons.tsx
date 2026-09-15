import type { ReactNode } from "react";

const s = {
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 1.6,
  strokeLinecap: "round" as const,
  strokeLinejoin: "round" as const,
};

const Ico = ({ children }: { children: ReactNode }) => (
  <svg viewBox="0 0 16 16" className="size-[15px]" {...s}>
    {children}
  </svg>
);

export const Dot = ({ className }: { className: string }) => (
  <span className={`size-3.5 rounded-full ${className}`} />
);
export const Bulb = () => <Ico><path d="M6 12h4M6.5 14h3M8 2a4 4 0 0 0-2.4 7.2c.3.3.4.6.4 1h4c0-.4.1-.7.4-1A4 4 0 0 0 8 2Z" /></Ico>;
export const Bug = () => <Ico><path d="M5 6a3 3 0 0 1 6 0v3a3 3 0 0 1-6 0Z" /><path d="M2.5 7h2.5M11 7h2.5M2.5 11h2.5M11 11h2.5M6 4 5 2.5M10 4l1-1.5" /></Ico>;
export const Refresh = () => <Ico><path d="M13.5 8a5.5 5.5 0 1 1-1.6-3.9" /><path d="M13.5 3v3h-3" /></Ico>;
export const Spark = () => <Ico><path d="M8 2.2 9.3 6 13 7.3 9.3 8.6 8 12.4 6.7 8.6 3 7.3 6.7 6Z" /></Ico>;
export const Grid = () => <Ico><rect x="2.5" y="2.5" width="4.5" height="4.5" rx="1" /><rect x="9" y="2.5" width="4.5" height="4.5" rx="1" /><rect x="2.5" y="9" width="4.5" height="4.5" rx="1" /><rect x="9" y="9" width="4.5" height="4.5" rx="1" /></Ico>;
export const Bolt = () => <Ico><path d="M9 2 4 9h3.5L7 14l5-7H8.5Z" /></Ico>;
export const TextIcon = () => <Ico><path d="M3 4h10M8 4v9M6 13h4" /></Ico>;
export const Keyboard = () => <Ico><rect x="1.8" y="4" width="12.4" height="8" rx="1.5" /><path d="M5 9.6h6M4.4 7h.1M7 7h.1M9.6 7h.1M12 7h.1" /></Ico>;
export const Arrow = () => <Ico><path d="M4 2.5 12 8l-3.6.9L10 13l-1.8.8-1.6-4.1L4 12Z" /></Ico>;
export const Wave = () => <Ico><path d="M2.5 8h1.5M6 4.5v7M9 6v4M12 3.5v9M14.5 7v2" /></Ico>;
export const MicIcon = () => <Ico><rect x="6" y="2" width="4" height="7" rx="2" /><path d="M4 8a4 4 0 0 0 8 0M8 12v2" /></Ico>;
export const Agent = () => <Ico><rect x="2.5" y="3.5" width="11" height="9" rx="2" /><circle cx="8" cy="8" r="1.8" /></Ico>;
export const DockIcon = () => <Ico><rect x="2" y="4" width="12" height="8" rx="1.5" /><path d="M2 9.5h12" /></Ico>;
export const Copy = () => <Ico><rect x="2.5" y="4.5" width="8" height="9" rx="1.5" /><path d="M5 4.5v-.8a1.2 1.2 0 0 1 1.2-1.2h6.1a1.2 1.2 0 0 1 1.2 1.2v6.1a1.2 1.2 0 0 1-1.2 1.2h-.8" /></Ico>;
export const Out = () => <Ico><path d="M10 11.5v1.3a1.2 1.2 0 0 1-1.2 1.2H3.7a1.2 1.2 0 0 1-1.2-1.2V3.2A1.2 1.2 0 0 1 3.7 2h5.1A1.2 1.2 0 0 1 10 3.2v1.3M7 8h6.5m0 0-2-2m2 2-2 2" /></Ico>;
export const Trash = () => <Ico><path d="M2.8 4.3h10.4M6 4.3V3a.8.8 0 0 1 .8-.8h2.4a.8.8 0 0 1 .8.8v1.3M4.3 4.3l.6 8.3a1 1 0 0 0 1 .9h4.2a1 1 0 0 0 1-.9l.6-8.3" /></Ico>;
export const Power = () => <Ico><path d="M8 2v5.5M4.6 4.4a5 5 0 1 0 6.8 0" /></Ico>;
export const Shield = () => <Ico><path d="M8 1.8 13.2 4v4c0 3-2.2 5.3-5.2 6.2C5 13.3 2.8 11 2.8 8V4Z" /></Ico>;
