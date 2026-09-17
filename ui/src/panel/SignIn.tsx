import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Companion } from "../components/Companion";

/**
 * The first thing anybody sees.
 *
 * It is the whole panel, not a page inside it: no rail, no sections. Until
 * there is an account there is nowhere else to be, and drawing the furniture of
 * an app somebody cannot use yet is an invitation to press things that will not
 * work.
 *
 * The companion is the hero because the companion is the product -- this is the
 * thing that will be living in their menu bar, and it is the only part of the
 * screen that could not belong to any other app. Everything under it is kept
 * deliberately quiet so that it stays that way: two buttons, no illustration
 * competing with the one that matters, no list of features being sold to
 * somebody who has already downloaded it.
 */

/** Who is signed in. Mirrors `core::account::Account`. */
export type Account = {
  provider: string;
  email: string;
  name: string;
  avatar_url: string;
};

type Waiting = {
  user_code: string;
  verification_uri: string;
  interval: number;
  expires_in: number;
};

/** What the panel is in the middle of. */
type Doing =
  | { at: "resting" }
  | { at: "google" }
  | { at: "github"; code?: Waiting }
  | { at: "failed"; why: string };

export function SignIn() {
  const [doing, setDoing] = useState<Doing>({ at: "resting" });

  // The code arrives after the flow has started, so it cannot be a return value.
  useEffect(() => {
    const stop = listen<Waiting>("github_code", (e) =>
      setDoing((was) => (was.at === "github" ? { at: "github", code: e.payload } : was)),
    );
    return () => {
      stop.then((off) => off());
    };
  }, []);

  // Nothing is done with the account here. The panel is listening for the same
  // event and swaps this whole page out, which is the only transition that
  // makes sense -- there is no "signed in" state of the sign-in page.
  const go = (provider: "google" | "github") => {
    setDoing({ at: provider });
    invoke(provider === "google" ? "sign_in_google" : "sign_in_github").catch((why) =>
      setDoing({ at: "failed", why: String(why) }),
    );
  };

  const busy = doing.at === "google" || doing.at === "github";

  return (
    <div className="flex min-h-0 flex-1 flex-col items-center px-8 pt-7 pb-5">
      <div className="pointer-events-none scale-[1.3]">
        <Companion mode="idle" anchored />
      </div>

      <h1 className="mt-6 text-[19px] leading-tight font-semibold tracking-[-0.01em] text-ink">
        {doing.at === "github" && doing.code ? "Type this at GitHub" : "Nudge"}
      </h1>
      <p className="mt-1.5 max-w-[19rem] text-center text-[12.5px] leading-[1.55] text-ink-2">
        {message(doing)}
      </p>

      <div className="mt-5 flex w-full max-w-[16rem] flex-col gap-2">
        {doing.at === "github" && doing.code ? (
          <Code waiting={doing.code} />
        ) : (
          <>
            <Provider
              onClick={() => go("google")}
              busy={doing.at === "google"}
              disabled={busy}
              icon={<GoogleMark />}
              label="Continue with Google"
            />
            <Provider
              onClick={() => go("github")}
              busy={doing.at === "github"}
              disabled={busy}
              icon={<GitHubMark />}
              label="Continue with GitHub"
            />
          </>
        )}
      </div>

      {/* Held at the foot rather than under the buttons, so that the page does
          not jump a row taller the moment something goes wrong. */}
      <div className="mt-auto pt-4 text-center">
        {doing.at === "failed" ? (
          <button
            onClick={() => setDoing({ at: "resting" })}
            className="text-[11.5px] text-accent transition-opacity duration-150 hover:opacity-80"
          >
            {doing.why} — try again
          </button>
        ) : (
          <p className="text-[10.5px] leading-relaxed text-ink-3">
            By continuing you agree to the{" "}
            <Link href="https://nudge.runmycrew.com/terms">Terms</Link> and{" "}
            <Link href="https://nudge.runmycrew.com/privacy">Privacy Policy</Link>.
          </p>
        )}
      </div>
    </div>
  );
}

