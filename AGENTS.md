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

- **Language:** C++14 (`CMAKE_CXX_STANDARD 14`, extensions off).
- **Build:** CMake ≥ 3.4. Root project is still named `barrier` for heritage reasons.
- **GUI:** Qt 5 (optional via `BARRIER_BUILD_GUI`).
- **CI:** Azure Pipelines (`azure-pipelines.yml`).
- **Release notes:** towncrier (`towncrier.toml`, fragments in `doc/newsfragments/`).

## Repository layout

```
src/
  lib/        core library: base, common, arch, mt, io, net, ipc,
              barrier (protocol), client, server, platform (OS-specific)
  cmd/        binaries: barriers (server), barrierc (client), barrierd (daemon)
  gui/        Qt GUI
  test/       mock, unittests, integtests, guitests
ext/          vendored third-party — avoid modifying
dist/         packaging per platform (inno, macos, rpm, wix)
res/          icons, desktop file, config.h.in
doc/          man pages, release notes, newsfragments
cmake/        Version.cmake, Package.cmake
```

Binary names (`barriers`/`barrierc`/`barrierd`) and the CMake project name (`barrier`)
are legacy. Renaming them is invasive and breaks downstream packagers — do not do
it without explicit user approval.

## Build & test

```bash
# Configure + build (out-of-source)
cmake -S . -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build -j

# Run tests (no ctest wrapper — invoke binaries directly)
./build/bin/unittests
./build/bin/integtests
./build/bin/guitests   # only if BARRIER_BUILD_GUI=ON
```

Default build options: GUI ON, installer ON, tests ON, external gtest OFF.
`compile_commands.json` is emitted to `build/` for LSP/clangd.

## Release notes policy (towncrier)

**Every user-visible change must add a fragment** in `doc/newsfragments/`:

- `<slug>.feature` / `.bugfix` / `.security` / `.doc` / `.removal` / `.misc`
- One short line of human-readable text as the file's content.
- This is how changes surface in the next release's notes. Skipping it means the
  change is invisible to users.

## Version bumps are atomic

When changing the version, edit **all** of these in one commit:
- `Build.properties`
- `cmake/Version.cmake`
- `doc/barrierc.1`
- `doc/barriers.1`

See [`RELEASING.md`](RELEASING.md) for the full release procedure.

## Git

- Default branch: `master`.
- Branch naming: `feature/*`, `fix/*`, `chore/*`, `docs/*`.
- Remotes:
  - `origin` → `helloxkk/flowdesk` (this fork)
  - `upstream` → `debauchee/barrier` (dormant original)
- Commit message style follows existing history (lower-case conventional prefixes
  are common: `feat:`, `fix:`, `chore:`, `docs:`).

## Things to be careful with

- **`ext/`** — vendored third-party. Don't patch casually; if unavoidable, explain
  why in the commit message.
- **`src/lib/platform/`** — OS-specific. A fix for one OS must not break others.
  If you can't test a platform, narrow the change and say so.
- **`src/lib/barrier/`** — wire protocol. Incompatible changes break client/server
  interop across versions. Treat as sensitive.
- **Wayland** support is incomplete upstream; do not assume it works.

## Escalate to the user before

- Changing the license or introducing a new dependency.
- Renaming binaries or the CMake project.
- Bumping the version, pushing tags, or drafting a release.
- Large refactors of the protocol layer or platform abstraction.
