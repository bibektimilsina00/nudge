import Image from "next/image";
import Link from "next/link";
import { Check } from "lucide-react";

import { Cat } from "@/components/cat";
import { Checksum, DownloadButton } from "@/components/download";
import { Nav } from "@/components/nav";
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@/components/ui/accordion";

/**
 * The landing page.
 *
 * One claim -- Nudge sees your screen and works it -- and one action, download,
 * offered in the bar, the hero and at the end. The order is the argument: what
 * you would actually say to it, then the thing itself, then how it works, then
 * the two permissions, then the questions somebody asks before installing
 * software that watches their screen.
 *
 * Deliberately absent: testimonials, logo strips, user counts. There are none,
 * and a page that invents social proof is a page for a product nobody should
 * let near their screen.
 */
export default function Home() {
  return (
    <div id="top">
      <a
        href="#download"
        className="sr-only focus-visible:not-sr-only focus-visible:fixed focus-visible:top-3 focus-visible:left-3 focus-visible:z-(--z-notch) focus-visible:rounded-full focus-visible:bg-accent focus-visible:px-4 focus-visible:py-2 focus-visible:text-sm focus-visible:font-medium focus-visible:text-white"
      >
        Skip to download
      </a>

      <Nav />

      <main>
        <Hero />
        <Uses />
        <Shown />
        <How />
        <Runs />
        <Permissions />
        <Questions />
        <LastCall />
      </main>

      <Footer />
    </div>
  );
}

function Section({
  id,
  children,
  className = "",
}: {
  id?: string;
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <section id={id} className={`border-t border-line scroll-mt-16 ${className}`}>
      <div className="mx-auto w-full max-w-[72rem] px-6 py-20">{children}</div>
    </section>
  );
}

function Hero() {
  const signals = [
    "Free, and no account",
    "Runs on your own model and API key",
    "Nothing routed through our servers",
  ];

  return (
    <section className="mx-auto w-full max-w-[72rem] px-6 pt-16 pb-20 sm:pt-24">
      <div className="grid items-center gap-12 lg:grid-cols-[1.25fr_1fr] lg:gap-8">
        <div>
          <h1 className="font-display text-[2.5rem] leading-[1.05] font-bold text-balance sm:text-[3.75rem]">
            An assistant that can see your screen.
          </h1>

          <p className="mt-6 max-w-[42rem] text-[1.0625rem] leading-relaxed text-pretty text-ink-2 sm:text-[1.125rem]">
          Hold one key and say what you want. Nudge reads what is in front of you
          and works it the way you would — finding the control, clicking it,
          typing into it — in whatever app you happen to be in.
        </p>

          <div id="download" className="mt-9 scroll-mt-24">
            <DownloadButton align="start" />
          </div>

          <ul className="mt-10 flex flex-col gap-2.5 sm:flex-row sm:flex-wrap sm:gap-x-7">
            {signals.map((s) => (
              <li key={s} className="flex items-center gap-2 text-[0.875rem] text-ink-2">
                <Check className="size-4 shrink-0 text-accent" aria-hidden />
                {s}
              </li>
            ))}
          </ul>
        </div>

        {/* Ordered after the text on a phone, beside it on a wide screen. */}
        <div className="order-first flex justify-center lg:order-none lg:justify-end">
          <Cat />
        </div>
      </div>
    </section>
  );
}

/**
 * What somebody would actually say to it.
 *
 * Leading with the product's capabilities in the abstract -- "agents", "MCP",
 * "computer use" -- describes the machinery to people who have not yet decided
 * they want it. These are sentences you could say out loud, which is also the
 * interface.
 */
