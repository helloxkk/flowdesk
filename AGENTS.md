# AGENTS.md

Project guidance for AI coding agents working on FlowDesk. If you are Claude Code,
also see [`CLAUDE.md`](CLAUDE.md) for the same content in Claude-specific framing —
the two files intentionally overlap; this one is the canonical, tool-neutral source.

## Project identity

**FlowDesk** is a software KVM utility: it lets one keyboard and mouse control
multiple computers over the network by moving the cursor across screen edges.

- It is a fork of [Barrier](https://github.com/debauchee/barrier), which forked
  from Symless's Synergy 1.9, which reimplemented Chris Schoeneman's
  CosmoSynergy. See the Acknowledgements section of [`README.md`](README.md).
- Owner/maintainer of this fork: **helloxkk**.
- Upstream Barrier is effectively dormant; this fork is the actively developed line.
- Architecture: **C++ core** (`barriers` binary, unchanged) + **Tauri GUI**
  (Rust backend + React frontend) that spawns barriers as a subprocess.

## License — GPLv2 (non-negotiable)

This project is licensed under the **GNU General Public License v2** with the
OpenSSL exemption noted at the top of [`LICENSE`](LICENSE). Anyone touching this
repo must respect:

1. **Preserve all upstream copyright notices** — in `LICENSE` and in every source
   file's header. The existing chain (Debauchee / Symless / Nick Bolton / Chris
   Schoeneman / FlowDesk) must remain intact.
2. **All derivative work stays GPLv2.** Do not relicense files or add permissive
   headers (MIT/Apache/etc.) to existing code.
3. **New files** should carry the same GPL header used by neighboring files.
4. **New dependencies** must be GPLv2-compatible. Flag any dependency addition to
   the user before introducing it.
5. **Public binary releases** must ship with corresponding complete source under GPLv2.

When in doubt about license implications of a change, **ask the user** — do not
guess.

## Tech stack

- **C++ core:** C++14 (`CMAKE_CXX_STANDARD 14`, extensions off). Build: CMake ≥ 3.4.
  Root project is still named `barrier` for heritage reasons.
- **GUI:** **Tauri** (Rust backend + React frontend) in `src/gui-tauri/`. See
  [`docs/design/tauri-gui.md`](docs/design/tauri-gui.md) for the full design.
- **Legacy GUI:** Qt 5 in `src/gui/` (being replaced by Tauri; not built by default
  in the Tauri flow).
- **Release notes:** towncrier (`towncrier.toml`, fragments in `doc/newsfragments/`).

## Repository layout

```
src/
  lib/          C++ core library: base, common, arch, mt, io, net, ipc,
                barrier (protocol), client, server, platform (OS-specific)
  cmd/          C++ binaries: barriers (server), barrierc (client), barrierd (daemon)
  gui/          LEGACY Qt GUI (not used by Tauri flow)
  gui-tauri/    NEW Tauri GUI (active)
    src-tauri/  Rust backend (Cargo.toml, src/, tauri.conf.json, Info.plist)
    src/        React frontend (App.tsx, components/, api.ts, types.ts)
    scripts/    build-bundled-barriers.sh, post-bundle.sh
  test/         mock, unittests, integtests, guitests
ext/            vendored third-party — avoid modifying
dist/           packaging per platform (inno, macos, rpm, wix)
res/            icons, desktop file, config.h.in
doc/            man pages, release notes, newsfragments, design/
cmake/          Version.cmake, Package.cmake
docs/design/    tauri-gui.md (the design doc)
```

Binary names (`barriers`/`barrierc`/`barrierd`) and the CMake project name (`barrier`)
are legacy. Renaming them is invasive and breaks downstream packagers — do not do
it without explicit user approval.

## Build & test

### C++ core (only needed when modifying the core, or building the bundled helper)

```bash
cmake -S . -B build -DCMAKE_BUILD_TYPE=Release -DBARRIER_BUILD_GUI=OFF -DBARRIER_BUILD_TESTS=OFF -DCMAKE_POLICY_VERSION_MINIMUM=3.5 -DCMAKE_OSX_SYSROOT=$(xcode-select -p)/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk
cmake --build build -j --target barriers
# binaries land in build/bin/
```

Note: CMake 4.x requires `-DCMAKE_POLICY_VERSION_MINIMUM=3.5` (upstream's
`cmake_minimum_required(VERSION 3.4)` is rejected). macOS 26 also needs
`-DCMAKE_OSX_SYSROOT` set explicitly or the linker can't find libc++.

