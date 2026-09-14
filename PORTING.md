# Porting Nudge

Everything that touches the operating system lives behind one seam, and most of
it now has a second implementation that works anywhere. This is what is done,
what is missing, and what it will cost.

Build the cross-platform path on any machine, including a Mac:

    cargo check  --features portable
    cargo test   --lib --features portable
    cargo run    --features portable --example portable

That last one runs it. **Code that is never compiled is code that does not
work**, and the whole reason this feature exists is that Windows and Linux cannot
be built from a Mac -- both cross-compiles die inside third-party build scripts,
`glib-sys` and `aws-lc-sys`, before reaching a line of ours.

## Done, and verified by running it

| | how | measured on macOS through the portable path |
|---|---|---|
| screenshot | `xcap` | works -- **2114ms**, against 61ms native |
| pointer | `enigo` | works -- position, move, click |
| key and button state | `device_query` | works -- held modifiers, mouse buttons |
| typing | `enigo::text` | compiles; types a *string*, so the layout is the platform's problem |
| shortcuts | `enigo` | parser tested; modifiers held as real keys, not flags |
| launching | `open` + Start menu / XDG entries | works; finds nothing on a Mac, correctly |
| frontmost window | `active-win-pos-rs` | works -- **this one is not optional** |
| recording | `cpal` | already cross-platform, unchanged |
| the loop, prompt, tools, agents, UI | -- | never platform-specific |

**The frontmost window is the one to do first.** `privacy::blocked_by` is handed
what it returns, so a platform where it answers nothing is a platform where the
guard never fires and every password manager gets photographed.

## Written but never run

**The Windows accessibility tree.** `uiautomation` is Windows-only, so it cannot
be compiled from a Mac, let alone exercised -- the `portable` feature does not
help here. Every line of `ax::imp` for Windows is a first draft.

What is *not* a first draft is everything around it. `Control`, the `usable`
rule that rejects a control with no name or no place, the numbering the model is
shown, and the matching that decides a goal is obvious -- all shared, all tested
on macOS. Windows supplies only the walk.

Expect to fix, in rough order of likelihood: the property names on `UIElement`,
whether `get_control_view_walker` is the right walker, and whether enumerating
top-level windows by process id actually finds them. The shape is right; the
calls may not be.

## Missing

| | what happens without it | what to use |
|---|---|---|
| accessibility tree, Linux | falls back to the vision model: slower and less accurate, but works | `atspi` |
| system voice | `speech_engine = "gemini"` works over the network | `tts` |
| is audio playing | reports silence; the model guesses from the picture | WASAPI / PulseAudio |

None of these stop it running. All three cost quality rather than function.

## What will be slower, and why it matters

A capture is **2114ms** here against **61ms** natively, because the native path
hands a `CGImage` straight from ScreenCaptureKit to a hardware JPEG encoder and
the pixels never pass through Rust. There is no portable equivalent.

That is not a footnote. On macOS a turn is 4.5 seconds and the screenshot is
hidden entirely behind transcription. Two seconds of capture cannot hide behind
anything, and it is paid again by the stillness check. **Expect a very different
latency profile and measure it there** -- every number in SPEED.md was measured
on macOS and none of them transfer.

## Two traps worth reading before starting

**Typing is not a table of key codes.** It is a lookup against the layout in use.
The macOS version goes through `UCKeyTranslate` because a hardcoded US table
typed "abc;" where someone meant "abcz", on a Dvorak keyboard. `enigo::text`
avoids this by typing strings; anything that maps characters to codes by hand
will hit it.

**The shortcut spec is written in Mac.** `cmd+shift+n` maps `cmd` to Meta, which
is Command on macOS and the Windows key elsewhere -- and a model thinking in
Windows terms would have written `ctrl`. Translate the spec, not the key table.

## What is left, honestly

The seam is drawn, held together by a test, and the implementations behind it run.
What has **not** happened is compiling or running any of it on Windows or Linux.
That has to be done on those machines, and the first thing to find out there is
whether it builds at all.
