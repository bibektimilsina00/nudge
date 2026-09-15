# Icons

The cat, taken out of `ui/src/assets/cat.riv` rather than redrawn.

`.riv` is a binary vector format with no command-line renderer here, so the art
was got at by letting the Rive runtime draw it in the app and reading the canvas
back with `toDataURL` — which keeps the alpha channel. A screenshot could not:
composited against the desktop, the transparency is already gone.

`source-1024.png` is what came out, composed onto the background. Regenerate the
rest from it:

```sh
cd src-tauri && cargo tauri icon icons/source-1024.png
rm -rf icons/android icons/ios icons/Square*.png icons/StoreLogo.png icons/icon.ico
```

That tool also writes Android, iOS and Windows sets. This is a macOS app; they
are deleted because otherwise they sit in every diff for the rest of the
project's life.

## Why it fills the whole square

macOS 26 masks app icons into its own shape. Drawn the old way — art inside a
squircle inset from the canvas — the system put *its* squircle around ours, and
the result was a rounded shape inside a rounded shape on a grey plate the system
supplied. Verified by reading the icon back out of the built bundle with
`NSWorkspace.icon(forFile:)`, which is what Finder and the Dock show.

So the background reaches every edge and the cat stays inside about 74% of the
canvas, because the corners are cropped away by whatever mask is applied. On
macOS 12–15, which does not mask, it is a square tile rather than a squircle —
the right way round, since a plain square reads as a choice and a squircle inside
a squircle reads as a mistake.

## Why it is light

The cat is black. The contrast between the two is the only reason the shape is
legible at 16px in a menu bar or a Finder list, which is where most icons are
actually seen.
