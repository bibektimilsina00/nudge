import { create } from "zustand";
import { persist } from "zustand/middleware";

export type Platform = "macos-arm64" | "macos-x64" | "windows-x64" | "linux-x64";

export const LABEL: Record<Platform, string> = {
  "macos-arm64": "macOS · Apple silicon",
  "macos-x64": "macOS · Intel",
  "windows-x64": "Windows",
  // An AppImage: one file, no install, and the only Linux package that does not
  // need a different answer per distribution.
  "linux-x64": "Linux · AppImage",
};

/** What the download button says, per platform. */
export const VERB: Record<Platform, string> = {
  "macos-arm64": "Download for Mac",
  "macos-x64": "Download for Mac",
  "windows-x64": "Download for Windows",
  "linux-x64": "Download for Linux",
};

/**
 * What you need to run it, in the line under the button.
 *
 * The Linux floor is glibc, and it is 2.39 because `xcap` pulls in the pipewire
 * bindings and those do not compile against anything older -- see the release
 * workflow, where the runner was moved for exactly this.
 */
export const NEEDS: Record<Platform, string> = {
  "macos-arm64": "macOS 12 or later",
  "macos-x64": "macOS 12 or later",
  "windows-x64": "Windows 10 or later",
  "linux-x64": "AppImage · glibc 2.39 or newer",
};

/** Which platforms actually have a build. The rest are honest about it. */
export const BUILT: Platform[] = ["macos-arm64", "linux-x64"];

/**
 * Guess what somebody is on, from the browser.
 *
 * Apple silicon versus Intel cannot be read from the user agent -- Safari
 * reports both as "Intel Mac OS X" -- so this is a guess that the picker exists
 * to correct. Every Mac sold since 2020 is Apple silicon, which makes it the
 * right guess and the wrong thing to be certain about.
 */
export function guess(): Platform {
  if (typeof navigator === "undefined") return "macos-arm64";
  const ua = navigator.userAgent;
  if (/Mac/.test(ua)) return "macos-arm64";
  if (/Win/.test(ua)) return "windows-x64";
  if (/Linux|X11/.test(ua)) return "linux-x64";
  return "macos-arm64";
}

type Store = {
  platform: Platform | null;
  /** True once somebody has chosen, so the guess stops overriding them. */
  chosen: boolean;
  choose: (p: Platform) => void;
  detect: () => void;
};

/**
 * Remembered, because somebody who corrected the guess and came back to the page
 * should not have to correct it again.
 */
export const usePlatform = create<Store>()(
  persist(
    (set, get) => ({
      platform: null,
      chosen: false,
      choose: (platform) => set({ platform, chosen: true }),
      detect: () => {
        if (get().chosen || get().platform) return;
        set({ platform: guess() });
      },
    }),
    { name: "nudge.platform" },
  ),
);