/** One sentence, and never a promise about how long it will take. */
function message(doing: Doing): string {
  switch (doing.at) {
    case "google":
      return "Finish signing in in your browser.";
    case "github":
      return doing.code
        ? "Then come back here — this window will catch up on its own."
        : "Asking GitHub for a code.";
    case "failed":
      return "That did not go through.";
    default:
      return "Sign in to get started. Your Mac stays yours — Nudge only sees what you point it at.";
  }
}

/**
 * The device code, which somebody has to read and retype.
 *
 * Wide letter-spacing and a tabular face, because this is a string being copied
 * character by character and the two jobs of a code are to be unambiguous and to
 * be findable again after looking away.
 */
function Code({ waiting }: { waiting: Waiting }) {
  return (
    <>
      <div className="grid place-items-center rounded-card border border-line bg-raise py-3 font-mono text-[19px] tracking-[0.26em] text-ink tabular-nums">
        {waiting.user_code}
      </div>
      <a
        href={waiting.verification_uri}
        target="_blank"
        rel="noreferrer"
        className="grid h-9 place-items-center rounded-control bg-blue text-[12.5px] font-medium text-white transition-colors duration-150 hover:bg-blue-hi"
      >
        Open GitHub
      </a>
    </>
  );
}

/**
 * One way in.
 *
 * Both buttons are the same weight on purpose. Making one primary would be
 * guessing which account somebody has, and guessing wrong puts the quieter
 * button under the one they wanted.
 */
function Provider({
  onClick,
  busy,
  disabled,
  icon,
  label,
}: {
  onClick: () => void;
  busy: boolean;
  disabled: boolean;
  icon: React.ReactNode;
  label: string;
}) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      className={[
        "flex h-9 items-center gap-2.5 rounded-control border border-line px-3",
        "text-[12.5px] font-medium text-ink",
        "transition-[background-color,transform] duration-150 ease-[cubic-bezier(0.23,1,0.32,1)]",
        "active:scale-[0.985]",
        disabled ? "opacity-45" : "bg-raise hover:bg-raise-hi",
      ].join(" ")}
    >
      <span className="grid size-4 shrink-0 place-items-center">{icon}</span>
      <span className="min-w-0 flex-1 text-left">{busy ? "Waiting…" : label}</span>
    </button>
  );
}

function Link({ href, children }: { href: string; children: React.ReactNode }) {
  return (
    <a
      href={href}
      target="_blank"
      rel="noreferrer"
      className="text-ink-2 underline decoration-line underline-offset-2 transition-colors duration-150 hover:text-ink"
    >
      {children}
    </a>
  );
}

/* The marks are inline because the panel is inside the binary -- there is no
   asset directory to fetch from -- and because both are required to be shown in
   their own colours. */

function GoogleMark() {
  return (
    <svg viewBox="0 0 48 48" className="size-4" aria-hidden>
      <path
        fill="#4285F4"
        d="M45.12 24.5c0-1.56-.14-3.06-.4-4.5H24v8.51h11.84c-.51 2.75-2.06 5.08-4.39 6.64v5.52h7.11c4.16-3.83 6.56-9.47 6.56-16.17z"
      />
      <path
        fill="#34A853"
        d="M24 46c5.94 0 10.92-1.97 14.56-5.33l-7.11-5.52c-1.97 1.32-4.49 2.1-7.45 2.1-5.73 0-10.58-3.87-12.31-9.07H4.34v5.7C7.96 41.07 15.4 46 24 46z"
      />
      <path
        fill="#FBBC05"
        d="M11.69 28.18c-.44-1.32-.69-2.73-.69-4.18s.25-2.86.69-4.18v-5.7H4.34A21.99 21.99 0 0 0 2 24c0 3.55.85 6.91 2.34 9.88l7.35-5.7z"
      />
      <path
        fill="#EA4335"
        d="M24 10.75c3.23 0 6.13 1.11 8.41 3.29l6.31-6.31C34.91 4.18 29.93 2 24 2 15.4 2 7.96 6.93 4.34 14.12l7.35 5.7c1.73-5.2 6.58-9.07 12.31-9.07z"
      />
    </svg>
  );
}

function GitHubMark() {
  return (
    <svg viewBox="0 0 16 16" className="size-4 fill-ink" aria-hidden>
      <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27s1.36.09 2 .27c1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.01 8.01 0 0 0 16 8c0-4.42-3.58-8-8-8z" />
    </svg>
  );
}
