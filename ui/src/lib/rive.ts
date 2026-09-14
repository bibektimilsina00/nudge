// Rive pulls its WASM from a CDN by default, which the app's CSP blocks and
// which would make every animation vanish offline. Bundled instead.
//
// Imported for the side effect, from `main.tsx`, so that it happens once per
// window. It used to live in the companion's module, which meant it happened in
// the window that shows the companion and nowhere else -- so the agent's card,
// in a different window, quietly rendered nothing at all. No error: Rive asks
// the CDN, the CSP refuses, and the canvas stays empty.
import { RuntimeLoader } from "@rive-app/react-canvas";
import riveWasm from "@rive-app/canvas/rive.wasm?url";

RuntimeLoader.setWasmUrl(riveWasm);
