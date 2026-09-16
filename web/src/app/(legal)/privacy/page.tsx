import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Privacy",
  description:
    "What Nudge sends, where it goes, and what our servers see. Nudge has no account, and your screen never passes through us.",
};

/**
 * The privacy policy.
 *
 * Written against the code rather than from a template. Every claim here was
 * checked in the source before it was written down: the download table in
 * `server/app/models.py`, the refusal list in `core/screen/privacy.rs`, the
 * Keychain indirection in `core/connect.rs`. If one of those changes, this page
 * is wrong and has to change with it.
 *
 * The structure follows the only question anybody actually has -- "where do the
 * pictures of my screen go" -- rather than the order a legal template would use.
 */
export default function Privacy() {
  return (
    <article className="[&_h2]:mt-12 [&_h2]:mb-3 [&_h2]:font-display [&_h2]:text-[1.25rem] [&_h2]:font-semibold [&_h2]:text-ink [&_p]:mb-4 [&_p]:text-[0.9375rem] [&_p]:leading-7 [&_p]:text-ink-2 [&_li]:mb-2 [&_li]:text-[0.9375rem] [&_li]:leading-7 [&_li]:text-ink-2 [&_ul]:mb-4 [&_ul]:list-disc [&_ul]:pl-5 [&_code]:font-mono [&_code]:text-[0.875em] [&_code]:text-ink [&_a]:text-accent [&_a]:underline [&_a]:underline-offset-2">
      <h1 className="font-display text-[2rem] leading-tight font-semibold text-ink">Privacy</h1>
      <p className="mt-3 text-[0.8125rem] text-ink-3 tabular-nums">Last updated 17 September 2026</p>

      <h2>The short version</h2>
      <p>
        Nudge has no account and no sign-up. It runs on your Mac, on a model provider you choose,
        billed to an API key that belongs to you. Pictures of your screen go from your Mac straight
        to that provider. <strong className="text-ink">They never pass through our servers</strong>,
        because we do not run any that could receive them.
      </p>
      <p>
        The only thing our servers see is a download. That is the whole of it; the rest of this page
        is detail.
      </p>

      <h2>What Nudge sends, and to whom</h2>
      <p>
        To do anything useful Nudge takes a screenshot and sends it somewhere that can read it. Which
        somewhere is your choice, made when you set it up:
      </p>
      <ul>
        <li>
          <strong className="text-ink">Anthropic or Google</strong> — the screenshot and your
          instruction go to that provider under your own API key, subject to their privacy terms and
          billed to your account. We are not party to it and cannot see it.
        </li>
        <li>
          <strong className="text-ink">Ollama</strong> — the model runs on your Mac and nothing
          leaves it at all.
        </li>
      </ul>
      <p>
        Nudge sends what the task needs: a screenshot, the text of your request, and a short record
        of the steps already taken. It does not send your files, your browsing history, or anything
        from a window it is not looking at.
      </p>

      <h2>What it refuses to look at</h2>
      <p>
        Some things should never reach a model provider, so Nudge checks{" "}
        <strong className="text-ink">before</strong> it captures rather than after — there is no
        taking the bytes back once they exist. It will not screenshot while a password manager is in
        front (1Password, Bitwarden, Keychain Access, LastPass, Dashlane, Proton Pass, Enpass,
        KeePassXC, NordPass, Strongbox, Authy), and it stops on window titles that name a secret —{" "}
        <code>.env</code>, <code>id_rsa</code>, <code>.pem</code>, and terms like{" "}
        <code>private key</code>, <code>seed phrase</code> or <code>api key</code>.
      </p>
      <p>
        This is a guard, not a guarantee. It reads the frontmost app and the window title, so a
        secret in a window that does not announce itself will not be caught. Treat it as a seatbelt.
      </p>

      <h2>Connected accounts</h2>
      <p>
        If you connect Google Workspace, GitHub, Slack or another service, Nudge acts as that
        account on your instruction. The connection is direct: your Mac to that service. Your
        documents, mail and calendar entries do not route through us and are not copied anywhere by
        Nudge.
      </p>
      <p>
        Access tokens are stored in your macOS <strong className="text-ink">Keychain</strong>.
        Nudge&apos;s own configuration file only records the <em>name</em> of the Keychain item, never
        the secret itself, so a copied config file carries no credentials. Revoke access at any time
        in that service&apos;s own settings — for Google, at{" "}
        <a href="https://myaccount.google.com/permissions">myaccount.google.com/permissions</a> — and
        delete the local copy by removing the connection in Nudge.
      </p>

      <h2>What this website records</h2>
      <p>
        When you download Nudge we write one row: which release, the time, your browser&apos;s
        user-agent string, and the page that referred you. That is the complete list.
      </p>
      <p>
        There is <strong className="text-ink">no IP address, no cookie, no analytics script and no
        third-party tracker</strong> on this site. The user-agent is kept because it separates a
        person from a link checker, and it will be dropped when there is a better way to tell them
        apart. None of it identifies you, and there is no account for it to be attached to.
      </p>

      <h2>Updates</h2>
      <p>
        Nudge checks this site for a newer version. That request tells us a version was checked; it
        carries no identifier and is not recorded against you.
      </p>

      <h2>Children</h2>
      <p>Nudge is not directed at children under 13 and we do not knowingly collect their data.</p>

      <h2>Changes to this page</h2>
      <p>
        If what Nudge sends ever changes, this page changes first and the date at the top moves. The
        page is in the public repository, so the history of what it said is public too.
      </p>

      <h2>Contact</h2>
      <p>
        Questions, or a request to delete something: <a href="mailto:bibektimilsina000@gmail.com">bibektimilsina000@gmail.com</a>.
      </p>
    </article>
  );
}
