# Nudge -- run `make` for the list.
#
# macOS note: every target that produces a runnable app signs it. TCC identifies an
# app by its code signature, so an ad-hoc signed bundle gets a fresh identity on each
# build and your Screen Recording grant silently stops applying -- the toggle still
# reads "on". Signing with a stable identity is what makes the grant persist.

APP     := src-tauri/target/release/bundle/macos/Nudge.app
GOAL    ?=
PROVIDER ?=
MODEL   ?=

# Which certificate signs the app. Pinned in a gitignored file rather than detected,
# because `security find-identity` does not order its output stably -- on a machine
# with several certificates, auto-detection picks a different one between runs, the
# signature changes, and macOS quietly voids your Screen Recording grant. Auto-detect
# only when there is exactly one candidate and therefore no choice to get wrong.
IDENTITY_FILE := .signing-identity
CERTS = $(shell security find-identity -v -p codesigning 2>/dev/null | grep -c 'Apple Development')
SIGN_ID ?= $(shell cat $(IDENTITY_FILE) 2>/dev/null \
             || { test '$(CERTS)' = 1 && security find-identity -v -p codesigning \
                  | grep 'Apple Development' | sed -E 's/.*"(.*)"/\1/'; })
export APPLE_SIGNING_IDENTITY = $(SIGN_ID)

# The updater artifact is signed with a minisign key, separate from the Apple one
# and for a different question: Apple's says who built it, this says the update
# you just downloaded came from whoever built the version you are running.
#
# `tauri.conf.json` carries the public half, so a build that cannot sign fails at
# the very last step -- after producing a perfectly good .app -- and takes `run`'s
# restart down with it. That is what it did for a whole session here, for want of
# two variables sitting in a gitignored file next to the one already being read.
#
# Empty password because the key was generated without one. Kept explicit: unset
# and set-to-empty are different to the signer, and the failure for unset reads as
# a missing key rather than a missing password.
UPDATER_KEY := .signing/updater.key
export TAURI_SIGNING_PRIVATE_KEY = $(shell cat $(UPDATER_KEY) 2>/dev/null)
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD =

.DEFAULT_GOAL := help
.PHONY: truth help dev build run test lint fmt probe bench record cases reset-perms clean sign-check tools picks release ship-check share site publish

help: ## Show this list
	@grep -hE '^[a-z-]+:.*?## ' $(MAKEFILE_LIST) \
	  | awk -F':.*?## ' '{printf "  \033[1m%-12s\033[0m %s\n", $$1, $$2}'
	@echo
	@echo "  make probe GOAL=\"open the UV editor\" [PROVIDER=gemini] [MODEL=...]"

dev: ## Hot-reloading dev build (borrows the launching terminal's permissions)
	cd src-tauri && cargo tauri dev

build: sign-check ## Build and sign the release .app
	cd src-tauri && cargo tauri build

run: build ## Build, then restart the app
	@pkill -f 'Nudge.app/Contents/MacOS/nudge' 2>/dev/null || true
	@# launchd rejects the relaunch with -600 if the old process is still exiting,
	@# so wait for it to actually go rather than guessing at a sleep.
	@while pgrep -f 'Nudge.app/Contents/MacOS/nudge' >/dev/null; do sleep 0.1; done
	@open $(APP) && echo "Nudge is in your menu bar."

test: ## Run the Rust test suite
	cd src-tauri && cargo test

picks: ## Score the control-picking cases (offline, instant, no API key)
	cd src-tauri && cargo run --quiet --bin picks

truth: ## Score the answers — does it tell the truth? (needs a key, makes real calls)
	cd src-tauri && cargo run --quiet --bin truth

lint: ## Clippy with warnings as errors, plus a format check
	cd src-tauri && cargo clippy --all-targets -- -D warnings && cargo fmt --check

fmt: ## Format Rust sources
	cd src-tauri && cargo fmt

