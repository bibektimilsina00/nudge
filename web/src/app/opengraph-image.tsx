import { readFileSync } from "node:fs";
import { join } from "node:path";
import { ImageResponse } from "next/og";

export const alt = "Nudge — say it out loud, watch your Mac do it";
export const size = { width: 1200, height: 630 };
export const contentType = "image/png";

/**
 * The social card, generated rather than exported from a design tool.
 *
 * Which means it cannot drift from the page: the headline here is the headline
 * there, and changing one is changing both. Read off disk at build time rather
 * than fetched, so the build needs no network and no running server.
 */
export default function Image() {
  const cat = readFileSync(join(process.cwd(), "public", "cat.png")).toString("base64");

  return new ImageResponse(
    (
      <div
        style={{
          width: "100%",
          height: "100%",
          display: "flex",
          alignItems: "center",
          gap: 64,
          background: "#000",
          padding: "0 96px",
        }}
      >
        <img src={`data:image/png;base64,${cat}`} alt="" width={260} height={293} />

        <div style={{ display: "flex", flexDirection: "column" }}>
          <div style={{ fontSize: 68, color: "#fff", lineHeight: 1.1, letterSpacing: -2 }}>
            Say it out loud.
          </div>
          <div style={{ fontSize: 68, color: "#fff", lineHeight: 1.1, letterSpacing: -2 }}>
            Watch your Mac do it.
          </div>
          <div style={{ marginTop: 28, fontSize: 28, color: "#a1a1a6" }}>
            An assistant that can see your screen
          </div>
          <div style={{ marginTop: 40, fontSize: 24, color: "#e9b44c" }}>
            nudge.runmycrew.com
          </div>
        </div>
      </div>
    ),
    size,
  );
}
