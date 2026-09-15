"use client";

import { useEffect } from "react";
import { useQuery } from "@tanstack/react-query";
import { Apple, Check, Copy, Download, Loader2 } from "lucide-react";
import { useState } from "react";

import { Button } from "@/components/ui/button";
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
export function DownloadButton() {
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

      <PlatformPicker current={target} chosen={chosen} onChoose={choose} />
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
      <p className="text-sm text-muted-foreground">
        Nothing published yet. Check back shortly.
      </p>
    );

  return (
    <div className="flex flex-col items-center gap-2">
      <Button size="lg" className="h-12 px-7 text-base" asChild={!!release} disabled={!release}>
        {release ? (
          <a href={downloadUrl(release)}>
            <Apple className="size-5" />
            Download for Mac
          </a>
        ) : (
          <span>
            <Loader2 className="size-5 animate-spin" />
            Loading
          </span>
        )}
      </Button>

      <p className="h-5 text-xs text-muted-foreground">
        {loading || !release
          ? " "
          : `Version ${release.version} · ${megabytes(release.size_bytes)} · macOS 12 or later`}
      </p>
    </div>
  );
}

function NotYet({ platform }: { platform: Platform }) {
  return (
    <div className="flex flex-col items-center gap-2">
      <Button size="lg" className="h-12 px-7 text-base" disabled>
        <Download className="size-5" />
        {LABEL[platform]}
      </Button>
      {/* Said plainly rather than hidden. A disabled button with no explanation
          reads as broken; one that says "not yet" reads as honest. */}
      <p className="text-xs text-muted-foreground">
        Not built yet — macOS on Apple silicon is the only one so far.
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
          className={`rounded-full px-2.5 py-1 transition-colors ${
            p === current
              ? "bg-foreground/10 text-foreground"
              : "text-muted-foreground hover:text-foreground"
          }`}
        >
          {LABEL[p]}
          {!BUILT.includes(p) && <span className="ml-1 opacity-50">soon</span>}
        </button>
      ))}
      {!chosen && (
        <span className="ml-1 text-muted-foreground/70">· guessed from your browser</span>
      )}
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
      className="group inline-flex max-w-full items-center gap-2 rounded-md border px-2.5 py-1.5 font-mono text-[11px] text-muted-foreground transition-colors hover:text-foreground"
    >
      <span className="shrink-0 not-italic">SHA-256</span>
      <span className="truncate">{data.sha256}</span>
      {copied ? (
        <Check className="size-3.5 shrink-0 text-green-500" />
      ) : (
        <Copy className="size-3.5 shrink-0 opacity-50 group-hover:opacity-100" />
      )}
    </button>
  );
}
