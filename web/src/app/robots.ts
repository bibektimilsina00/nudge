import type { MetadataRoute } from "next";

/**
 * Indexable, deliberately.
 *
 * The landing-page rule is that an ad-only or time-bound page should not be
 * indexed. This is neither: it is the only place Nudge exists, the offer is
 * evergreen, and somebody searching for a way to talk to their Mac should find
 * it. The download endpoint is excluded because a crawler pulling a 19MB build
 * is bandwidth spent on nobody.
 */
export default function robots(): MetadataRoute.Robots {
  return {
    rules: { userAgent: "*", allow: "/", disallow: "/api/download/" },
    sitemap: "https://nudge.runmycrew.com/sitemap.xml",
  };
}