function Uses() {
  const uses = [
    {
      said: "What is this error actually telling me?",
      does: "Reads the dialog in front of you and answers in plain language.",
    },
    {
      said: "Where is the setting for scaling in this app?",
      does: "Finds the buried menu item and points at it, so you learn where it was.",
    },
    {
      said: "Play something by Radiohead on YouTube.",
      does: "Opens the browser, searches, picks a result and presses play.",
    },
    {
      said: "Fill this form in with the details from that email.",
      does: "Reads one window, types into another, field by field.",
    },
    {
      said: "Write these notes up as a file in my workspace.",
      does: "Works inside the folder you chose and nowhere else.",
    },
    {
      said: "Summarise what is on my screen right now.",
      does: "One screenshot, read for the thing you asked about.",
    },
  ];

  return (
    <Section id="uses">
      <h2 className="max-w-[24ch] font-display text-[1.875rem] leading-tight font-semibold text-balance sm:text-[2.25rem]">
        Things you can say to it
      </h2>
      <p className="mt-3 max-w-[52ch] text-[0.9375rem] leading-relaxed text-pretty text-ink-3">
        Out loud, while holding the key. No syntax and nothing to memorise.
      </p>

      <ul className="mt-10 grid gap-px overflow-hidden rounded-xl bg-line sm:grid-cols-2 lg:grid-cols-3">
        {uses.map((u) => (
          <li key={u.said} className="bg-surface p-6">
            <p className="font-display text-[0.9375rem] leading-snug font-medium text-balance">
              “{u.said}”
            </p>
            <p className="mt-2.5 text-[0.875rem] leading-relaxed text-pretty text-ink-3">
              {u.does}
            </p>
          </li>
        ))}
      </ul>
    </Section>
  );
}

function Shown() {
  return (
    <Section>
      <div className="grid items-center gap-10 lg:grid-cols-[1.15fr_1fr] lg:gap-16">
        <div className="overflow-hidden rounded-xl bg-surface ring-1 ring-line">
          <Image
            src="/panel.png"
            alt="Nudge's panel, showing skills, keyboard shortcuts and integrations."
            width={1058}
            height={456}
            className="h-auto w-full"
          />
        </div>

        <div>
          <h2 className="max-w-[20ch] font-display text-[1.875rem] leading-tight font-semibold text-balance">
            It lives in the menu bar, not in your way
          </h2>
          <p className="mt-4 max-w-[46ch] text-[0.9375rem] leading-relaxed text-pretty text-ink-2">
            Point at the top of the screen and the panel opens; move away and it
            goes. There is no window to arrange and nothing in your Dock. The key
            works from anywhere, including over a full-screen app.
          </p>
        </div>
      </div>
    </Section>
  );
}

function How() {
  const beats = [
    {
      title: "You hold Control",
      body: "And talk. Let go when you are done — that is the whole interaction. Tap the same key instead of holding it and it takes the next step on its own.",
    },
    {
      title: "It looks at your screen",
      body: "One screenshot, read for the thing you asked about. It refuses to look at password managers, and at any window whose title names a secret.",
    },
    {
      title: "It does the thing",
      body: "Points at the control, or clicks it and types for you. You watch it happen, because it drives the real cursor. Escape stops it mid-action.",
    },
  ];

  return (
    <Section id="how">
      <h2 className="font-display text-[1.875rem] leading-tight font-semibold text-balance sm:text-[2.25rem]">
        How it works
      </h2>

      <ol className="mt-10 grid gap-10 sm:grid-cols-3 sm:gap-8">
        {beats.map((b, i) => (
          <li key={b.title}>
            <span
              aria-hidden
              className="font-mono text-[0.8125rem] tabular-nums text-accent"
            >
              {String(i + 1).padStart(2, "0")}
            </span>
            <h3 className="mt-3 font-display text-[1.0625rem] font-semibold">{b.title}</h3>
            <p className="mt-2 max-w-[44ch] text-[0.9375rem] leading-relaxed text-pretty text-ink-2">
              {b.body}
            </p>
          </li>
        ))}
      </ol>
    </Section>
  );
}

