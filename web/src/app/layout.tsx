import type { Metadata, Viewport } from "next";
import { Inter, JetBrains_Mono, Space_Grotesk } from "next/font/google";

import { Providers } from "@/components/providers";
import "./globals.css";

// Space Grotesk carries the personality -- flat terminals, an odd single-storey
// look in places -- which suits a small precise Mac utility better than a
// neutral grotesque. Inter does the reading. JetBrains Mono exists because there
// is a real shell command on this page and a checksum, and both are things
// people copy.
const display = Space_Grotesk({
  variable: "--font-space-grotesk",
  subsets: ["latin"],
  weight: ["500", "600", "700"],
});
const sans = Inter({ variable: "--font-inter", subsets: ["latin"] });
const mono = JetBrains_Mono({ variable: "--font-jetbrains", subsets: ["latin"], weight: ["400"] });

const url = "https://nudge.runmycrew.com";

export const metadata: Metadata = {
  metadataBase: new URL(url),
  title: {
    default: "Nudge — an assistant that can see your screen",
    template: "%s · Nudge",
  },
  description:
    "A companion that lives in your Mac's notch. Hold a key, say what you want, and it points, clicks and types for you. Free, no account, runs on your own API key.",
  applicationName: "Nudge",
  keywords: [
    "mac assistant",
    "screen reading AI",
    "computer use",
    "macOS automation",
    "AI agent mac",
  ],
  authors: [{ name: "Nuddg Inc" }],
  alternates: { canonical: url },
  openGraph: {
    type: "website",
    url,
    siteName: "Nudge",
    title: "Nudge — an assistant that can see your screen",
    description:
      "Hold a key. Say what you want. Watch it happen. Free, no account, runs on your own API key.",
  },
  twitter: {
    card: "summary_large_image",
    title: "Nudge — an assistant that can see your screen",
    description: "Hold a key. Say what you want. Watch it happen.",
  },
  robots: { index: true, follow: true },
};

export const viewport: Viewport = {
  themeColor: "#000000",
  colorScheme: "dark",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" className="dark">
      <body className={`${display.variable} ${sans.variable} ${mono.variable}`}>
        <Providers>{children}</Providers>
      </body>
    </html>
  );
}
