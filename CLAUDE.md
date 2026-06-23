# CLAUDE.md

Guidance for Claude Code (and other coding agents) when working in this repository.

## What is FlowDesk

FlowDesk is a software KVM utility: share one keyboard and mouse across multiple
computers over the network. It is a fork of [Barrier](https://github.com/debauchee/barrier)
(GPLv2), which itself was forked from Symless's Synergy 1.9 codebase.

- **Architecture:** C++ core (`barriers` binary, unchanged) + **Tauri GUI**
  (Rust backend + React frontend) that spawns barriers as a subprocess and
  supervises via stdout/stdin. macOS does **not** use the IPC protocol
  (that's Windows service mode only).
- **License:** GPLv2, with the OpenSSL exemption stated at the top of [`LICENSE`](LICENSE).
- **C++ core:** C++14 (`CMAKE_CXX_STANDARD 14`, no extensions), CMake ≥ 3.4.
- **Tauri GUI (active):** `src/gui-tauri/`. See
  [`docs/design/tauri-gui.md`](docs/design/tauri-gui.md) for the full design.
- **Qt5 GUI (legacy):** `src/gui/` — being replaced by Tauri; not built in the
  Tauri flow.

## Critical: license compliance (read before editing)

FlowDesk is GPLv2. Every change you make MUST keep the project GPLv2-compliant:

1. **Never delete** upstream copyright lines in `LICENSE` or in source-file headers.
   The chain is Debauchee → Symless → Nick Bolton → Chris Schoeneman, plus FlowDesk.
2. **New source files** should carry the same GPL header used in existing files in
   the same directory (copy from a neighbor).
3. **New dependencies** must be GPLv2-compatible. When in doubt, ask the user.
4. **Do not relicense.** This repo stays GPLv2; do not add permissive headers.

## Directory layout

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
  test/         mock/ unittests/ integtests/ guitests/
ext/            vendored third-party libs (do not modify casually)
dist/           packaging: inno/ (Win), macos/, rpm/, wix/
res/            icons, desktop file, config.h.in, install assets
doc/            man pages, release notes, newsfragments/
docs/design/    tauri-gui.md (the design doc)
cmake/          Version.cmake, Package.cmake
```

Binary names (`barriers`, `barrierc`, `barrierd`) and the CMake project name
(`barrier`) still reflect the upstream heritage. Renaming them is invasive —
coordinate with the user before attempting it.

## Build

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

### C++ core (only when modifying core, or building helper manually)

```bash
cmake -S . -B build -DCMAKE_BUILD_TYPE=Release -DBARRIER_BUILD_GUI=OFF -DBARRIER_BUILD_TESTS=OFF -DCMAKE_POLICY_VERSION_MINIMUM=3.5 -DCMAKE_OSX_SYSROOT=$(xcode-select -p)/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk
cmake --build build -j --target barriers
```

CMake 4.x requires `-DCMAKE_POLICY_VERSION_MINIMUM=3.5` (upstream's
`cmake_minimum_required(VERSION 3.4)` is rejected). macOS 26 needs
`-DCMAKE_OSX_SYSROOT` set explicitly or the linker can't find libc++.

## CRITICAL: x86_64 barriers requirement (macOS)

On Apple Silicon, the barriers helper **MUST be x86_64** (run via Rosetta), NOT
arm64-native. The arm64 build has degraded CGEventTap behavior — the cursor
stutters when crossing to a Windows client. This is the single most important
build constraint.

- `build-bundled-barriers.sh` defaults to `x86_64` on Apple Silicon.
- Needs x86_64 OpenSSL at `FLOWDESK_OPENSSL_ROOT`. On this machine: `~/openssl-x86_64/`.
- `CMakeLists.txt` checks `FLOWDESK_OPENSSL_ROOT` env var before brew paths.
- `supervisor.rs::resolve_binary` launches the x86_64 slice via `/usr/bin/arch -x86_64`.
- Do NOT "fix" it to arm64-native. The arm64 backup is `barriers.arm64.bak`.

## Tests

```bash
cd src/gui-tauri/src-tauri
cargo test     # 7 unit tests + 1 integration test (spawns real barriers)
```

## macOS permissions (critical UX)

FlowDesk needs TWO macOS permissions, both bundle-level grants:

1. **Accessibility** — keyboard + mouse button capture.
2. **Screen Recording** — global mouse position (macOS 15+/26+ requires this for
   `CGEventGetLocation`; without it the cursor stutters even with Accessibility).

- The GUI's **「权限」(Permissions) tab** shows live status and links to System Settings.
- Screen Recording requires an **app restart** to take effect after granting.
- `Info.plist` declares `NSScreenCaptureUsageDescription` +
  `NSAppleEventsUsageDescription` (merged via `tauri.conf.json` `infoPlist`).
- Distribution: ad-hoc signing + clearing quarantine (`xattr -cr`). Use `ditto`
  to copy into /Applications to avoid Gatekeeper's slow async deep-scan.

## Release notes (towncrier) — required for user-visible changes

Add a fragment under `doc/newsfragments/`:
- `<slug>.feature` / `.bugfix` / `.security` / `.doc` / `.removal` / `.misc`
- One short line describing the change.

## Version bumps touch these files together

- `Build.properties`
- `cmake/Version.cmake`
- `doc/barrierc.1`, `doc/barriers.1`

## Git workflow

- `master` is the default branch and is the released baseline.
- Work on feature/fix branches: `feature/<slug>`, `fix/<slug>`, `chore/<slug>`.
- Two remotes: `origin` → this fork (`helloxkk/flowdesk`),
  `upstream` → `debauchee/barrier` (dormant).
- Commit messages: lower-case conventional prefixes (`feat:`, `fix:`, `chore:`, `docs:`).

## Things to be careful with

- **barriers architecture** — must be x86_64 on Apple Silicon. See CRITICAL above.
- `ext/` is vendored third-party code. Prefer not to patch it.
- `src/lib/platform/` is OS-specific; a change for one platform must not break others.
- `src/lib/barrier/` is wire-protocol sensitive.
- Wayland support is incomplete upstream; do not assume it works.
- If port 24800 is held by a zombie barriers in `UNE` state, only a reboot clears it.

## When you're unsure

Ask the user before: changing the license, adding a dependency, renaming binaries
or the CMake project, bumping versions, pushing tags/releases, or changing the
barriers build architecture away from x86_64.
