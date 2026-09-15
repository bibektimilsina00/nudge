"use client";

import { useEffect, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Apple, Check, Copy, Download, Loader2 } from "lucide-react";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { downloadUrl, latest, megabytes, type Release } from "@/lib/releases";
import { BUILT, LABEL, usePlatform, type Platform } from "@/lib/platform";

/**
 * The button the whole page exists for.
 *
 * Served from here rather than bounced to GitHub. That is a deliberate cost --
 * bandwidth, a server to keep up -- bought for two things: the numbers, and the
 * first impression. A download that hands somebody to a repository page asks
 * them to work out which of six files is theirs, before they know what the thing
 * does.
 */
export function DownloadButton({ compact = false }: { compact?: boolean }) {
  const { platform, chosen, choose, detect } = usePlatform();
  useEffect(detect, [detect]);

  const target = platform ?? "macos-arm64";
  const available = BUILT.includes(target);

  const { data, isPending, isError } = useQuery({
    queryKey: ["release", target],
    queryFn: () => latest(target),
    enabled: available,
  });

  return (
    <div className="flex flex-col items-center gap-3">
      {available ? (
        <Primary release={data} loading={isPending} failed={isError} />
      ) : (
        <NotYet platform={target} />
      )}

      {!compact && <PlatformPicker current={target} chosen={chosen} onChoose={choose} />}
    </div>
  );
}

function Primary({
  release,
  loading,
  failed,
}: {
  release?: Release;
  loading: boolean;
  failed: boolean;
}) {
  if (failed)
    return (
      <p className="text-[0.9375rem] text-ink-3">
        Nothing published yet. Check back shortly.
      </p>
    );

  return (
    <div className="flex flex-col items-center gap-2">
      <Button
        size="lg"
        asChild={!!release}
        disabled={!release}
        className="h-12 rounded-full bg-gold px-7 font-display text-[0.9375rem] font-semibold text-black hover:bg-gold/90"
      >
        {release ? (
          <a href={downloadUrl(release)}>
            <Apple className="size-[18px]" aria-hidden />
            Download for Mac
          </a>
        ) : (
          <span>
            <Loader2 className="size-[18px] animate-spin" />
            Loading…
          </span>
        )}
      </Button>

      {/* Reserved height, so the line arriving does not shove the page down. */}
      <p className="h-5 text-[0.8125rem] tabular-nums text-ink-3">
        {loading || !release
          ? " "
          : `Version ${release.version} · ${megabytes(release.size_bytes)} · macOS 12 or later`}
      </p>
    </div>
  );
}

/**
 * A platform with no build, said plainly and with somewhere to go.
 *
 * A disabled button on its own reads as broken. One that says why, and offers
 * the thing that does exist, reads as honest.
 */
function NotYet({ platform }: { platform: Platform }) {
  return (
    <div className="flex flex-col items-center gap-2">
      <Button
        size="lg"
        disabled
        className="h-12 rounded-full px-7 font-display text-[0.9375rem] font-semibold"
      >
        <Download className="size-[18px]" aria-hidden />
        {LABEL[platform]}
      </Button>
      <p className="max-w-[34ch] text-center text-[0.8125rem] text-pretty text-ink-3">
        Not built yet. macOS on Apple silicon is the only one so far — pick it
        below if that is what you are on.
      </p>
    </div>
  );
}

function PlatformPicker({
  current,
  chosen,
  onChoose,
}: {
  current: Platform;
  chosen: boolean;
  onChoose: (p: Platform) => void;
}) {
  return (
    <div className="flex flex-wrap items-center justify-center gap-1 text-xs">
      {(Object.keys(LABEL) as Platform[]).map((p) => (
        <button
          key={p}
          onClick={() => onChoose(p)}
          className={cn(
            "rounded-full px-2.5 py-1 transition-colors",
            p === current
              ? "bg-surface-2 text-ink ring-1 ring-line"
              : "text-ink-3 hover:text-ink-2",
          )}
        >
          {LABEL[p]}
          {!BUILT.includes(p) && <span className="ml-1 opacity-50">soon</span>}
        </button>
      ))}
      {!chosen && <span className="ml-1 text-ink-3/70">guessed from your browser</span>}
    </div>
  );
}

/**
 * The checksum, offered rather than pushed.
 *
 * Almost nobody checks it. The people who do are exactly the people deciding
 * whether to trust an unsigned build from someone they have not heard of, and
 * the cost of publishing it is one line.
 */
export function Checksum({ platform = "macos-arm64" }: { platform?: string }) {
  const { data } = useQuery({ queryKey: ["release", platform], queryFn: () => latest(platform) });
  const [copied, setCopied] = useState(false);

  if (!data) return null;

  return (
    <button
      onClick={() => {
        void navigator.clipboard.writeText(data.sha256);
        setCopied(true);
        window.setTimeout(() => setCopied(false), 1600);
      }}
      aria-label="Copy the SHA-256 checksum"
      className="group inline-flex max-w-full items-center gap-2 rounded-lg bg-surface px-3 py-2 font-mono text-[11px] text-ink-3 ring-1 ring-line transition-colors hover:text-ink-2"
    >
      <span className="shrink-0">SHA-256</span>
      <span className="truncate tabular-nums">{data.sha256}</span>
      <span className="sr-only" aria-live="polite">
        {copied ? "Checksum copied" : ""}
      </span>
      {copied ? (
        <Check className="size-3.5 shrink-0 text-gold" aria-hidden />
      ) : (
        <Copy className="size-3.5 shrink-0 opacity-50 group-hover:opacity-100" aria-hidden />
      )}
    </button>
  );
}