/** What it plugs into, stated plainly. */
function Runs() {
  const groups = [
    {
      head: "Models",
      items: ["Ollama, on your own machine", "Google Gemini", "Anthropic Claude"],
      note: "Your key, your bill, and your call on how hard it thinks.",
    },
    {
      head: "Tools",
      items: ["Any MCP server", "Skills — folders of instructions", "Your shell and files, if you allow it"],
      note: "Three lines of config per server, and it arrives with its own tools.",
    },
  ];

  return (
    <Section>
      <div className="grid gap-px overflow-hidden rounded-xl bg-line sm:grid-cols-2">
        {groups.map((g) => (
          <div key={g.head} className="bg-surface p-8">
            <h2 className="font-display text-[1.25rem] font-semibold">{g.head}</h2>
            <ul className="mt-5 space-y-2.5">
              {g.items.map((i) => (
                <li key={i} className="flex items-start gap-2.5 text-[0.9375rem] text-ink-2">
                  <Check className="mt-[3px] size-4 shrink-0 text-accent" aria-hidden />
                  {i}
                </li>
              ))}
            </ul>
            <p className="mt-6 max-w-[40ch] text-[0.875rem] leading-relaxed text-pretty text-ink-3">
              {g.note}
            </p>
          </div>
        ))}
      </div>
    </Section>
  );
}

/**
 * Raised before macOS raises it.
 *
 * Screen Recording plus Accessibility is a description of spyware, and somebody
 * meets that framing at the moment the system asks. A page that skips it gets
 * uninstalled thirty seconds later.
 */
function Permissions() {
  return (
    <Section id="permissions">
      <h2 className="max-w-[26ch] font-display text-[1.875rem] leading-tight font-semibold text-balance sm:text-[2.25rem]">
        It needs two permissions, and it means both
      </h2>

      <dl className="mt-10 grid gap-px overflow-hidden rounded-xl bg-line sm:grid-cols-2">
        <div className="bg-surface p-8">
          <dt className="font-display text-[1.0625rem] font-semibold">Screen Recording</dt>
          <dd className="mt-3 max-w-[48ch] text-[0.9375rem] leading-relaxed text-pretty text-ink-2">
            So it can see what you are pointing at. The screenshot goes to the
            model you configured and nowhere else. It refuses to look at password
            managers, or at any window whose title names a secret.
          </dd>
        </div>
        <div className="bg-surface p-8">
          <dt className="font-display text-[1.0625rem] font-semibold">Accessibility</dt>
          <dd className="mt-3 max-w-[48ch] text-[0.9375rem] leading-relaxed text-pretty text-ink-2">
            So it can click and type. It drives the one real cursor, so you watch
            everything it does while it does it — and Escape stops it mid-action,
            from anywhere.
          </dd>
        </div>
      </dl>

      <p className="mt-6 max-w-[62ch] text-[0.9375rem] leading-relaxed text-pretty text-ink-3">
        Everything past that stays off until you turn it on: running commands that
        change things, writing files outside its workspace, sending requests that
        are not just reads.
      </p>
    </Section>
  );
}

