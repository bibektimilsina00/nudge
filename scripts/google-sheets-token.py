#!/usr/bin/env python3
"""Sign in to Google for the Sheets connector, and write the token it wants.

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

import http.server
import json
import pathlib
import secrets
import socket
import threading
import urllib.parse
import urllib.request
import webbrowser

SCOPES = [
    "https://www.googleapis.com/auth/spreadsheets",
    "https://www.googleapis.com/auth/drive.file",
]
KEYS = pathlib.Path.home() / ".config/gcp-oauth.keys.json"
OUT = pathlib.Path.home() / ".mcp-google-sheets-token.json"


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
                body = b"<h1>Signed in.</h1><p>You can close this window.</p>"
            else:
                body = b"<h1>That did not carry a code.</h1>"
            self.send_response(200)
            self.send_header("Content-Type", "text/html")
            self.end_headers()
            self.wfile.write(body)
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