### Tauri GUI (the main build)

```bash
cd src/gui-tauri

# Build the x86_64 barriers helper (REQUIRED before tauri build):
FLOWDESK_OPENSSL_ROOT="$HOME/openssl-x86_64" npm run build:core

# Build the frontend + Rust backend + .app bundle:
FLOWDESK_OPENSSL_ROOT="$HOME/openssl-x86_64" npm run tauri build -- --bundles app

# Dev mode (hot reload):
FLOWDESK_OPENSSL_ROOT="$HOME/openssl-x86_64" npm run tauri dev
```

### CRITICAL: x86_64 barriers requirement (macOS)

On Apple Silicon, the barriers helper **MUST be x86_64** (run via Rosetta), NOT
arm64-native. The arm64 build has degraded CGEventTap behavior — the cursor
stutters when crossing to a Windows client. This is the single most important
build constraint.

- `build-bundled-barriers.sh` defaults to `x86_64` on Apple Silicon.
- It needs an x86_64 OpenSSL at `FLOWDESK_OPENSSL_ROOT` (built via Rosetta).
  On this machine it's at `~/openssl-x86_64/`.
- The C++ `CMakeLists.txt` checks `FLOWDESK_OPENSSL_ROOT` env var before the
  brew paths, so the x86_64 lib is picked up for cross-arch builds.
- The Rust `supervisor.rs::resolve_binary` launches the x86_64 slice via
  `/usr/bin/arch -x86_64` even if a universal binary is present.

### Rust tests

```bash
cd src/gui-tauri/src-tauri
cargo test     # 7 unit tests + 1 integration test (spawns real barriers)
```

## macOS permissions (critical UX)

FlowDesk needs TWO macOS permissions, both bundle-level grants:

1. **Accessibility** — keyboard + mouse button capture.
2. **Screen Recording** — global mouse position (macOS 15+/26+ requires this for
   `CGEventGetLocation`; without it the cursor stutters even with Accessibility).

- The GUI's **「权限」(Permissions) tab** shows live status of both and links to
  the right System Settings pane.
- Screen Recording requires an **app restart** to take effect after granting.
- `Info.plist` declares `NSScreenCaptureUsageDescription` +
  `NSAppleEventsUsageDescription` (merged via `tauri.conf.json` `infoPlist`).
  macOS 15+ refuses to honor grants without these keys.
- Distribution: ad-hoc signing + clearing quarantine attributes (`xattr -cr`).
  Formal notarization requires a Developer ID Application certificate (not yet
  obtained). Use `ditto` to copy into /Applications to avoid Gatekeeper's slow
  async deep-scan on drag-copy.

## Release notes policy (towncrier)

**Every user-visible change must add a fragment** in `doc/newsfragments/`:

- `<slug>.feature` / `.bugfix` / `.security` / `.doc` / `.removal` / `.misc`
- One short line of human-readable text as the file's content.

## Version bumps are atomic

When changing the version, edit **all** of these in one commit:
- `Build.properties`
- `cmake/Version.cmake`
- `doc/barrierc.1`
- `doc/barriers.1`

## Git

- Default branch: `master`.
- Branch naming: `feature/*`, `fix/*`, `chore/*`, `docs/*`.
- Remotes:
  - `origin` → `helloxkk/flowdesk` (this fork)
  - `upstream` → `debauchee/barrier` (dormant original)
- Commit messages: lower-case conventional prefixes (`feat:`, `fix:`, `chore:`,
  `docs:`). Match surrounding history.

## Things to be careful with

- **barriers architecture** — must be x86_64 on Apple Silicon (see above). Do not
  "fix" it to arm64-native; the arm64 path stutters. The arm64 backup is kept at
  `barriers.arm64.bak` for diagnostics only.
- **`ext/`** — vendored third-party. Don't patch casually; if unavoidable, explain
  why in the commit message.
- **`src/lib/platform/`** — OS-specific. A fix for one OS must not break others.
- **`src/lib/barrier/`** — wire protocol. Incompatible changes break client/server
  interop across versions. Treat as sensitive.
- **Wayland** support is incomplete upstream; do not assume it works.
- **macOS port 24800** — if a zombie barriers process holds it in `UNE` state
  (uninterruptible), `kill -9` won't work; only a reboot clears it.

## Escalate to the user before

- Changing the license or introducing a new dependency.
- Renaming binaries or the CMake project.
- Bumping the version, pushing tags, or drafting a release.
- Large refactors of the protocol layer or platform abstraction.
- Changing the barriers build architecture away from x86_64.
