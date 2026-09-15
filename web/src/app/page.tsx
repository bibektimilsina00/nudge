import Image from "next/image";

import { Checksum, DownloadButton } from "@/components/download";
import { Notch } from "@/components/notch";
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@/components/ui/accordion";

/**
 * The landing page.
 *
 * One claim: Nudge sees your screen and works it. That is the product and also
 * the thing somebody has to be talked into allowing, so it is said at the top
 * and answered honestly further down rather than eased around.
 *
 * One action: download. At the top and at the bottom, nowhere between. The only
 * other link is the source, in the header, where a competing call to action does
 * the least damage.
 *
 * Deliberately absent: testimonials, logo strips, user counts. There are none,
 * and a page that invents social proof is a page for a product nobody should let
 * near their screen.
 */
export default function Home() {
  return (
    <>
      {/* Visible the moment it is focused, and the first thing a keyboard lands
          on. The notch is sticky, so without this a tab through the page starts
          behind it. */}
      <a
        href="#download"
        className="sr-only focus-visible:not-sr-only focus-visible:fixed focus-visible:top-3 focus-visible:left-3 focus-visible:z-(--z-notch) focus-visible:rounded-full focus-visible:bg-gold focus-visible:px-4 focus-visible:py-2 focus-visible:text-sm focus-visible:font-medium focus-visible:text-black"
      >
        Skip to download
      </a>
      <Notch />
      <main className="mx-auto w-full max-w-[64rem] px-6 pb-24">
        <Hero />
        <Sequence />
        <Shown />
        <Reach />
        <Permissions />
        <Questions />
        <LastCall />
        <Footer />
      </main>
    </>
  );
}

function Hero() {
  return (
    <section className="pt-14 pb-20 text-center sm:pt-20">
      <h1 className="mx-auto max-w-[19ch] font-display text-[2.6rem] leading-[1.02] font-bold text-balance sm:text-[4.4rem]">
        Say it out loud. Watch your Mac do it.
      </h1>

      <p className="mx-auto mt-7 max-w-[54ch] text-[1.0625rem] leading-relaxed text-pretty text-ink-2">
        Nudge sits in the notch. Hold a key and talk, and it reads what is on your
        screen and works it the way you would — finding the control, clicking it,
        typing into it.
      </p>

      <div id="download" className="mt-10 flex scroll-mt-20 justify-center">
        <DownloadButton />
      </div>
    </section>
  );
}

/**
 * A real sequence, so it is drawn as one.
 *
 * Three things that happen in order, along a line, rather than three cards with
 * 01/02/03 on them. The rule is the information: it is what says these follow
 * one another instead of being a menu of features.
 */
function Sequence() {
  const beats = [
    {
      key: "⌃",
      title: "You hold Control",
      body: "And talk. Let go when you are done — that is the whole interaction.",
    },
    {
      key: "◉",
      title: "It looks at your screen",
      body: "One screenshot, read for the thing you asked about, in whatever app happens to be in front of you.",
    },
    {
      key: "⏎",
      title: "It does the thing",
      body: "Points at the control, or clicks it and types for you. Escape stops it mid-action, from anywhere.",
    },
  ];

  return (
    <section className="border-t border-line py-16">
      <h2 className="sr-only">How it works</h2>
      <ol className="relative grid gap-10 sm:grid-cols-3 sm:gap-8">
        {beats.map((b) => (
          <li key={b.title} className="relative sm:pt-8">
            {/* The line is only on the wide layout, where the three genuinely
                read left to right. Stacked on a phone, order is already clear. */}
            <span
              aria-hidden
              className="absolute top-[13px] left-0 hidden h-px w-full bg-line sm:block"
            />
            {/* The glyph repeats what the heading already says, so it is
                decoration as far as a screen reader is concerned. */}
            <span
              aria-hidden
              className="relative flex size-7 items-center justify-center rounded-full bg-surface-2 font-mono text-[13px] text-gold ring-1 ring-line sm:absolute sm:top-0 sm:left-0"
            >
              {b.key}
            </span>
            <h3 className="mt-4 font-display text-[1.0625rem] font-semibold sm:mt-0">
              {b.title}
            </h3>
            <p className="mt-2 max-w-[42ch] text-[0.9375rem] leading-relaxed text-pretty text-ink-2">
              {b.body}
            </p>
          </li>
        ))}
      </ol>
    </section>
  );
}

/** The thing itself. A product you look at, shown rather than described. */
function Shown() {
  return (
    <section className="border-t border-line py-16">
      <div className="overflow-hidden rounded-xl bg-surface ring-1 ring-line">
        <Image
          src="/panel.png"
          alt="Nudge's panel open beneath the notch, showing skills, keyboard shortcuts and integrations."
          width={1058}
          height={456}
          className="h-auto w-full"
        />
      </div>
      <p className="mt-4 max-w-[52ch] text-[0.9375rem] leading-relaxed text-pretty text-ink-3">
        Point at the notch and this opens. Move away and it goes. Everything else
        lives behind the one key.
      </p>
    </section>
  );
}

