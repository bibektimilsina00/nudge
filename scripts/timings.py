#!/usr/bin/env python3
"""What every turn has cost, added up.

    make timings

Reads `~/.config/nudge/timings.jsonl`, one object per turn, and prints the
median and the worst case for each stage in the order the pipeline runs them.

The median rather than the mean: one turn that waited forty seconds on a cold
`npx` pulls a mean somewhere no turn has ever been, and the question this
answers is "what does a turn normally cost".
"""

import json
import pathlib
import statistics
import sys

# What each stage is, because the file says `wav` and means something.
MEANS = {
    "wav": "draining the recording",
    "heard": "speech to text",
    "heard-here": "speech to text, on this machine",
    "heard-cloud": "speech to text, over the network",
    "fold": "summarising the history",
    "settle": "waiting for the screen to stop moving",
    "still": "waiting for it to stop moving again",
    "hush": "waiting for our own voice to stop",
    "shot": "screenshot and the accessibility tree",
    "brain": "the model",
}


def main() -> int:
    path = pathlib.Path.home() / ".config/nudge/timings.jsonl"
    if not path.is_file():
        print(f"nothing recorded yet: {path}")
        return 0

    turns = []
    for line in path.read_text().splitlines():
        try:
            turns.append(json.loads(line))
        except json.JSONDecodeError:
            # A half-written line from a process that was killed costs that line.
            continue
    if not turns:
        print("nothing recorded yet")
        return 0

    # In pipeline order, taken from the turns themselves rather than a list here
    # that would go stale the first time a stage is added.
    order: list[str] = []
    for turn in turns:
        for name, _ in turn["stages"]:
            if name not in order:
                order.append(name)

    spent: dict[str, list[float]] = {name: [] for name in order}
    for turn in turns:
        for name, secs in turn["stages"]:
            spent[name].append(secs)

    totals = [t["total"] for t in turns]
    drew = sum(1 for t in turns if t.get("drew"))
    print(f"{len(turns)} turns, {drew} of them drawn on\n")
    print(f"{'stage':<8} {'median':>8} {'worst':>8} {'share':>7}  what it is")
    middle = statistics.median(totals)
    for name in order:
        times = spent[name]
        med = statistics.median(times)
        print(
            f"{name:<8} {med:>7.2f}s {max(times):>7.2f}s "
            f"{med / middle * 100:>6.0f}%  {MEANS.get(name, '')}"
        )
    print(f"\n{'total':<8} {middle:>7.2f}s {max(totals):>7.2f}s")
    return 0


if __name__ == "__main__":
    sys.exit(main())
