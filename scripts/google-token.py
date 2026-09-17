#!/usr/bin/env python3
"""Sign in to Google for a connector, and write the token it wants.

Scopes and destination come from the environment so one flow serves several
connectors, each asking for only its own:

    SCOPES="https://www.googleapis.com/auth/youtube.readonly" \\
    OUT=~/.config/nudge/youtube-token.json python3 scripts/google-token.py

Defaults to the Sheets pair, which is what it was written for.

`mcp-google-sheets` reads a token file and, if there is not one, raises an error
telling you to authenticate in a browser -- which it has no code to do. There is
no flow in the package at all, only the check for the file. So this does the
part it describes and does not implement.

Two scopes and no more: `spreadsheets`, and `drive.file`, which is per-file
access to what the app itself created or you opened with it rather than your
Drive. That is the whole reason for having a Sheets entry separate from the
Workspace one, so it is worth not widening here.

    python3 scripts/google-sheets-token.py
"""

import base64
import html
import http.server
import os
import json
import pathlib
import secrets
import socket
import threading
import urllib.parse
import urllib.request
import webbrowser

SCOPES = [s.strip() for s in os.environ.get(
    "SCOPES",
    "https://www.googleapis.com/auth/spreadsheets "
    "https://www.googleapis.com/auth/drive.file",
).split()]
KEYS = pathlib.Path.home() / ".config/gcp-oauth.keys.json"
OUT = pathlib.Path(os.environ.get("OUT", pathlib.Path.home() / ".mcp-google-sheets-token.json"))
ICON = pathlib.Path(__file__).resolve().parent.parent / "src-tauri/icons/64x64.png"


def page(ok: bool, heading: str, detail: str, scopes: list[str]) -> bytes:
    """The one page anybody sees after handing Google access to something.

    Worth more than `<h1>Signed in.</h1>` on browser-default white, for a reason
    beyond looking nicer: at this exact moment somebody has just granted access
    to their account and the only thing that can tell them *who to* and *what to*
    is this page. So it carries Nudge's mark and lists the scopes back.

    Self-contained -- the icon is inlined and there are no fonts to fetch. It is
    served by a loopback server that stops a second later, so anything it asked
    the network for would be a race it loses.
    """
    try:
        mark = base64.b64encode(ICON.read_bytes()).decode()
        art = f'<img class="mark" src="data:image/png;base64,{mark}" alt="">'
    except OSError:
        # Running from somewhere the repo is not. Not a reason to fail.
        art = ""

    granted = "".join(
        f'<li><span class="tick">✓</span>{html.escape(s.rsplit("/", 1)[-1])}</li>'
        for s in scopes
    )
    # Failure colours the sentence that says what went wrong; success lets the
    # ticks carry it. Either way the accent appears once.
    detail_style = "" if ok else ' style="color:#ff6961"'
    return f"""<!doctype html>
<html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Nudge</title>
<style>
  :root {{ color-scheme: dark; }}
  * {{ box-sizing: border-box; }}
  body {{
    margin: 0; min-height: 100vh; display: grid; place-items: center;
    background: #0b0b0d; color: rgba(255,255,255,.95);
    font: 15px/1.55 -apple-system, BlinkMacSystemFont, "Segoe UI", system-ui, sans-serif;
    -webkit-font-smoothing: antialiased; padding: 24px;
  }}
  .card {{
    width: 100%; max-width: 27rem; background: #141417; border-radius: 16px;
    border: 1px solid rgba(255,255,255,.09); padding: 32px 28px; text-align: center;
  }}
  .mark {{ width: 52px; height: 52px; border-radius: 12px; }}
  h1 {{
    margin: 18px 0 6px; font-size: 1.3rem; font-weight: 600; letter-spacing: -.01em;
  }}
  p {{ margin: 0; color: rgba(255,255,255,.55); font-size: .875rem; }}
  ul {{
    margin: 22px 0 0; padding: 14px 16px; list-style: none; text-align: left;
    background: rgba(255,255,255,.04); border-radius: 10px;
    font: .8125rem/1.9 ui-monospace, SFMono-Regular, Menlo, monospace;
    color: rgba(255,255,255,.72);
  }}
  .tick {{ color: #30d158; margin-right: 9px; }}
  .what {{
    margin: 18px 0 0; font-size: .75rem; color: rgba(255,255,255,.38);
  }}
  @media (prefers-reduced-motion: no-preference) {{
    .card {{ animation: rise .32s cubic-bezier(.23,1,.32,1) both; }}
    @keyframes rise {{ from {{ opacity: 0; transform: translateY(6px); }} }}
  }}
</style></head>
<body><main class="card">
  {art}
  <h1>{html.escape(heading)}</h1>
  <p{detail_style}>{html.escape(detail)}</p>
  {f'<ul>{granted}</ul><p class="what">Granted to Nudge on this Mac. Nothing else.</p>' if granted else ""}
</main></body></html>"""


