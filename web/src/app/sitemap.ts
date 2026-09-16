import type { MetadataRoute } from "next";

const url = "https://nudge.runmycrew.com";

export default function sitemap(): MetadataRoute.Sitemap {
  return [
    { url, lastModified: new Date(), changeFrequency: "weekly", priority: 1 },
    // Listed so Google Search Console can see them: an OAuth consent screen
    // pointing at a privacy policy is checked against the verified domain, and
    // a page Google cannot find is a page it treats as missing.
    { url: `${url}/privacy`, lastModified: new Date(), changeFrequency: "yearly", priority: 0.3 },
    { url: `${url}/terms`, lastModified: new Date(), changeFrequency: "yearly", priority: 0.3 },
  ];
}