function Reach() {
  const things = [
    {
      title: "It finishes whole tasks",
      body: "Ask for an outcome rather than a next click and it goes away and does it — opening things, filling them in, telling you where it got to. After forty steps it gives up rather than clicking forever.",
    },
    {
      title: "It uses your tools",
      body: "Any Model Context Protocol server is three lines of config and arrives with its own tools. Skills are folders of instructions, the same shape the rest of the agent world uses, so one you already have works here.",
    },
    {
      title: "It runs on your model",
      body: "Ollama on your own machine, or Gemini, or Anthropic. Your key, your bill, and your call on how hard it thinks. Nothing passes through a server of ours.",
    },
  ];

  return (
    <section className="border-t border-line py-16">
      <h2 className="sr-only">What it can do</h2>
      <div className="grid gap-x-8 gap-y-10 sm:grid-cols-3">
        {things.map((t) => (
          <div key={t.title}>
            <h3 className="font-display text-[1.0625rem] font-semibold">{t.title}</h3>
            <p className="mt-2 text-[0.9375rem] leading-relaxed text-pretty text-ink-2">
              {t.body}
            </p>
          </div>
        ))}
      </div>
    </section>
  );
}

/**
 * Raised before macOS raises it.
 *
 * Screen Recording plus Accessibility is a description of spyware, and the
 * person will meet that framing at the moment the system asks. A page that skips
 * it gets uninstalled thirty seconds later.
 */
function Permissions() {
  return (
    <section className="border-t border-line py-16">
      <h2 className="max-w-[24ch] font-display text-[1.75rem] leading-tight font-semibold text-balance">
        It needs two permissions, and it means both of them
      </h2>

      <dl className="mt-8 grid gap-px overflow-hidden rounded-xl bg-line sm:grid-cols-2">
        <div className="bg-surface p-6">
          <dt className="font-display text-[0.9375rem] font-semibold">Screen Recording</dt>
          <dd className="mt-2 text-[0.9375rem] leading-relaxed text-pretty text-ink-2">
            So it can see what you are pointing at. The screenshot goes to the
            model you configured and nowhere else. It refuses to look at password
            managers, or at any window whose title names a secret.
          </dd>
        </div>
        <div className="bg-surface p-6">
          <dt className="font-display text-[0.9375rem] font-semibold">Accessibility</dt>
          <dd className="mt-2 text-[0.9375rem] leading-relaxed text-pretty text-ink-2">
            So it can click and type. It drives the one real cursor, so you watch
            everything it does while it does it — and Escape stops it mid-action,
            from anywhere.
          </dd>
        </div>
      </dl>

      <p className="mt-6 max-w-[60ch] text-[0.9375rem] leading-relaxed text-pretty text-ink-3">
        Everything past that stays off until you turn it on: running commands that
        change things, writing files outside its workspace, sending requests that
        are not just reads.
      </p>
    </section>
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
      q: "Which Macs does it run on?",
      a: "Apple silicon, macOS 12 or later. An Intel build needs its own compile and has not been done. Windows is next — that code is written and has never been compiled for it.",
    },
    {
      q: "Do I need an account?",
      a: "No. There is no sign-up, no licence key, nothing to activate. Download it and it runs.",
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
    <section className="border-t border-line py-16">
      <h2 className="font-display text-[1.75rem] leading-tight font-semibold text-balance">
        Before you install it
      </h2>

      <Accordion type="single" collapsible className="mt-6">
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
    </section>
  );
}

/**
 * The same action again, where somebody has finished reading and has no reason
 * to scroll back up to act on it. Not a second offer -- the identical one.
 */
function LastCall() {
  return (
    <section className="border-t border-line py-20 text-center">
      <h2 className="mx-auto max-w-[20ch] font-display text-[2rem] leading-tight font-bold text-balance">
        Free, no account, your own key
      </h2>
      <p className="mx-auto mt-3 max-w-[46ch] text-[0.9375rem] leading-relaxed text-pretty text-ink-2">
        Nothing to sign up for and nothing to cancel. If you do not get on with
        it, drag it to the Bin.
      </p>
      <div className="mt-9 flex justify-center">
        <DownloadButton compact />
      </div>
    </section>
  );
}

function Footer() {
  return (
    <footer className="flex flex-col gap-3 border-t border-line pt-10 text-[0.8125rem] text-ink-3 sm:flex-row sm:items-center sm:justify-between">
      <p className="max-w-[46ch] text-pretty">
        An early build. Things will break, and it will say so when they do.
      </p>
      <div className="flex items-center gap-5">
        <a
          href="https://github.com/bibektimilsina00/nudge"
          className="transition-colors hover:text-ink"
        >
          Source
        </a>
        <span className="tabular-nums">© {new Date().getFullYear()} Nuddg Inc</span>
      </div>
    </footer>
  );
}
