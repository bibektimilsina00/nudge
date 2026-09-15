import Image from "next/image";

/**
 * The page's top edge, pretending to be a screen's.
 *
 * This is the design decision the rest of the page is quiet for. Nudge lives in
 * the notch -- a black rectangle cut out of a MacBook's display, which is a
 * genuinely odd place to put software -- so the page opens by being a screen,
 * with the thing hanging off it and the cat sitting inside, before any copy
 * tries to explain it.
 *
 * Drawn first as true black on a true black page, which made it invisible: a
 * black cutout, a black cat, and nothing to see. A real notch reads because the
 * menu bar around it is lighter, so that is what carries it here -- a thin
 * translucent strip with the notch punched through. The page below stays true
 * black, which is what makes the strip look like a menu bar rather than a header.
 */
export function Notch() {
  return (
    <div className="sticky top-0 z-(--z-notch)">
      <div className="relative h-9 bg-white/[0.045] backdrop-blur-xl">
        {/* The hairline a menu bar has along its bottom. */}
        <span aria-hidden className="absolute inset-x-0 bottom-0 h-px bg-line" />

        <div className="settle absolute left-1/2 flex h-full -translate-x-1/2 items-end">
          <div className="relative flex h-full w-[190px] items-end justify-center rounded-b-[13px] bg-void">
            {/* Overflowing the notch on purpose: sitting entirely inside, the cat
                is a black shape in a black hole. Half out, its ears break the
                edge and the white of its eyes does the rest. */}
            <Image
              src="/cat.png"
              alt="Nudge, a small black cat, sitting in the notch"
              width={128}
              height={128}
              priority
              className="mb-[-14px] size-[52px] object-contain drop-shadow-[0_2px_8px_rgba(0,0,0,0.9)]"
            />
          </div>
        </div>
      </div>
    </div>
  );
}
