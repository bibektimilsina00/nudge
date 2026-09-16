import Image from "next/image";
import Link from "next/link";

/**
 * Chrome for the two legal pages.
 *
 * Not the site's `Nav`: every link in it is a hash anchor into the landing page,
 * which from `/privacy` scrolls nowhere. A logo that goes home is the whole
 * requirement here, so that is all this is.
 *
 * The measure is narrower than the landing page on purpose -- this is the only
 * part of the site somebody reads a paragraph at a time.
 */
export default function LegalLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="min-h-dvh">
      <header className="border-b border-line">
        <div className="mx-auto flex h-16 w-full max-w-[44rem] items-center px-6">
          <Link href="/" className="flex items-center gap-2.5">
            <Image src="/icon.png" alt="" width={28} height={28} className="rounded-[7px]" />
            <span className="font-display text-[0.9375rem] font-semibold" translate="no">
              Nudge
            </span>
          </Link>
        </div>
      </header>

      <main className="mx-auto w-full max-w-[44rem] px-6 py-16">{children}</main>

      <footer className="border-t border-line">
        <div className="mx-auto flex w-full max-w-[44rem] flex-wrap items-center gap-x-6 gap-y-2 px-6 py-10 text-[0.8125rem] text-ink-3">
          <span className="tabular-nums">© {new Date().getFullYear()} Nuddg Inc</span>
          <Link href="/privacy" className="transition-colors hover:text-ink">
            Privacy
          </Link>
          <Link href="/terms" className="transition-colors hover:text-ink">
            Terms
          </Link>
          <Link href="/" className="transition-colors hover:text-ink">
            Home
          </Link>
        </div>
      </footer>
    </div>
  );
}
