# CLAUDE.md

Guidance for Claude Code (and other coding agents) when working in this repository.

## What is FlowDesk

FlowDesk is a software KVM utility: share one keyboard and mouse across multiple
computers over the network. It is a fork of [Barrier](https://github.com/debauchee/barrier)
(GPLv2), which itself was forked from Symless's Synergy 1.9 codebase.

- **License:** GPLv2, with the OpenSSL exemption stated at the top of [`LICENSE`](LICENSE).
- **Language:** C++14 (`CMAKE_CXX_STANDARD 14`, no extensions).
- **Build system:** CMake ≥ 3.4 (`CMakeLists.txt` at repo root, `project(barrier)`).
- **Qt version:** Qt 5 (GUI). GUI is optional via `-DBARRIER_BUILD_GUI=OFF`.

## Critical: license compliance (read before editing)

FlowDesk is GPLv2. Every change you make MUST keep the project GPLv2-compliant:

1. **Never delete** upstream copyright lines in `LICENSE` or in source-file headers.
   The chain is Debauchee → Symless → Nick Bolton → Chris Schoeneman, plus FlowDesk.
2. **New source files** should carry the same GPL header used in existing files in
   the same directory (copy from a neighbor).
3. **New dependencies** must be GPLv2-compatible. When in doubt, ask the user — do
   not silently introduce MIT/Apache/BSD code into a GPL-only build.
4. **Do not relicense.** This repo stays GPLv2; do not add permissive headers.

## Directory layout

```
src/
  lib/        # core library, split by concern:
    base/ common/ arch/ mt/ io/ net/ ipc/
    barrier/  # the shared protocol/state machine
    client/ server/   # the two roles
    platform/         # OS-specific code (X11, Wayland, Win32, macOS)
  cmd/
    barriers/  # server binary  (historically "barriers")
    barrierc/  # client binary  (historically "barrierc")
    barrierd/  # daemon
  gui/        # Qt GUI (barrier.app)
  test/       # mock/ unittests/ integtests/ guitests/
ext/          # vendored third-party libs (do not modify casually)
dist/         # packaging: inno/ (Win), macos/, rpm/, wix/
res/          # icons, desktop file, config.h.in, install assets
doc/          # man pages, release notes, newsfragments/
cmake/        # Version.cmake, Package.cmake — touched on version bumps
azure-pipelines.yml, azure-pipelines/  # CI
```

Note: binary names (`barriers`, `barrierc`, `barrierd`) and the CMake project name
(`barrier`) still reflect the upstream heritage. Renaming them is invasive —
coordinate with the user before attempting it.

## Build

Out-of-source build, standard CMake flow:

```bash
cmake -S . -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build -j
# binaries land in build/bin/
```

Common options (all default ON except external gtest):
- `-DBARRIER_BUILD_GUI=ON/OFF`
- `-DBARRIER_BUILD_INSTALLER=ON/OFF`
- `-DBARRIER_BUILD_TESTS=ON/OFF`
- `-DBARRIER_USE_EXTERNAL_GTEST=ON/OFF`

macOS note: use `osx_environment.sh` and see `dist/macos/` for packaging.
`CMAKE_EXPORT_COMPILE_COMMANDS=ON` is already set — `compile_commands.json` is in
`build/` for clangd/LSP.

## Tests

```bash
# build with tests on (default), then run the test binaries directly:
cmake --build build -j
./build/bin/unittests
./build/bin/integtests
# GUI tests only when BARRIER_BUILD_GUI=ON:
./build/bin/guitests
```

There is no top-level `ctest` wrapper by default — invoke the binaries.

## Release notes (towncrier) — required for user-visible changes

User-visible changes (features, bugfixes, security, docs, removals) MUST add a
fragment under `doc/newsfragments/`:

- Filename convention: `<issue-or-slug>.<type>` where type ∈
  `.feature`, `.bugfix`, `.security`, `.doc`, `.removal`, `.misc`.
- Content is one short line describing the change (used directly in release notes).
- See `doc/newsfragments/README.md`. Version bump + release is driven by
  `RELEASING.md` + `towncrier.toml`.

## Version bumps touch these files together

When changing the version (do not edit one without the others):
- `Build.properties`
- `cmake/Version.cmake`
- `doc/barrierc.1`, `doc/barriers.1`

## Git workflow

- `master` is the default branch and is the released baseline.
- Work on feature/fix branches: `feature/<slug>`, `fix/<slug>`, `chore/<slug>`.
- Two remotes are configured:
  - `origin` → this fork (`helloxkk/flowdesk`)
  - `upstream` → `debauchee/barrier` (the original, now mostly dormant)
- To pull in upstream fixes occasionally: `git fetch upstream && git merge upstream/master`.
- Commit messages: conventional-ish prefix is conventional in this repo
  (`feat:`, `fix:`, `chore:`, `docs:`). Match surrounding history.

## Things to be careful with

- `ext/` is vendored third-party code. Prefer not to patch it; if a patch is
  unavoidable, document why in the commit message.
- `src/lib/platform/` is OS-specific; a change for one platform should not break
  others. If you can't test a platform, say so and limit scope.
- The clipboard/protocol code in `src/lib/barrier/` is wire-protocol sensitive —
  changing it may break client/server compatibility across versions.
- Wayland support is incomplete upstream; do not assume it works.

## When you're unsure

Ask the user before: changing the license, adding a dependency, renaming binaries
or the CMake project, bumping versions, or pushing tags/releases.
