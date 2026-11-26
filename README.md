# Victory Native

A fork of [VIctory-7900XTX](https://github.com/avatarmanz/VIctory-7900XTX) (at its
commit 2de9980), itself a fork of [VIctory](https://github.com/Marvisak/VIctory): a
compatibility layer that runs an Xbox Series title natively on Windows from files the
user supplies. Everything below this section up to "The goal" is unchanged from
upstream; the history, authors and credits of both projects are kept.

## Translate, don't fake

The founding rule of Victory Native, for every part of it: rendering, shaders, ray tracing,
memory, audio, synchronisation, streaming and system APIs.

- **Translate** the behaviour of the Xbox hardware and APIs into its PC equivalent.
- Where no native equivalent exists, **reproduce the semantics**.
- **Never invent a result** to hide a feature that is wrong or missing: no stand-in light, no
  value, success code or texture made up.
- **Never mistake a visual enhancement for a renderer fix.**
- **Fix root causes.** A temporary workaround exists only to diagnose or to unblock an
  investigation: optional, off by default, documented, and stated in the log when it acts.
- **Optimise the implementation without changing what the game is meant to do.**
- **A problem is fixed when measurements show it,** not before.
- **Never hide an error, a crash, an incompatibility or a regression.**

How contributions are reviewed: [CONTRIBUTING.md](CONTRIBUTING.md). Rules for people and
agents working in the tree: [AGENTS.md](AGENTS.md). What departs from the console today:
[docs/workarounds.md](docs/workarounds.md) and the audit of inherited shortcuts,
[docs/shortcuts-audit.md](docs/shortcuts-audit.md).

Upstream is followed. Its changes are taken one by one (`git cherry-pick -x`) when they meet
these rules, and its open pull requests are reviewed here in the open, with their authors
credited. License: GPL-3.0-or-later, as upstream.

## The goal

**The Series X version at 4K output (drawn at 1440p) with ray tracing on,
looking the way it does on the Xbox**, on AMD RDNA cards (RX 7900 XTX, RX 7800 XT) first, other PC cards next.

In this order:

1. **A correct picture.** Textures and models reach full detail and stay there
   (no pop-in or pop-out, no flickering trees), and ray-traced lighting is right:
   not too dark, not too bright, no flicker.
2. **Then the frame rate:** 30 fps or more with ray tracing on, then as high a
   frame rate and as low frame times as the PC allows.
3. **Then smaller PCs:** 32 GB (later 16 GB) of RAM and smaller cards.

The way there is **porting, not emulating**: patching the title's renderer so it
behaves the way a PC renderer does, rather than having the layer watch and copy
the console's shared memory every frame. Shaders are translated ahead of time
into a versioned store that is kept between builds. The Series S profile and
ray tracing off stay available.

**This repository contains no game files, code or assets.** You need your own
copy of the game. It is not affiliated with or endorsed by the game's developer
or publisher.

## Where it stands (measured, 2026-11-26)

Measured by this fork on one machine: Windows 11, Intel i9-12900K, 64 GB RAM, Radeon
RX 7800 XT 16 GB. Series X profile, 4K output drawn at 1440p, ray tracing on
(`VICTORY_BVH_BUILDER=1`), renderer patches 1, 2, 5, 6 and 7 on, warm pipeline cache, the
same save and scripted input each run, last 90 s of a 480 s run in a boat scene:

| Build | Frame | Limited by |
|---|---|---|
| 2de9980 (this fork's base) | 103-116 ms (8.7-9.8 fps) over two runs | the CPU: the card is busy 60-85% of the frame |
| base + #363 and #364 (in review here) | 90 ms (11.1 fps), one run | the CPU |

**30 fps with ray tracing is far away.** What sets the frame is the thread that prepares
the title's submissions for the card. The goal is to cut that work by porting, not by
dropping any of it.

Known to be wrong, with causes where known: [docs/workarounds.md](docs/workarounds.md).
In short: trees flicker at the start of shots, lamp-lit rooms flicker, shallow water shows
striped artefacts, models stay at low detail for long while the title streams, and ray-traced
structures of moving objects are partly corrupt.

Upstream's own measurements (RX 7900 XTX, a city street, not reproduced here):

| Version | Standing | Walking | RAM |
|---|---|---|---|
| Series X, 4K drawn at 1440p, ray tracing on | 99 ms (10 fps) | ~110 ms (9 fps) | 45 GB |
| Series X, 4K drawn at 1440p, ray tracing off | 49 ms (20 fps) | ~62 ms (16 fps) | 45 GB |
| Series X, 1080p output, ray tracing off | 40 ms (25 fps) | ~43 ms (23 fps) | 42 GB |
| Series S, ray tracing off | 43 ms (23 fps) | ~64 ms (16 fps) | 37 GB |

Memory: about 30-45 GB of private memory and over 10 GB of VRAM; **64 GB of RAM is needed**
for now.

### Renderer patches (off by default while they are tested)

| Switch | What it does |
|---|---|
| `VICTORY_PORT_CONSTANTS=1` | The title's 174 MB constants heap is read by the card where the title writes it: no uploads, no fills |
| `VICTORY_PORT_DESCRIPTORS=1` | The title's descriptor-table stores are patched (byte-checked) to report the slots they write, instead of a scan of ~600,000 slots a frame |
| `VICTORY_PORT_CBV=1` | Translated shaders read constant buffers in place as raw bytes (~22,000 fills a frame to none; -4.7 ms) |
| `VICTORY_PORT_CBV_COPIES=1` | With `VICTORY_PORT_CBV=1`, table constant buffers in host copies are also viewed directly as raw bytes instead of copied through scratch fills (avoids `card_steps` bounce copies and waits for flights) |
| `VICTORY_PORT_GUARD=1` | The write guards' lock is shared by faults and refreshes, with a claim per 64 KiB block; the submission's planning runs beside it (title threads' waits 29 ms to ~2 ms a frame; -2 ms) |
| `VICTORY_PORT_JOURNAL=1` | The heaps' records are asked for at the title's submit, and the journal's notes are split into 64 parts with locks of their own (the submit thread's waits ~5 ms + 1 ms to ~0.3 ms; -1 to -3 ms) |

### Other switches

| Switch | What it does |
|---|---|
| `--console series-s` | The Series S profile: 1080p output, about 5 ms faster, a softer picture |
| `VICTORY_OUTPUT=1920x1080` | The title renders to that output size itself (Series X profile; about 16 ms less card time) |
| `VICTORY_FRAME_INTERVAL_MS=16.667` | Lets the title go above 30 fps (menus and videos reach 60; the game's clock stays right) |
| `VICTORY_TEXTURE_STORE=1` | Converted textures kept between runs (about 8 ms a frame for now: off) |
| `VICTORY_BORDERLESS=1` | The window covers the primary display with no border or title bar (windowed borderless: the picture is stretched to it, the render size stays; `play.ps1 -Borderless`) |
| `VICTORY_PORT_BVH_ARENA_WRITES=1` | Records CPU writes into private BVH arenas and refreshes them from the journal. Off by default. |

Pipelines are made before the title starts. `victory run --execute` opens a Victory Native
window with a loading screen and makes the host pipelines earlier runs drew with (the same work as
`victory warm --used`, in processes of their own); the title then starts and its window takes the
loading window's place. Only pipelines that are new, or made under another translator or driver,
are made; failures are shown and logged, kept in `warm-bad.txt` and tried again once the
translator or the driver changes (`victory warm --again` tries them now). Nothing of it runs once
the title has started. The pipelines the title creates are written down for this by default
(`VICTORY_PIPELINE_BOOK=0`: not kept).

| Switch | What it does |
|---|---|
| `VICTORY_LAUNCH_WARM=used` | Default: the loading screen makes the pipelines earlier runs drew with |
| `VICTORY_LAUNCH_WARM=all` | Every pipeline the title ever created (84,000: long the first time) |
| `VICTORY_LAUNCH_WARM=0` | No loading screen; pipelines are made when a draw first needs them |

A pipeline met for the first time in a run is still made when a draw needs it. Until it exists,
the draws that need it are dropped and counted in the log (listed in
[docs/workarounds.md](docs/workarounds.md)).

## How to run it

You need Windows 10/11 x64, a CPU with AVX2, a Direct3D 12 GPU with shader
model 6.6 and mesh shaders, Rust (the toolchain is pinned), the MSVC build
tools, and your own dump of the title.

```powershell
git clone https://github.com/avatarmanz/VIctory-7900XTX
cd VIctory-7900XTX
copy victory.example.toml victory.toml      # then set your paths, outside this folder
cargo build --locked --profile diag -p victory --features diagnostics
scripts\play.ps1 -Exe <path of your copy's executable>
```

On an RTX 5080, Ryzen 9 9950X3D and 32 GB PC: `scripts\play-rtx5080.ps1`, and read
[docs/pc-rtx5080-9950x3d.md](docs/pc-rtx5080-9950x3d.md) first.

`scripts\play.ps1 -Safe` leaves out the newest, faster ways; try it first when
something is wrong. `-RayTracing` turns the title's ray-traced indirect light on.

On a hybrid-graphics PC, the runtime and offline pipeline checks prefer a
high-performance adapter with Shader Model 6.6. The runtime logs its adapter
name, PCI vendor and device IDs, and dedicated VRAM. Set
`VICTORY_D3D12_ADAPTER` to a zero-based DXGI index or part of the adapter's
name to select it explicitly. With the launcher, pass
`-More @{VICTORY_D3D12_ADAPTER='NVIDIA'}`.

The loading screen does the pipelines; for diagnostics and scripts the commands stay (both can
be stopped and go on later):

```powershell
target\diag\victory.exe shaders <the shader archives>   # every shader translated, minutes
target\diag\victory.exe warm --used                     # what the loading screen does, on the command line
```

Commands, debugging switches and upstream's own description:
[docs/README-fxceless.md](docs/README-fxceless.md). Every `VICTORY_...` switch is
described in a comment where it is read.

## What is different here

In short; the reasons and the measurements are in [docs/findings.md](docs/findings.md).

- **List calls are written down, not carried out, on the title's threads**
  (`VICTORY_THIN`): eight threads of ours translate them.
- **Submissions are prepared by a thread of ours and joined**
  (`VICTORY_ASYNC_SUBMIT`, `VICTORY_JOIN_PARTS`), with what can be spread over
  threads spread: refreshes, compares (AVX2), descriptor records, landings.
- **What shaders wrote comes back beside the write guards**, by the dwords that
  changed, on a thread of its own (`VICTORY_RELAY`, `VICTORY_LAND`).
- **Buffer fills that need bytes still on the card are done on the card**
  (`VICTORY_CARD_STEPS`) instead of waiting for it.
- **The title's writes are caught more cheaply**: one view opened at a fault,
  blocks that rest, places in the title's code that look before they write.
- **Textures go up on a copy queue of their own** and a slot shows one only after
  its upload ran, which also ended a device loss that came every 15 minutes or so.
- **Boats float**: textures that shaders write and the title's CPU reads go back
  to title memory (from fxceless and Geckomacabre, on by default here).
- **Frame record and replay** with per-pass times and shader variants compared
  turn by turn ([docs/replay.md](docs/replay.md)).

## Credits

This is a tree of other people's work with more on top.

- **[VIctory-7900XTX](https://github.com/avatarmanz/VIctory-7900XTX)** by avatarmanz and its
  contributors: everything up to 2de9980, this fork's base, and the pull requests reviewed here. The history is kept, and
commits taken from elsewhere name the commit and its author.

- **[VIctory](https://github.com/Marvisak/VIctory)** by Marvisak and its
  contributors: the loader, the runtime services, the Direct3D 12 back end and
  the shader recompiler.
- **[fxceless/victory-fork](https://github.com/fxceless/victory-fork)**: the fork
  this one started from, and the people whose pull requests it carries.
- **[DexNom/VIctory](https://github.com/DexNom/VIctory)**: ray tracing fixes,
  cube gradients, sparse textures.
- **[Belobug/VIctory-Belobug-AMD](https://github.com/Belobug/VIctory-Belobug-AMD)**:
  the ambient light idea, pipeline workers, and work by romainhedouin.
- **[FQY55kai/ZJOU-VICTORY](https://github.com/FQY55kai/ZJOU-VICTORY)**.
- **[myokex/victory-amd](https://github.com/myokex/victory-amd)**: the guard over
  every view of a page (with Spexivus, joroyt and tightfall), and Joshua Odom's
  reads into title memory.
- **Geckomacabre** and **cxy0072019-sketch**: the texture read-back.
- **[XCL](https://github.com/ivall/xcl)** (GPL-2.0-only) was read for how the same
  title behaves under another layer; no code of it is here.
- This fork's changes were made with [Claude Code](https://claude.com/claude-code).

## License

GPL-3.0-or-later, as VIctory. See [LICENSE](LICENSE); the one file that is not
covered is named in [docs/asset-policy.md](docs/asset-policy.md).
