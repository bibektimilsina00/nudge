"use client";

import { useEffect, useState, useSyncExternalStore } from "react";
import Image from "next/image";
import { Alignment, Fit, Layout, useRive } from "@rive-app/react-canvas";

/**
 * The cat, alive.
 *
 * The same `cat.riv` the app ships, running the same state machine — so what is
 * on the page is the thing you download rather than a picture of it. It idles
 * and blinks on its own, which is the entire reason to carry a canvas runtime
 * here; a still of it would be a sticker and a PNG would do.
 *
 * Mounted only on the client and only after the page is interactive. The runtime
 * is around 1.9MB, which is not a thing to put in front of somebody's first
 * paint — the static cat holds the space until it is ready, so the layout never
 * shifts and there is always a cat.
 */
export function Cat() {
  const [ready, setReady] = useState(false);
  const reduced = usePrefersReducedMotion();

  useEffect(() => {
    if (reduced) return;

    // After paint, not during. The hero should be readable before a megabyte of
    // canvas runtime is fetched for something decorative.
    const id = window.requestIdleCallback
      ? window.requestIdleCallback(() => setReady(true), { timeout: 2500 })
      : window.setTimeout(() => setReady(true), 900);
    return () => {
      if (window.cancelIdleCallback) window.cancelIdleCallback(id as number);
      else window.clearTimeout(id as number);
    };
  }, [reduced]);

  return (
    <div className="relative aspect-square w-full max-w-[26rem]">
      {/* Always present, and underneath. If the runtime never arrives -- blocked,
          slow, or reduced-motion -- this is what is on the page, at the same size
          and in the same place. */}
      <Image
        src="/cat.png"
        alt="Nudge, a small black cat"
        width={512}
        height={512}
        priority
        className="absolute inset-0 size-full object-contain"
      />
      {!reduced && ready && <Live />}
    </div>
  );
}

function Live() {
  const { RiveComponent } = useRive({
    src: "/cat.riv",
    // The machine, not a timeline: that is what gives it an idle of its own
    // rather than a loop being driven from outside.
    stateMachines: "State Machine 1",
    autoplay: true,
    layout: new Layout({ fit: Fit.Contain, alignment: Alignment.Center }),
  });

  // Sits exactly over the still, so the swap is invisible.
  return <RiveComponent className="absolute inset-0 size-full" />;
}

/**
 * Whether the reader has asked for less movement.
 *
 * Subscribed to rather than read once in an effect: it is external state that
 * can change while the page is open, and `useSyncExternalStore` is the thing
 * built for external state. Reading it with `setState` in an effect also costs a
 * second render on every visit, for a value that is known before the first.
 *
 * The server snapshot is `false`, so the markup matches a machine that has not
 * expressed a preference; the animation only ever mounts on the client anyway.
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
