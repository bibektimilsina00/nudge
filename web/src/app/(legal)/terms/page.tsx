import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Terms",
  description:
    "The terms for using Nudge: what it is, what it costs, and who is responsible for what it does on your Mac.",
};

/**
 * Terms of service.
 *
 * Short because the product is small: there is no account to suspend, no
 * subscription to cancel and no data of yours we hold. What is left is the one
 * clause that genuinely matters -- Nudge drives the real cursor, so the person
 * who starts it owns what it does.
 */
export default function Terms() {
  return (
    <article className="[&_h2]:mt-12 [&_h2]:mb-3 [&_h2]:font-display [&_h2]:text-[1.25rem] [&_h2]:font-semibold [&_h2]:text-ink [&_p]:mb-4 [&_p]:text-[0.9375rem] [&_p]:leading-7 [&_p]:text-ink-2 [&_li]:mb-2 [&_li]:text-[0.9375rem] [&_li]:leading-7 [&_li]:text-ink-2 [&_ul]:mb-4 [&_ul]:list-disc [&_ul]:pl-5 [&_code]:font-mono [&_code]:text-[0.875em] [&_code]:text-ink [&_a]:text-accent [&_a]:underline [&_a]:underline-offset-2">
      <h1 className="font-display text-[2rem] leading-tight font-semibold text-ink">Terms</h1>
      <p className="mt-3 text-[0.8125rem] text-ink-3 tabular-nums">Last updated 17 September 2026</p>

      <h2>What Nudge is</h2>
      <p>
        Nudge is a macOS application published by Nuddg Inc. It reads your screen and controls your
        mouse and keyboard to carry out tasks you ask it for. It is free, and there is no account.
      </p>
      <p>
        You supply the model: Nudge runs on an API key you hold with a provider you choose, or on a
        model running locally. We do not resell model access and we do not bill you for anything.
      </p>

      <h2>It drives your Mac, and you are responsible</h2>
      <p>
        This is the clause worth reading twice. When Nudge runs it is using your real cursor and
        keyboard, with your permissions, inside your logged-in session. Anything it can do, you
        could have done, and anything it does is attributable to you.
      </p>
      <p>So:</p>
      <ul>
        <li>
          Watch what it does the first few times, and keep the stop control within reach. Escape
          stops a running agent.
        </li>
        <li>
          Do not point it at anything where a wrong click is expensive and unrecoverable — money
          moving, production systems, anything irreversible — unless you are watching it.
        </li>
        <li>
          Language models get things wrong. Nudge asks before consequential steps and refuses some
          outright, but the last line of defence is you.
        </li>
        <li>
          When you connect an account, Nudge acts as you in that service, bound by that
          service&apos;s own terms. Breaking them with Nudge is still breaking them.
        </li>
      </ul>

      <h2>What you may not use it for</h2>
      <p>
        Anything unlawful; defeating another service&apos;s access controls, rate limits or
        anti-automation measures; impersonating someone; or operating an account you have no right
        to. Automating your own accounts is the point of the product. Automating other
        people&apos;s is not.
      </p>

      <h2>No warranty</h2>
      <p>
        Nudge is provided <strong className="text-ink">as is</strong>, without warranty of any kind,
        express or implied, including merchantability, fitness for a particular purpose and
        non-infringement. It is a young program that automates a computer; it will have bugs, and
        some of them will do the wrong thing on your screen.
      </p>

      <h2>Limitation of liability</h2>
      <p>
        To the fullest extent the law allows, Nuddg Inc is not liable for any indirect, incidental,
        special or consequential damages, or for lost data, lost profits or work lost to something
        Nudge did. Since Nudge is free, our total liability is limited to what you paid for it,
        which is nothing.
      </p>
      <p>
        Some jurisdictions do not allow these exclusions, in which case they apply to you only as
        far as that jurisdiction permits.
      </p>

      <h2>Model providers and connected services</h2>
      <p>
        Anthropic, Google, Ollama, GitHub, Slack and anything else you connect are independent of
        us. Your relationship with each is your own, under their terms and their pricing. We are not
        responsible for their availability, their output or their charges.
      </p>

      <h2>Source and licence</h2>
      <p>
        Nudge&apos;s source is published at{" "}
        <a href="https://github.com/bibektimilsina00/nudge">github.com/bibektimilsina00/nudge</a>.
        Your use of the source is governed by the licence in that repository, which these terms do
        not override.
      </p>

      <h2>Changes</h2>
      <p>
        These terms may change; the date at the top moves when they do. Continuing to use Nudge
        after a change means accepting it. If you do not, stop using it and delete the app — there
        is no account to close.
      </p>

      <h2>Contact</h2>
      <p>
        <a href="mailto:bibektimilsina000@gmail.com">bibektimilsina000@gmail.com</a>
      </p>
    </article>
  );
}