def free_port() -> int:
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


def main() -> int:
    if not KEYS.exists():
        print(f"No OAuth client at {KEYS}.")
        print("Download a Desktop client's JSON from Google Cloud Console first.")
        return 1

    creds = json.loads(KEYS.read_text())
    creds = creds.get("installed") or creds.get("web")
    port = free_port()
    redirect = f"http://localhost:{port}"
    # Guards against a stray request to the loopback being taken for the answer.
    state = secrets.token_urlsafe(16)
    got: dict[str, str] = {}
    done = threading.Event()

    class Handler(http.server.BaseHTTPRequestHandler):
        def do_GET(self):  # noqa: N802
            q = urllib.parse.parse_qs(urllib.parse.urlparse(self.path).query)
            if q.get("state", [None])[0] == state and "code" in q:
                got["code"] = q["code"][0]
                body = page(
                    True,
                    "Signed in",
                    "You can close this window.",
                    SCOPES,
                )
            else:
                # Either a stray request to the loopback, or consent was refused.
                # Both mean the same thing here and neither is worth a stack trace.
                body = page(
                    False,
                    "Not signed in",
                    q.get("error", ["That did not carry an authorisation code."])[0],
                    [],
                )
            self.send_response(200)
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.end_headers()
            self.wfile.write(body.encode())
            done.set()

        def log_message(self, *_):
            pass

    server = http.server.HTTPServer(("127.0.0.1", port), Handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()

    url = "https://accounts.google.com/o/oauth2/v2/auth?" + urllib.parse.urlencode({
        "client_id": creds["client_id"],
        "redirect_uri": redirect,
        "response_type": "code",
        "scope": " ".join(SCOPES),
        "access_type": "offline",
        "prompt": "consent",
        "state": state,
    })
    print(f"Opening a browser for consent. Scopes: {', '.join(s.rsplit('/', 1)[1] for s in SCOPES)}")
    print(f"If it does not open:\n{url}\n")
    webbrowser.open(url)

    if not done.wait(timeout=300):
        print("Timed out after five minutes.")
        return 1
    server.shutdown()
    if "code" not in got:
        print("No authorisation code came back.")
        return 1

    body = urllib.parse.urlencode({
        "code": got["code"],
        "client_id": creds["client_id"],
        "client_secret": creds["client_secret"],
        "redirect_uri": redirect,
        "grant_type": "authorization_code",
    }).encode()
    tok = json.load(urllib.request.urlopen("https://oauth2.googleapis.com/token", body))

    # The shape google-auth-library's setCredentials expects. `expires_in` is
    # seconds from now; the library wants an absolute `expiry_date` in millis,
    # and gets it wrong in a way that looks like a permissions error if omitted.
    import time
    tok["expiry_date"] = int(time.time() * 1000) + int(tok.get("expires_in", 3600)) * 1000
    OUT.write_text(json.dumps(tok, indent=2))
    OUT.chmod(0o600)
    print(f"Wrote {OUT}")
    print(f"Scopes granted: {tok.get('scope', '?')}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
