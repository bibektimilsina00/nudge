import { z } from "zod";

/**
 * What the server promises, checked rather than assumed.
 *
 * The site and the API are separate deployments that will drift -- a field
 * renamed on one side is a silent `undefined` on the other, and the place that
 * surfaces is a download button showing "NaN MB" to somebody deciding whether to
 * trust this. Parsing at the boundary turns that into an error where it happened.
 */
export const Release = z.object({
  version: z.string(),
  platform: z.string(),
  size_bytes: z.number().int().positive(),
  sha256: z.string().length(64),
  notes: z.string(),
  published_at: z.string(),
  download_url: z.string(),
});

export type Release = z.infer<typeof Release>;

/** Where the API lives. Same origin in production, a separate port in dev. */
export const API = process.env.NEXT_PUBLIC_API ?? "http://localhost:8080";

export async function latest(platform: string): Promise<Release> {
  const res = await fetch(`${API}/api/releases/${platform}`);
  if (!res.ok) throw new Error(`no build published for ${platform}`);
  return Release.parse(await res.json());
}

export function downloadUrl(release: Release) {
  return `${API}${release.download_url}`;
}

/**
 * Megabytes, to one decimal. Nobody wants 19267889.
 *
 * Joined with a non-breaking space so the number and its unit cannot end up on
 * different lines, which is the one place a download size looks broken.
 */
export function megabytes(bytes: number) {
  return `${(bytes / 1_000_000).toFixed(1)}\u00a0MB`;
}
