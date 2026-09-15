"use client";

import Image from "next/image";
import { useEffect, useState } from "react";

import { cn } from "@/lib/utils";

const links = [
  { href: "#uses", label: "What it does" },
  { href: "#how", label: "How it works" },
  { href: "#permissions", label: "Permissions" },
  { href: "#faq", label: "FAQ" },
];

/**
 * The app bar.
 *
 * Transparent over the hero and solid once the page moves, which is the ordinary
 * behaviour and the reason it is worth doing: it keeps the top of the page open
 * while somebody reads the headline, then gives the bar a surface to sit on so
 * text does not run underneath it.
 *
 * Scroll state is read from a listener rather than an IntersectionObserver
 * because there is one threshold and no element to observe.
 */
export function Nav() {
  const [moved, setMoved] = useState(false);

  useEffect(() => {
    const onScroll = () => setMoved(window.scrollY > 8);
    onScroll();
    window.addEventListener("scroll", onScroll, { passive: true });
    return () => window.removeEventListener("scroll", onScroll);
  }, []);

  return (
    <header
      className={cn(
        "sticky top-0 z-(--z-notch) transition-colors duration-200",
        moved ? "border-b border-line bg-void/80 backdrop-blur-xl" : "border-b border-transparent",
      )}
    >
      <nav className="mx-auto flex h-16 w-full max-w-[72rem] items-center gap-8 px-6">
        <a href="#top" className="flex shrink-0 items-center gap-2.5">
          <Image src="/icon.png" alt="" width={28} height={28} className="rounded-[7px]" priority />
          <span className="font-display text-[0.9375rem] font-semibold" translate="no">
            Nudge
          </span>
        </a>

        {/* Hidden rather than collapsed into a menu: four anchors on a
            single-page site do not need a drawer, and the download is what
            matters on a phone. */}
        <ul className="hidden flex-1 items-center gap-7 md:flex">
          {links.map((l) => (
            <li key={l.href}>
              <a
                href={l.href}
                className="text-[0.875rem] text-ink-2 transition-colors hover:text-ink"
              >
                {l.label}
              </a>
            </li>
          ))}
        </ul>

        <div className="ml-auto flex items-center gap-2 md:ml-0">
          <a
            href="https://github.com/bibektimilsina00/nudge"
            className="hidden px-2 text-[0.875rem] text-ink-2 transition-colors hover:text-ink sm:block"
          >
            Source
          </a>
          <a
            href="#download"
            className="rounded-full bg-ink px-4 py-2 text-[0.875rem] font-medium text-void transition-colors hover:bg-ink/90"
          >
            Download
          </a>
        </div>
      </nav>
    </header>
  );
}
