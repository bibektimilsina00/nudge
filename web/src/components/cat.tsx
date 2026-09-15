"use client";

import { useEffect, useState, useSyncExternalStore } from "react";
import Image from "next/image";
import { Alignment, Fit, Layout, useRive } from "@rive-app/react-canvas";

/**
 * The cat, alive.
 *
 * The same `cat.riv` the app ships, running the same state machine — so what is
 * on the page is the thing you download rather than a picture of it. It idles
 * and blinks on its own, which is the whole reason to carry a canvas runtime for
 * it; a still would be a sticker and a PNG would do.
 *
 * The still sits underneath and stays there. It is rendered from the same
 * artboard at the same fit, so the canvas lands exactly on top of it and the two
 * are indistinguishable — which is what makes layering safe.
 *
 * It did not start that way. The still was a tight crop of the artwork while
 * Rive draws the whole artboard, so they never lined up and both were visible at
 * once: a large blurred cat behind a smaller sharp one. Swapping one for the
 * other on Rive's `onLoad` fixed the doubling and introduced a worse bug --
 * `onLoad` means the file parsed, not that anything was painted, and in a
 * context where the canvas never paints it removed the only cat on the page.
 * Aligning them removes the need to choose.
 */
export function Cat() {
  const [wanted, setWanted] = useState(false);
  const reduced = usePrefersReducedMotion();

  useEffect(() => {
    if (reduced) return;
    // After paint, not during. The hero should be readable before a megabyte of
    // canvas runtime is fetched for something decorative.
    const id = window.requestIdleCallback
      ? window.requestIdleCallback(() => setWanted(true), { timeout: 2500 })
      : window.setTimeout(() => setWanted(true), 900);
    return () => {
      if (window.cancelIdleCallback) window.cancelIdleCallback(id as number);
      else window.clearTimeout(id as number);
    };
  }, [reduced]);

  return (
    <div className="relative aspect-square w-full max-w-[26rem]">
      {/* Always present. Under reduced motion, on a slow connection, or if the
          runtime never arrives, this is simply what is on the page. */}
      <Image
        src="/cat.png"
        alt="Nudge, a small black cat"
        width={1024}
        height={1024}
        priority
        className="absolute inset-0 size-full object-contain"
      />
      {wanted && !reduced && <Live />}
    </div>
  );
}

function Live() {
  const { RiveComponent } = useRive({
    src: "/cat.riv",
    // The machine, not a timeline: that is what gives it an idle of its own
    // rather than a loop driven from outside.
    stateMachines: "State Machine 1",
    autoplay: true,
    // The same fit and alignment the still was rendered at, which is what lets
    // it sit on top without a seam.
    layout: new Layout({ fit: Fit.Contain, alignment: Alignment.Center }),
  });

  return <RiveComponent className="absolute inset-0 size-full" />;
}

/**
 * Whether the reader has asked for less movement.
 *
 * Subscribed to rather than read once in an effect: it is external state that
 * can change while the page is open, and reading it with `setState` in an effect
 * also costs a second render on every visit for a value known before the first.
 */
function usePrefersReducedMotion() {
  return useSyncExternalStore(
    (onChange) => {
      const query = window.matchMedia("(prefers-reduced-motion: reduce)");
      query.addEventListener("change", onChange);
      return () => query.removeEventListener("change", onChange);
    },
    () => window.matchMedia("(prefers-reduced-motion: reduce)").matches,
    () => false,
  );
}