function Questions() {
  const qa = [
    {
      q: "macOS says it cannot check this app for malicious software. Is it safe?",
      a: "That message means the build has not been through Apple's notarisation service yet, which is being sorted out. It is not the result of a scan. Until then: drag Nudge to Applications, then run xattr -dr com.apple.quarantine /Applications/Nudge.app in Terminal. That clears the downloaded-from-the-internet flag and nothing else. The checksum below is published so you can confirm you got the file this page is offering.",
    },
    {
      q: "Does my screen go anywhere?",
      a: "To the model you configured, which is the thing doing the looking. Point it at Ollama and your screen never leaves the machine. Point it at Gemini or Anthropic and it goes to them, under your own API key. Nothing routes through a server of ours — there is not one in the path.",
    },
    {
      q: "Can I use my Mac while it is working?",
      a: "No, and that is what it means rather than a shortcoming. Nudge drives your actual cursor, so while it works it has the mouse and the keyboard, exactly as a person sitting at your laptop would. Escape takes them back.",
    },
    {
      q: "What does it run on?",
      a: "Apple silicon Macs on macOS 12 or later, and Linux as an AppImage — glibc 2.39 or newer, so Ubuntu 24.04, Fedora 40, Debian 13 and anything after them. The Linux build is X11: Wayland deliberately forbids the synthetic input Nudge is built on, and doing it properly there is its own piece of work. An Intel Mac needs its own compile and has not been done. Windows is written and has never been built.",
    },
    {
      q: "Do I need an account?",
      a: "Sign in with Google and Nudge uses its own model — nothing to set up, no key to find, no card. If you would rather not, paste your own Gemini API key into Settings and the app talks to Google directly, billed to you and never passing through us. One or the other: a model is what makes it work, and there is no third way to reach one.",
    },
    {
      q: "What does it cost?",
      a: "Nothing. You pay whoever makes the model you point it at, or nobody at all if you run Ollama on your own machine.",
    },
    {
      q: "How do I get rid of it?",
      a: "Drag it to the Bin. Its settings sit in ~/.config/nudge, and macOS drops the permissions once the app is gone.",
    },
  ];

  return (
    <Section id="faq">
      <div className="grid gap-10 lg:grid-cols-[1fr_1.6fr] lg:gap-16">
        <div>
          <h2 className="font-display text-[1.875rem] leading-tight font-semibold text-balance sm:text-[2.25rem]">
            Before you install it
          </h2>
          <p className="mt-3 max-w-[34ch] text-[0.9375rem] leading-relaxed text-pretty text-ink-3">
            The questions people ask about software that watches their screen.
          </p>
        </div>

        <div>
          <Accordion type="single" collapsible>
            {qa.map(({ q, a }) => (
              <AccordionItem key={q} value={q} className="border-line">
                <AccordionTrigger className="text-left font-display text-[0.9375rem] font-medium hover:no-underline">
                  {q}
                </AccordionTrigger>
                <AccordionContent className="max-w-[68ch] text-[0.9375rem] leading-relaxed text-pretty text-ink-2">
                  {a}
                </AccordionContent>
              </AccordionItem>
            ))}
          </Accordion>

          <div className="mt-8">
            <Checksum />
          </div>
        </div>
      </div>
    </Section>
  );
}

function LastCall() {
  return (
    <Section className="text-center">
      <h2 className="mx-auto max-w-[22ch] font-display text-[2rem] leading-tight font-bold text-balance sm:text-[2.5rem]">
        Free, no account, your own key
      </h2>
      <p className="mx-auto mt-4 max-w-[48ch] text-[0.9375rem] leading-relaxed text-pretty text-ink-2">
        Nothing to sign up for and nothing to cancel. If you do not get on with
        it, drag it to the Bin.
      </p>
      <div className="mt-9">
        <DownloadButton compact />
      </div>
    </Section>
  );
}

function Footer() {
  return (
    <footer className="border-t border-line">
      <div className="mx-auto flex w-full max-w-[72rem] flex-col gap-6 px-6 py-10 text-[0.8125rem] text-ink-3 sm:flex-row sm:items-center sm:justify-between">
        <div className="flex items-center gap-2.5">
          <Image src="/icon.png" alt="" width={20} height={20} className="rounded-[5px]" />
          <span className="tabular-nums">© {new Date().getFullYear()} Nuddg Inc</span>
        </div>

        <nav className="flex flex-wrap items-center gap-x-6 gap-y-2">
          <a href="#uses" className="transition-colors hover:text-ink">
            What it does
          </a>
          <a href="#permissions" className="transition-colors hover:text-ink">
            Permissions
          </a>
          <a href="#faq" className="transition-colors hover:text-ink">
            FAQ
          </a>
          <a
            href="https://github.com/bibektimilsina00/nudge"
            className="transition-colors hover:text-ink"
          >
            Source
          </a>
          <Link href="/privacy" className="transition-colors hover:text-ink">
            Privacy
          </Link>
          <Link href="/terms" className="transition-colors hover:text-ink">
            Terms
          </Link>
        </nav>
      </div>
    </footer>
  );
}