probe: ## Ask one question and write hit-<provider>.png -- GOAL="..." required
	@test -n '$(GOAL)' || { echo 'usage: make probe GOAL="open the UV editor"'; exit 2; }
	cd src-tauri && $(if $(PROVIDER),NUDGE_PROVIDER=$(PROVIDER)) $(if $(MODEL),NUDGE_MODEL=$(MODEL)) \
	  cargo run --release --quiet --bin probe -- '$(GOAL)'

site: ## Run the marketing site and its API together
	@echo "  api  http://localhost:8080/docs"
	@echo "  web  http://localhost:3000"
	@trap 'kill 0' EXIT; \
	  (cd server && uv run uvicorn app.main:app --reload --port 8080) & \
	  (cd web && pnpm dev) & wait

publish: build ## Publish the current build so the site can serve it -- VERSION=0.1.0
	@test -n '$(VERSION)' || { echo 'usage: make publish VERSION=0.1.0'; exit 2; }
	cd server && uv run publish.py \
	  ../src-tauri/target/release/bundle/dmg/Nudge_$(VERSION)_aarch64.dmg \
	  --version '$(VERSION)' --platform macos-arm64 --notes '$(NOTES)'

share: ## Pack a build to send someone, with instructions (no Apple account needed)
	@./scripts/share.sh

release: ## Build, sign, notarise and staple a build other people can open
	@./scripts/release.sh

ship-check: ## Ask Gatekeeper what another Mac would say about the current build
	@echo "signature:"
	@codesign -dv --verbose=2 $(APP) 2>&1 | grep -E 'Authority|flags' | sed 's/^/  /' || echo "  unsigned"
	@echo "gatekeeper:"
	@spctl -a -vvv -t install $(APP) 2>&1 | sed 's/^/  /' || true
	@echo "stapled ticket:"
	@xcrun stapler validate $(APP) 2>&1 | sed 's/^/  /' || true

reset-perms: ## Forget Nudge's Screen Recording and Microphone grants
	@tccutil reset ScreenCapture dev.nudge.app
	@tccutil reset Microphone dev.nudge.app
	@echo "Granted again on next launch."

clean: ## Remove build output
	cd src-tauri && cargo clean
	rm -rf ui/dist ui/node_modules

tools: ## Install the build tooling this repo needs
	cargo install tauri-cli --version '^2' --locked
	cd ui && pnpm install

pin-identity: ## Choose the signing certificate -- ID="Apple Development: ..."
	@test -n '$(ID)' || { echo 'Pick one and re-run with ID="...":'; \
	  security find-identity -v -p codesigning | grep 'Apple Development' \
	    | sed -E 's/^ *[0-9]+\) [A-F0-9]+ /  /'; exit 2; }
	@printf '%s' '$(ID)' > $(IDENTITY_FILE)
	@echo "pinned: $(ID)"
	@echo "Signature changes void existing permission grants -- run 'make reset-perms'."

sign-check:
	@test -n '$(SIGN_ID)' || { \
	  echo 'No signing certificate chosen, and this machine has $(CERTS) to pick from.'; \
	  echo 'Building unsigned means macOS re-prompts for Screen Recording after every'; \
	  echo 'rebuild. Run: make pin-identity ID="..."'; exit 2; }
	@echo "signing as: $(SIGN_ID)"

# Accuracy: the number that decides the project. `make record GOAL="..."` adds a
# case by capturing the screen and letting you click the right answer; `make
# bench` re-scores every case. Change the model or the prompt, run it again, and
# the two numbers are comparable because the screenshots did not move.
bench: ## Score every saved case -- the accuracy number
	cd src-tauri && cargo run --release --bin bench

record: ## Add a case: capture the screen, click the answer -- GOAL="..."
	cd src-tauri && cargo run --release --bin bench -- record "$(GOAL)"

cases: ## List the saved cases
	cd src-tauri && cargo run --release --bin bench -- list
