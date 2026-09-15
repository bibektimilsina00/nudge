import Image from "next/image";
import { Keyboard, Lock, MousePointer2, Puzzle, ScanEye, TerminalSquare } from "lucide-react";

import { Checksum, DownloadButton } from "@/components/download";
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@/components/ui/accordion";
import { Badge } from "@/components/ui/badge";

/**
 * The landing page.
 *
 * Written around one claim, because a page that makes four makes none: Nudge can
 * see the screen and act on it. That is the product and also the thing somebody
 * has to be persuaded to allow, so it is said at the top rather than eased into.
 *
 * One primary action throughout -- download -- repeated at the top and the
 * bottom and nowhere in between. The only other link on the page is the source,
 * in the header, where a competing call to action does the least harm.
 *
 * What it deliberately does not have: testimonials, logo strips, user counts.
 * There are none, and a landing page that invents social proof is a landing page
 * for a product nobody should trust with their screen.
 */
export default function Home() {
  return (
    <main className="mx-auto flex min-h-dvh max-w-5xl flex-col px-6">
      <Header />
      <Hero />
      <Steps />
      <Features />
      <Permissions />
      <Questions />
      <LastCall />
      <Footer />
    </main>
  );
}

function Header() {
  return (
    <header className="flex items-center justify-between py-6">
      <div className="flex items-center gap-2.5">
        <Image src="/icon.png" alt="" width={32} height={32} className="rounded-[7px]" priority />
        <span className="text-[15px] font-semibold">Nudge</span>
      </div>
      <a
        href="https://github.com/bibektimilsina00/nudge"
        className="text-sm text-muted-foreground transition-colors hover:text-foreground"
      >
        Source
      </a>
    </header>
  );
}

function Hero() {
  return (
    <section className="flex flex-col items-center pt-16 pb-12 text-center sm:pt-24">
      <Badge variant="secondary" className="mb-6 font-normal">
        Early build · macOS
      </Badge>

      <h1 className="max-w-3xl text-4xl font-semibold text-balance sm:text-6xl">
        Hold a key. Say what you want. Watch it happen.
      </h1>

      {/* The second sentence does the work the headline cannot: what it actually
          is. "AI assistant" says nothing; "sees your screen and uses it" is the
          whole difference. */}
      <p className="mt-6 max-w-2xl text-lg text-pretty text-muted-foreground">
        Nudge lives in your Mac&apos;s notch. It sees what is on your screen and
        works it the way you would — pointing, clicking, typing — so the thing you
        cannot find a menu for gets done anyway.
      </p>

      <div className="mt-10">
        <DownloadButton />
      </div>

      {/* The thing itself, because this is a product you look at.
          A page selling something that reads your screen, with no picture of it,
          asks people to imagine the one thing they are being asked to trust. */}
      <div className="mt-14 w-full">
        <div className="overflow-hidden rounded-xl border bg-neutral-950 shadow-2xl">
          <Image
            src="/panel.png"
            alt="Nudge's panel open under the notch, showing skills, keyboard shortcuts and integrations."
            width={1058}
            height={456}
            className="h-auto w-full"
            priority
          />
        </div>
        <p className="mt-3 text-xs text-muted-foreground">
          It hangs from the notch. Point at it and it opens; move away and it goes.
        </p>
      </div>
    </section>
  );
}

function Steps() {
  const steps = [
    {
      icon: Keyboard,
      title: "Hold Control",
      body: "One key, held. Talk while you hold it, let go when you are done.",
    },
    {
      icon: ScanEye,
      title: "It looks",
      body: "A screenshot, read for what you asked about. Whatever is in front of you, in whatever app.",
    },
    {
      icon: MousePointer2,
      title: "It acts",
      body: "Points at the control, or clicks it and types for you. Escape stops it, from anywhere.",
    },
  ];

  return (
    <section className="border-t py-16">
      <h2 className="sr-only">How it works</h2>
      <div className="grid gap-10 sm:grid-cols-3">
        {steps.map(({ icon: Icon, title, body }, i) => (
          <div key={title}>
            <div className="flex items-center gap-2.5">
              <Icon className="size-[18px] text-muted-foreground" aria-hidden />
              <span className="font-mono text-xs tabular-nums text-muted-foreground">
                0{i + 1}
              </span>
            </div>
            <h3 className="mt-3 font-medium">{title}</h3>
            <p className="mt-1.5 text-sm leading-relaxed text-pretty text-muted-foreground">
              {body}
            </p>
          </div>
        ))}
      </div>
    </section>
  );
}

