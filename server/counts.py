#!/usr/bin/env python
"""What the app has been doing, read straight off the box.

A script rather than an endpoint. An authenticated admin route would be a new
piece of surface to get wrong, protecting numbers that are already sitting in a
file on a machine only I can log into -- and the first thing that happens to an
endpoint is somebody finding it. `publish.py` lives here for the same reason.

    uv run counts.py            # the last week
    uv run counts.py --days 30

Nothing here can say what anybody did: the table holds no goal, no transcript,
no window title and no application name. See `app/routes/events.py`.
"""

import argparse
import statistics
from collections import Counter
from datetime import timedelta

from sqlmodel import Session, select

from app.db import create_tables, engine
from app.models import Event, aware, now


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--days", type=int, default=7)
    days = ap.parse_args().days
    since = now() - timedelta(days=days)

    # The table is made when the server starts, and this can be run against a
    # database no server has opened yet -- a fresh checkout, or before the first
    # deploy. Making it here means "nothing yet" rather than a stack trace.
    create_tables()
    with Session(engine) as db:
        rows = [e for e in db.exec(select(Event)) if aware(e.at) >= since]

    if not rows:
        print(f"Nothing in the last {days} days.")
        return 0

    machines = {e.install for e in rows}
    signed_in = {e.user_id for e in rows if e.user_id}
    print(f"Last {days} days: {len(machines)} machines, {len(signed_in)} signed in\n")

    by_name = Counter(e.name for e in rows)
    for name, count in by_name.most_common():
        seconds = [e.seconds for e in rows if e.name == name and e.seconds]
        # The median, not the mean: one forgotten agent left running for an hour
        # moves a mean enough to hide a week of fast turns behind it.
        pace = f"  median {statistics.median(seconds):.1f}s" if seconds else ""
        print(f"{name:10} {count:>6}{pace}")
        # Which ones, in a line -- outcomes for agents, transcribers for turns.
        shapes = Counter(e.detail for e in rows if e.name == name and e.detail)
        if shapes:
            print("           " + "  ".join(f"{d} {n}" for d, n in shapes.most_common(6)))

    versions = Counter(f"{e.platform} {e.version}" for e in rows if e.version)
    print("\n" + "  ".join(f"{v} {n}" for v, n in versions.most_common(8)))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
