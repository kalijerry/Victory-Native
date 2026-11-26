# VIctory — NVIDIA Hybrid Optimization

A fork of [VIctory](https://github.com/Marvisak/VIctory), a compatibility layer
that runs an Xbox Series title natively on Windows from files the user supplies.

## The goal

Improve compatibility and runtime stability on NVIDIA hardware while preserving upstream rendering improvements. Stable 30 FPS gameplay without ray tracing remains a long-term target, not an achieved benchmark.

The way there is **porting, not emulating**: patching the title's renderer so it
behaves the way a PC renderer does, rather than having the layer watch and copy
the console's shared memory every frame. Shaders are translated ahead of time
into a versioned store that is kept between builds. Ray tracing stays optional
and off by default. The picture comes after the frame rate: texture pop-in and
lighting that is too bright or too dark when walking indoors are next in line.

**This repository contains no game files, code or assets.** You need your own
copy of the game. It is not affiliated with or endorsed by the game's developer
or publisher.

## NVIDIA hybrid branch: local test configuration (2026-11-26)

Active development: [hybrid-nvidia-optimization](https://github.com/spatsariya/victory-7900xtx/tree/hybrid-nvidia-optimization). This is an experimental NVIDIA-focused fork of [avatarmanz/VIctory-7900XTX](https://github.com/avatarmanz/VIctory-7900XTX), with selected compatibility fixes from other VIctory contributors.

| Component | Tested configuration |
|---|---|
| CPU | AMD Ryzen 7 3700X, 8 cores, 3.60 GHz |
| GPU | NVIDIA GeForce RTX 3060, 12 GB VRAM |
| RAM | 64 GB, 2667 MHz |
| Operating system | Windows x64 |
| Runtime | Direct3D 12, Safe launch profile, ray tracing off |
| Tested source commit | [ea88833](https://github.com/spatsariya/victory-7900xtx/commit/ea888334f360c252962cf55b1c2f184db7e556ec) |

### Observed results

| Test | Result |
|---|---|
| Loading-screen artwork | Renders at 30 FPS |
| Prologue 3D scenes | Render; approximately 1–3 FPS in early testing |
| Chapter 1 | Reached the boating sequence; approximately 3–4 FPS observed in later diagnostics |
| Character rendering | Improved in some views; missing facial geometry not fully verified as resolved |
| Stability | Access violation (0xc0000005, null read) around the boating sequence; root cause unknown |
| Wave64 mask recovery | Regression test passed; all 14 structure tests passed |
| Windows diagnostic build | Successful, six compiler warnings |

The hybrid branch includes a selective port of DexNom's Wave64 scalar-mask recovery fix (original commit `76749ce`). It preserves the existing 7900XTX renderer and shader improvements. This is not a stable or fully playable release; FPS varies by scene, shader cache state and hardware. The 30 FPS loading screen is not a gameplay benchmark. Further work targets runtime stability, character rendering, rejected compute pipelines and memory/descriptor overhead.

The upstream Radeon results below are retained for reference and are **not** measurements from the RTX 3060 test system.

## Upstream Radeon results (2026-11-26)

Upstream Radeon test machine: Windows 11, Intel i9-12900K, 64 GB RAM, Radeon
RX 7900 XTX, the Series X profile, ray tracing off.

| | |
|---|---|
| City street, renderer patches 1+2 on | about 53 ms a frame (about 19 fps) |
| Same scene before the patches | 57-75 ms (13-17 fps) |
| Shaders the title uses that translate | all of them (25,790 of 25,985; the rest are unused geometry shaders) |
| Memory | about 35 GB of private memory: **64 GB of RAM recommended** for now |

It started at 280 ms a frame in the same scenes three days earlier.

### Renderer patches (off by default while they are tested)

| Switch | What it does |
|---|---|
| `VICTORY_PORT_CONSTANTS=1` | The title's 174 MB constants heap is read by the card where the title writes it: no uploads, no fills |
| `VICTORY_PORT_DESCRIPTORS=1` | The title's descriptor-table stores are patched (byte-checked) to report the slots they write, instead of a scan of ~600,000 slots a frame |

What is known to be wrong:

- Trees, signs and far objects flicker in the distance in some places.
- Lighting is too bright or too dark after walking indoors, until the title has
  run for a while; rooms with little direct light are too dark without `-RayTracing`.
- Textures of what just came into view appear late while the title streams.
- The first run of a new build stutters while the driver makes pipelines
  (`victory warm` makes them ahead).
- Recording a frame in the world makes the title's own watchdog end it.

## How to run it

You need Windows 10/11 x64, a CPU with AVX2, a Direct3D 12 GPU with shader
model 6.6 and mesh shaders, Rust (the toolchain is pinned), the MSVC build
tools, and your own dump of the title.

```powershell
git clone --branch hybrid-nvidia-optimization https://github.com/spatsariya/victory-7900xtx.git
cd victory-7900xtx
copy victory.example.toml victory.toml      # then set your paths, outside this folder
cargo build --locked --profile diag -p victory --features diagnostics
.\scripts\play.ps1 -Exe 'C:\path\to\your\authorized\title.exe' -Safe
```

`scripts\play.ps1 -Safe` leaves out the newest, faster ways; try it first when
something is wrong. `-RayTracing` turns the title's ray-traced indirect light on.

On a hybrid-graphics PC, the runtime and offline pipeline checks prefer a
high-performance adapter with Shader Model 6.6. The runtime logs its adapter
name, PCI vendor and device IDs, and dedicated VRAM. Set
`VICTORY_D3D12_ADAPTER` to a zero-based DXGI index or part of the adapter's
name to select it explicitly. With the launcher, pass
`-More @{VICTORY_D3D12_ADAPTER='NVIDIA'}`.

Before the first run, ahead of time (both can be stopped and go on later):

```powershell
target\diag\victory.exe shaders <the shader archives>   # every shader translated, minutes
target\diag\victory.exe warm --used                     # pipelines earlier runs drew with, for a new driver or build
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

- **[avatarmanz/VIctory-7900XTX](https://github.com/avatarmanz/VIctory-7900XTX)**: the immediate base of this NVIDIA-focused fork.
- **DexNom**: original Wave64 scalar-mask recovery fix, selectively ported in `ea88833`.

This is a tree of other people's work with more on top. The history is kept, and
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