function Features() {
  const features = [
    {
      icon: TerminalSquare,
      title: "Agents that finish the job",
      body: "Ask for a whole task rather than a next click and Nudge takes it away — opening things, filling them in, reporting back as it goes. It gives up after forty steps rather than clicking forever.",
    },
    {
      icon: Puzzle,
      title: "Your tools, not ours",
      body: "Any Model Context Protocol server is three lines of config and brings its own tools. Skills are folders with instructions in them — the same shape the rest of the agent world already uses, so one you already have works here.",
    },
    {
      icon: Lock,
      title: "Bring your own model",
      body: "Ollama on your own machine, or Gemini, or Anthropic. Your key, your bill, your choice of how hard it thinks. Nothing is proxied through us.",
    },
  ];

  return (
    <section className="border-t py-16">
      <h2 className="sr-only">What it does</h2>
      <div className="grid gap-8 sm:grid-cols-3">
        {features.map(({ icon: Icon, title, body }) => (
          <div key={title}>
            <Icon className="size-5 text-muted-foreground" aria-hidden />
            <h3 className="mt-3 font-medium">{title}</h3>
            <p className="mt-1.5 text-sm leading-relaxed text-pretty text-muted-foreground">
              {body}
            </p>
          </div>
        ))}
      </div>
    </section>
  );
}

/**
 * Said before it is asked for, not after.
 *
 * Nudge needs Screen Recording and Accessibility, which is a description of
 * spyware, and somebody is going to notice that at the moment macOS asks. A
 * download page that skips it is a page that gets uninstalled thirty seconds
 * later; one that raises it first is the only way to be believed.
 */
function Permissions() {
  return (
    <section className="border-t py-16">
      <h2 className="text-xl font-semibold text-balance">
        It asks for two permissions, and it means them
      </h2>
      <div className="mt-6 grid gap-6 sm:grid-cols-2">
        <div className="rounded-lg border p-5">
          <h3 className="text-sm font-medium">Screen Recording</h3>
          <p className="mt-1.5 text-sm leading-relaxed text-pretty text-muted-foreground">
            So it can see what you are pointing at. Screenshots go to whichever
            model you configured and nowhere else. It refuses to look at password
            managers, and at windows whose title names a secret.
          </p>
        </div>
        <div className="rounded-lg border p-5">
          <h3 className="text-sm font-medium">Accessibility</h3>
          <p className="mt-1.5 text-sm leading-relaxed text-pretty text-muted-foreground">
            So it can click and type. It drives the one real cursor, which means
            you can see everything it does as it does it — and Escape stops it
            mid-action, from anywhere.
          </p>
        </div>
      </div>
      <p className="mt-6 text-sm text-pretty text-muted-foreground">
        Anything beyond that is off until you turn it on: running commands that
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
      a: "That message means the build has not been through Apple's notarisation service yet, which is being sorted out. It is not a scan result. Until then, drag Nudge to Applications and run: xattr -dr com.apple.quarantine /Applications/Nudge.app — that removes the downloaded-from-the-internet flag and nothing else. The checksum below is there so you can verify you got the file this page is offering.",
    },
    {
      q: "Does my screen go anywhere?",
      a: "To the model you configured, which is the one doing the looking. If that is Ollama it never leaves your machine. If it is Gemini or Anthropic it goes to them under your own API key. Nothing routes through a server of ours — there is not one in the path.",
    },
    {
      q: "Can I use my Mac while it is working?",
      a: "No, and that is what it means rather than a limitation. Nudge drives your actual cursor, so while an agent runs it has the mouse and keyboard, the same as a person sitting at your laptop would. The stop is always one key away.",
    },
    {
      q: "Which Macs?",
      a: "Apple silicon, macOS 12 or later. An Intel build needs its own compile and is not done. Windows is next — the code is written and has never been compiled for it.",
    },
    {
      q: "Do I need an account?",
      a: "No. There is no sign-up, no licence key and nothing to activate. Download it and it runs.",
    },
    {
      q: "What does it cost?",
      a: "Nothing. You pay whoever makes the model you point it at, or nobody at all if you run Ollama locally.",
    },
    {
      q: "How do I remove it?",
      a: "Drag it to the Bin. Its settings live in ~/.config/nudge, and macOS forgets the permissions when the app is gone.",
    },
  ];

  return (
    <section className="border-t py-16">
      <h2 className="text-xl font-semibold text-balance">Before you download it</h2>
      <Accordion type="single" collapsible className="mt-4">
        {qa.map(({ q, a }) => (
          <AccordionItem key={q} value={q}>
            <AccordionTrigger className="text-left text-sm">{q}</AccordionTrigger>
            <AccordionContent className="text-sm leading-relaxed text-pretty text-muted-foreground">
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
 * The same action again, at the point somebody has finished reading.
 *
 * Not a second offer -- the identical one. Somebody who scrolled the whole page
 * has answered their own objections and should not have to scroll back up to
 * act on that.
 */
function LastCall() {
  return (
    <section className="border-t py-16 text-center">
      <h2 className="text-2xl font-semibold text-balance">
        Free, no account, and it runs on your own key
      </h2>
      <p className="mx-auto mt-3 max-w-md text-sm text-pretty text-muted-foreground">
        Nothing to sign up for and nothing to cancel. If you do not like it, drag
        it to the Bin.
      </p>
      <div className="mt-8 flex justify-center">
        <DownloadButton compact />
      </div>
    </section>
  );
}

function Footer() {
  return (
    <footer className="mt-auto flex flex-col items-center gap-2 border-t py-10 text-xs text-muted-foreground">
      <p className="text-pretty">
        Nudge is an early build. Things will break, and it will tell you when they do.
      </p>
      <p className="tabular-nums">© {new Date().getFullYear()} Nuddg Inc</p>
    </footer>
  );
}
