# Audit of inherited shortcuts (upstream 2de9980)

Generated from the code: every place the upstream code marks itself as `shortcut:`, i.e. a behaviour that does not reproduce the console's semantics in full. Each one is classified by its owner as:
- **FAKE**: invents a result (a value, success, light, an empty texture) to hide something missing. Replace with a real implementation; until then, only behind an off-by-default, logged switch.
- **APPROXIMATION**: a narrower semantics that is right for what the title does today. Replace or prove (test or assertion that logs when the assumption breaks).
- **OK**: exact for the console (the comment is conservative); note why.

167 shortcuts. Status column: TODO until classified.

## victory-formats (4; owner: codex)

| where | what the code says | class | plan |
|---|---|---|---|
| `crates/victory-formats/src/bcpack.rs:406` | one lane step at a time (BC1 roughly 30-60 MB/s, BC7 15-20 MB/s on one core); if texture streaming needs more, run the schedule only to split the input per lane and decode the lanes apart. | TODO | |
| `crates/victory-formats/src/texture.rs:532` | no addressing for the XOR modes (`_X`, `_T`) and MSAA; they need the GPU's pipe and bank configuration (Series X and S differ). Titles use them for render targets and depth, which the host renders; add them when a title copies into or out of one. | TODO | |
| `crates/victory-formats/src/texture.rs:822` | those formats have no layout here; add them when a title uses one. | TODO | |
| `crates/victory-formats/src/texture.rs:873` | no compression metadata planes (HTILE, CMASK, FMASK, DCC); the console adds them to render targets and depth buffers unless the description denies them, so those are smaller here than on the console. The host GPU renders them, so nothing reads that memory; add the planes if a title sizes a pool from | TODO | |

## victory-gpu (80; owner: gemini / coordinator (memory, watch, ports, RT))

| where | what the code says | class | plan |
|---|---|---|---|
| `crates/victory-gpu/src/console.rs:19` | later system updates gave Series S titles more (amount not published here); the title sizes nothing from this figure (it goes by the device type), so update it if a title does. | TODO | |
| `crates/victory-gpu/src/copystat.rs:653` | the walk takes about 0.6 s on a thread of its own, and a query of a region of gigabytes holds up other threads' memory calls for its length (89 ms for the 6.2 GB pool), so it runs with every sixth memory line only. | TODO | |
| `crates/victory-gpu/src/descriptor.rs:1721` | a placement is recognised by its size matching the count, since the plain and extended descriptions share the slot and the interface version; find the flag that tells them apart if a title trips over this. | TODO | |
| `crates/victory-gpu/src/descriptor/host.rs:1687` | descriptors past the host's limit are left out; a title that reaches them needs remapping. They read the null slot instead. | TODO | |
| `crates/victory-gpu/src/descriptor/host.rs:2040` | the ranges are kept in one place for all heaps; the heap whose pass runs first takes them (a title has one view heap). | TODO | |
| `crates/victory-gpu/src/descriptor/host.rs:2169` | the starts are kept in one place for all heaps; the heap whose pass runs first takes them (a title has one view heap). Levels on demand (`crate::tiles`): the card has room, so the slots of up to [`Self::WAITING_A_PASS`] placed textures whose upload left a held level without tiles at least a second a | TODO | |
| `crates/victory-gpu/src/descriptor/host.rs:2181` | kept in one place for all heaps, as the starts below. | TODO | |
| `crates/victory-gpu/src/descriptor/host.rs:3984` | what shaders write stays in the host copy; title memory (and so the CPU, and copies run on the CPU) does not see it. | TODO | |
| `crates/victory-gpu/src/descriptor/host.rs:4111` | such a view (sRGB of UNORM, say) reads without its conversion; cast with castable formats if colors come out wrong. | TODO | |
| `crates/victory-gpu/src/descriptor/host.rs:4511` | unordered access to a texture the title placed itself is not mirrored; give it a resource as shader views of such textures get when a dispatch writes one. Each texture is said once (the first 64), with how many times such a view was asked for so far: what shaders write through them is lost. | TODO | |
| `crates/victory-gpu/src/descriptor/host.rs:5061` | slots with no view at all (kind unknown) get this null 2D view; null buffer views there did not stop the world's low-address page faults (2026-11-24, run w10), so they are not the cause. | TODO | |
| `crates/victory-gpu/src/device.rs:162` | the creation interface is the only sign of the newer layouts we have (the title, April 2026, versus Minecraft, March 2022); look for a better one if a title breaks the rule. | TODO | |
| `crates/victory-gpu/src/device.rs:309` | every use of every format; answer from the host device once there is one. | TODO | |
| `crates/victory-gpu/src/device.rs:357` | not implemented. Answering `S_OK` here without making the pipeline would hand the title whatever its stack held as an object. | TODO | |
| `crates/victory-gpu/src/draw.rs:460` | a list the title runs a second time without a reset then reads slots others may have taken (logged once). | TODO | |
| `crates/victory-gpu/src/draw.rs:813` | a root view has no size; constant buffers are taken as 64 KiB (cut at their buffer's end), other views as all that is left of their buffer. (They were taken as 1 MiB: 1,452 of the 2,021 mesh vertex streams of a cutscene frame reach further, and what lay past the first megabyte was never brought up t | TODO | |
| `crates/victory-gpu/src/draw.rs:1135` | a view without a description or a format has no null view to stand for it and is left out, as before. | TODO | |
| `crates/victory-gpu/src/draw.rs:1241` | the render target views are made again from the handles' descriptors as they are now, not as they were when the title set the targets (a descriptor rewritten in between names another view). | TODO | |
| `crates/victory-gpu/src/draw.rs:1333` | found again here, not taken from the list's slot. | TODO | |
| `crates/victory-gpu/src/draw.rs:1819` | a root view has no size: the view, the refresh and the copy back cover all that is left of the title's buffer (`VICTORY_ROOT_UAV_MB` sets a limit for all three). | TODO | |
| `crates/victory-gpu/src/draw.rs:2019` | the buffers are read, not written: what such a shader writes through a picked unordered access view stays in the host copy. A list refreshes them where it first runs such a shader; title writes recorded later in the same list are not seen. | TODO | |
| `crates/victory-gpu/src/draw.rs:2048` | what the title writes into such buffers between lists of one frame is seen a frame late. | TODO | |
| `crates/victory-gpu/src/draw.rs:2244` | uploaded once, and again only when the title rewrites the slot. A placed texture not all mapped yet is uploaded with what is there and looked at again (1, 2, 4, ... frames later, at most about a second): uploaded again when more of it is mapped. Pages mapped in place of others, and contents changed  | TODO | |
| `crates/victory-gpu/src/draw.rs:3994` | in a pipeline without that stage, a rectangle with no per-vertex inputs is drawn as its first triangle (three vertices, half of it), and one with vertex buffers is not drawn. | TODO | |
| `crates/victory-gpu/src/draw.rs:4277` | whole textures have one state, so a view that writes one of two planes counts as writing. | TODO | |
| `crates/victory-gpu/src/dxgi.rs:152` | memory sizes are the title's split (GPU-optimal, standard), not measured; revisit once memory is emulated. | TODO | |
| `crates/victory-gpu/src/dxgi.rs:233` | not yet the host display's own size by default; make it so once a title is seen to render for the mode it is given. | TODO | |
| `crates/victory-gpu/src/dxgi.rs:253` | fixed mode, not the host display's; report the host's rates if a title paces by them. | TODO | |
| `crates/victory-gpu/src/fence.rs:121` | a null event means "block until reached"; nothing can reach it while no GPU work runs, so fail instead of hanging. | TODO | |
| `crates/victory-gpu/src/frame.rs:9` | event offsets are ignored (every event is at the tick); tie ! the clock to presentation if a title paces by them. ! ! `VICTORY_FRAME_INTERVAL_MS=<ms>` ticks at that interval whatever the ! title sets (16.667: 60 a second; 0: no wait at all, every wait ends at ! once); unset, the title's interval as  | TODO | |
| `crates/victory-gpu/src/host.rs:4` | every submission is waited for, so CPU and GPU never overlap ! (unless `VICTORY_LATE_WAIT=1`, see [`late`]); ! overlap them with per-submission fences once more than clears run. | TODO | |
| `crates/victory-gpu/src/host.rs:1425` | a view must last until its list has run; with one or two passes a frame the ring is far from coming round. | TODO | |
| `crates/victory-gpu/src/host.rs:1713` | the depth plane's twin is not given the plane (that is a pass the other way: the plane read by a shader, the twin as its target). The twin then lacks what draws and clears wrote to the plane since shaders last wrote the twin. In the title nothing writes that plane but the twin's pass (its 192 draws  | TODO | |
| `crates/victory-gpu/src/host.rs:6657` | every depth target bound for a draw counts as written (also when the draw only tests), and every unordered access view a shader names as written; views a shader picks from the heap by a computed index are not seen here at all. | TODO | |
| `crates/victory-gpu/src/host.rs:6707` | the depth plane's twin is not drawn into the plane here: the pass sets a pipeline and a target of ours, which a list still recording the title's commands cannot take. This use sees the plane as it was; the pass runs when the list closes (`twins_to_depth`). Counted. | TODO | |
| `crates/victory-gpu/src/host.rs:7159` | one buffer per copy; pool them once uploads are frequent. | TODO | |
| `crates/victory-gpu/src/indirect.rs:31` | the bounds (below 2^23 vertices an instance, 2^26 vertex instances, 65,535 ! groups per dimension, in x 2^20 ([`widest`]), and 2^20 per dispatch, one-dimensional mesh ! dispatches, the budget) cut legitimate work beyond them; raise them if ! the title needs more. | TODO | |
| `crates/victory-gpu/src/inplace.rs:82` | while a window is in place the title cannot decommit, free or protect its pages (the call fails with error 87; the probe shows it). The title's heaps are allocated once and kept, and nothing of the kind was seen; [`title_memory_freeing`] is there for the runtime to call before such a call once it ta | TODO | |
| `crates/victory-gpu/src/list.rs:304` | a bottom-of-pipe write is done when its part has run, not at its place in the part: a queue waiting for it starts later than on the console, never earlier, and nothing can wait for itself that way (a part has no wait after its first work). | TODO | |
| `crates/victory-gpu/src/list.rs:324` | without a new host list the wait stands before the part it was recorded in. | TODO | |
| `crates/victory-gpu/src/list.rs:375` | a list the submission names a second time runs again only if it has one part and that part is in the entry being filled; otherwise the second naming is dropped (logged once). | TODO | |
| `crates/victory-gpu/src/list.rs:719` | whole resources, from the state the list tracked (`HostList::transition`); subresources are not told apart. | TODO | |
| `crates/victory-gpu/src/list.rs:1486` | no query runs on the host GPU; results are plausible values, not measured ones (time spans come out as the submission's length). | TODO | |
| `crates/victory-gpu/src/list.rs:1799` | dispatches whose group count is in GPU memory (indirect) are dropped; make them ExecuteIndirect when the title uses them. | TODO | |
| `crates/victory-gpu/src/list.rs:1865` | dropped until their shaders translate (decision 0007). | TODO | |
| `crates/victory-gpu/src/list.rs:1878` | signatures that change root arguments are dropped (logged once per signature). | TODO | |
| `crates/victory-gpu/src/list.rs:1967` | the comparison codes are taken to be the comparison functions' (1 never .. 8 always, value against reference); one not known counts as met, and so does memory that is not mapped.  The value is read through the kernel, which reports memory that is not there instead of faulting (as the copies of title | TODO | |
| `crates/victory-gpu/src/list.rs:2051` | a wait holds its whole list (and submission) back, not only the commands after it. | TODO | |
| `crates/victory-gpu/src/list.rs:2338` | 0x470's purpose is unknown; ignored until a title shows it matters. | TODO | |
| `crates/victory-gpu/src/memory.rs:65` | a page held for anything else is kept for the rest of the run. | TODO | |
| `crates/victory-gpu/src/memory.rs:66` | pages a list does not name are looked at again in turn, over ! [`SWEEP`] frames. | TODO | |
| `crates/victory-gpu/src/memory.rs:340` | a page of a title buffer inside this one that has a copy of its own is given this copy too. | TODO | |
| `crates/victory-gpu/src/pipeline.rs:12` | only vertex plus pixel pipelines from streams and compute ! pipelines get host pipelines; mesh, tessellation and geometry stages, ! and the plain graphics description, keep their containers only. The 32-byte ! packet after the header stays zero: titles only copy it inline, which ! our null command-l | TODO | |
| `crates/victory-gpu/src/pipeline.rs:265` | and only with `VICTORY_BVH_BUILDER=1`, until a run has shown the whole builder and the host side of title memory by address (`memory.rs`) working on a card; nothing reads the structures before the programs that walk them translate. | TODO | |
| `crates/victory-gpu/src/pipeline.rs:309` | not a way to run: the title also reads results back (sizes, counters) and gets none, and a copy's pages are overwritten from title memory when the title writes them. | TODO | |
| `crates/victory-gpu/src/pipeline.rs:649` | the extensions (console shader settings) are ignored, as is the rest of the description. | TODO | |
| `crates/victory-gpu/src/port.rs:43` | it is never unmapped, a ! few hundred MB of address space at most). ! ! What it assumes: the CPU steps of ours that read this memory at ! submission (fills of constant buffers, the title's buffer copies done on ! the CPU) read what the title's CPU wrote, not what a submission still in ! the air writ | TODO | |
| `crates/victory-gpu/src/port.rs:217` | the view stays mapped for good (the heap holds its pages while anything names it). | TODO | |
| `crates/victory-gpu/src/primitive.rs:8` | the fourth vertex is computed from vertex inputs, not from ! vertex shader outputs as the hardware does; that is the same for vertex ! shaders linear in their inputs (blits, 2D quads). Inputs in formats ! [`components`] does not know keep the first vertex's value, and so do ! components past the end | TODO | |
| `crates/victory-gpu/src/queue.rs:210` | a wait still holds its whole list, not only the commands after it; and lists named twice among those held stay in one piece. | TODO | |
| `crates/victory-gpu/src/queue.rs:911` | one plane, no scaling or placement, the source expected in the common state. | TODO | |
| `crates/victory-gpu/src/queue.rs:940` | what a queue still holds at a present runs now, whatever its fence: a wait is not kept across frames. | TODO | |
| `crates/victory-gpu/src/resource.rs:271` | copies between such a target and a texture the title fills itself in the 16-bit format are skipped ([`stands_in`]); convert them if a pass turns out to need one. | TODO | |
| `crates/victory-gpu/src/resource.rs:351` | whole textures are copied, not the mips and slices written. A depth texture with a stencil plane gets a twin for each plane ( the title's tiled lighting writes each pixel's material class into the depth plane and masks into the stencil plane, then draws each class with the depth test "equal"): the s | TODO | |
| `crates/victory-gpu/src/resource.rs:994` | the console runtime's own choice of page size and memory type is not known; this is the plausible one. | TODO | |
| `crates/victory-gpu/src/resource.rs:2058` | [`host_resource`] hands out the host texture without holding it; a thread that asked for it and has not yet recorded its use (which makes the list or the slot hold it) is protected by that idle time alone. Every caller uses it at once, within the title's call. Called before a submission starts, by t | TODO | |
| `crates/victory-gpu/src/shadow.rs:13` | only the ranges draws are known to read are brought up to ! date (vertex and index buffer views, constant buffers in tables, root ! descriptors, buffer views in tables); buffers read through directly ! indexed heaps keep what their first use copied. | TODO | |
| `crates/victory-gpu/src/shadow.rs:2291` | a piece with very many runs has them joined over small gaps, where the old race is left. | TODO | |
| `crates/victory-gpu/src/shadow.rs:3225` | what shaders wrote is found by keeping the range's bytes before the submission (a copy per declared range and submission); a GPU-side dirty mask would save that if big ranges make it slow. | TODO | |
| `crates/victory-gpu/src/shadow.rs:3231` | another host copy of the same bytes (a title buffer inside this one with a copy of its own, a texture in it) is not told: the card's writes are not seen by write watching. | TODO | |
| `crates/victory-gpu/src/shadow.rs:5310` | a window is 4 MiB however little of it is read (the memory line says how much was ever asked for, and how much was ever copied); [`window`] cuts the windows of asks that give a length on a finer grid. | TODO | |
| `crates/victory-gpu/src/shadow.rs:5323` | the default stays 4 MiB until a run with the memory line's window counts has shown what the finer grids give; the same memory asked for with different ranges then lies in more than one window. | TODO | |
| `crates/victory-gpu/src/shadow.rs:5359` | off unless `VICTORY_WINDOW_TAILS=1`. A longer tail is another host buffer than the shorter one: what was bound to the shorter stays right only because every list brings up to date the copy it bound (`draw.rs` keeps a root view's copy with its slot). It pays only together with `VICTORY_WINDOW_KB` bel | TODO | |
| `crates/victory-gpu/src/texback.rs:17` | by kind, not by texture: textures of those kinds the title never ! reads are copied back too (about a megabyte per such list). The copy shows ! what the list's last write left; a title read in between sees the frame ! before, as it would before the console's GPU has run. | TODO | |
| `crates/victory-gpu/src/texture.rs:13` | a change detiles the whole texture, not the pages written. ! Only textures read through descriptor tables are brought up to date ! (directly indexed views are not known per draw); plane 0 is uploaded ! (every mip and array slice), depth and stencil planes are not. | TODO | |
| `crates/victory-gpu/src/texture.rs:740` | a texture the host copied into keeps the host's contents even if the title later writes its memory too. | TODO | |
| `crates/victory-gpu/src/texture.rs:1057` | two lists that write the same new level, submitted in the other order than recorded, lose the first one's write under the zeros of the other; so does an upload recorded and not yet submitted. | TODO | |
| `crates/victory-gpu/src/texture.rs:1331` | read again at most every 30 frames (reading around unmapped pages is slow, and nothing tells us when pages arrive): a mip that streams in shows up to 30 frames late. | TODO | |
| `crates/victory-gpu/src/tiles.rs:41` | tiles are not taken back while the host copy lives. A level !   the title lets go of, or that turns out to be the next texture's, !   keeps its tiles; it is hidden by the views and written with zeros at !   the next upload. Its tiles go back when the host copy does. ! ! Only for a texture the title  | TODO | |
| `crates/victory-gpu/src/tiles.rs:460` | a second list recorded between the claim and this sends no zeros; if it is the first whose write gets tiles, what that write does not cover is what the tiles' last owner left. | TODO | |

## victory-hle (25; owner: codex)

| where | what the code says | class | plan |
|---|---|---|---|
| `crates/victory-hle/src/modules/apu.rs:163` | rate conversion (decode-convert, PCM convert) is not implemented; add a resampler if a title connects for it. | TODO | |
| `crates/victory-hle/src/modules/audio.rs:140` | the cached and non-cached sizes are not enforced as limits; enforce them if a title relies on running out. | TODO | |
| `crates/victory-hle/src/modules/audio.rs:309` | each enabled context decodes from a cold state and the overlap buffer is ignored (titles copy it between consecutive contexts of one voice); keep decoder state there if a title's seams turn out audible. | TODO | |
| `crates/victory-hle/src/modules/audio.rs:886` | context updates and the other SHAPE units' commands are acknowledged but not carried out; add them when a title sends them (each kind is logged once above). | TODO | |
| `crates/victory-hle/src/modules/display.rs:61` | HDR output is not done (see the presentation items in `TODO.md`); report enabled, with the host display's luminance, once the swap chain can present HDR. | TODO | |
| `crates/victory-hle/src/modules/display.rs:130` | screenshots fail as if capture were unavailable. Taking one needs a hook in the GPU side: the last presented frame read back as BGRA8 (size, row pitch, bytes) on request, e.g. `victory_gpu::last_frame() -> Option<Frame>`, copied from the back buffer after `PresentX`. | TODO | |
| `crates/victory-hle/src/modules/dsp.rs:15` | no convolution or block transform; outputs are zeros. Needs the coprocessor's impulse response and spectrum layouts to do better. | TODO | |
| `crates/victory-hle/src/modules/files.rs:545` | a read that goes on after the call (overlapped) is written into pages the kernel has locked by now: it lands all the same, but a refresh that guards its blocks before it does will not see it. | TODO | |
| `crates/victory-hle/src/modules/gameinput.rs:675` | with no device given only the reference's device is walked, not every device's readings in time order; the title walks only keyboard readings (one device) and the current pad's previous one. | TODO | |
| `crates/victory-hle/src/modules/gameinput.rs:751` | reading callbacks never run; the title registers none (it polls). | TODO | |
| `crates/victory-hle/src/modules/gameinput.rs:947` | trigger motors are dropped (XInput has none). The host API that drives them without a redistributable is Windows.Gaming.Input (WinRT `Gamepad.Vibration`), which needs WinRT bindings this crate lacks outside diagnostics builds; the title sends trigger levels in slot 0x50. | TODO | |
| `crates/victory-hle/src/modules/gameinput.rs:1231` | no characters (code point 0); the title reads scan codes and virtual keys only. | TODO | |
| `crates/victory-hle/src/modules/net.rs:514` | `[::]` becomes `[::1]`, so a dual-stack listener no longer hears 127.0.0.1; bind both families if a title relies on that. | TODO | |
| `crates/victory-hle/src/modules/net.rs:640` | local name spaces are refused too; pass those through if a title needs one. | TODO | |
| `crates/victory-hle/src/modules/speech.rs:4` | silent speech. The host's engine (SAPI through COM, or WinRT speech synthesis) needs COM bindings this crate has only in diagnostics builds, plus voice enumeration and SSML the title sends (slot 0xa8); the title then plays the empty stream and goes on. | TODO | |
| `crates/victory-hle/src/modules/xg.rs:54` | the rule of the library's older optimal-mode function (any size, 2D, 3D and cubes alike); `XGSuggestSwizzleMode` itself is newer than every copy we could check. Revisit if streamed textures come out scrambled. | TODO | |
| `crates/victory-hle/src/modules/xmem.rs:111` | committing part of such a reservation later would fail (the title's 23 calls never do); replace that part of the placeholder then. | TODO | |
| `crates/victory-hle/src/modules/xmem.rs:225` | every committed page reports graphics memory and no tag, since all of it is GPU-reachable here (the title's BVH builder, the only caller, wants graphics memory); remember the flags of `XMemVirtualAlloc` regions if a title tells CPU-only memory apart. | TODO | |
| `crates/victory-hle/src/modules/xmem.rs:339` | the host heap's addresses lie above 2^40, unlike the console's; nothing seen builds ray-tracing structures in this memory (the title uses its GPU heap). Sub-allocate from a reservation below 2^40 if one does. SAFETY: the layout has a non-zero size. | TODO | |
| `crates/victory-hle/src/modules/xmem.rs:1404` | `auto` is not the default. The intro's diner was there in time with it (one run, textures-d3), but in the dorm, where each call takes 0.3 ms of something that only one at a time gets (until a step of the mission, after which it takes 0.01), 16 threads map no more pages than one, and the groups hold  | TODO | |
| `crates/victory-hle/src/modules/xmem.rs:1919` | all fields are filled whatever the flags ask for, and use is not split by bandwidth class (GPU-optimal use reads 0); track allocations per class if a title budgets from that field. | TODO | |
| `crates/victory-hle/src/modules/xmem.rs:1944` | the figure is not yet the title's own memory alone (its heaps and the pages it mapped); count those if this line shows the title asking. | TODO | |
| `crates/victory-hle/src/modules/xpackage.rs:150` | every selector counts as installed, including ones the package's layout does not have (the console answers those with an invalid-selector error); the title asks only about its own chunks, which a full install has. | TODO | |
| `crates/victory-hle/src/modules/xpackage.rs:222` | accepted but nothing is removed (the package folder is the user's), so the chunks stay ready; the title's one call site (title+0x293b4b0, its chunk manager) would then see them still installed. | TODO | |
| `crates/victory-hle/src/modules/xpackage.rs:268` | the game kind (0) also lists nothing, not this installation; The title enumerates only content (kind 1, title+0x2937bf0). | TODO | |

## victory-loader (2; owner: coordinator)

| where | what the code says | class | plan |
|---|---|---|---|
| `crates/victory-loader/src/console_insn.rs:318` | a remembered region that was freed or protected since is written all the same for up to [`ANSWERS`] answers: the write then faults inside the handler, which the guard on view memory opens as it does any write, and which is a crash otherwise (as the unchecked single-line emulation behind this one alr | TODO | |
| `crates/victory-loader/src/tls.rs:85` | a byte match, not a decode. In the title all 53,246 hits are instruction starts and none is missed (checked against a full disassembly); decode .text if a title shows one. | TODO | |

## victory-shader (56; owner: codex)

| where | what the code says | class | plan |
|---|---|---|---|
| `crates/victory-shader/src/emit/lower.rs:405` | a genuine loop of more iterations than this is cut short too. | TODO | |
| `crates/victory-shader/src/emit/lower.rs:774` | `v_readfirstlane` of a value that differs between lanes is taken as it is (this host wave's first lane), and a guarded write of a plain value over a wave-dependent one is the only mix of old and new lanes that is looked at. | TODO | |
| `crates/victory-shader/src/emit/lower.rs:1082` | two writes in one region are taken not to hit the same dword from both halves. | TODO | |
| `crates/victory-shader/src/emit/lower.rs:1408` | masks only selects read (`v_cndmask_b32`, carries in) are not followed; such a program still stops with "a constant lane mask" or "an opaque lane mask". | TODO | |
| `crates/victory-shader/src/emit/lower.rs:1475` | where a count or sum goes is not checked. A program that uses a wave-wide result as data for the whole console wave (a tile's maximum, say, with one wave per thread group) gets its half's result. It is used only for programs that do not translate otherwise. | TODO | |
| `crates/victory-shader/src/emit/lower.rs:2155` | an index outside the table reads its last dword (the console would read whatever memory follows the code). | TODO | |
| `crates/victory-shader/src/emit/lower.rs:2298` | the field is taken not to cross a dword. | TODO | |
| `crates/victory-shader/src/emit/lower.rs:2983` | only the wave index (bits 24-27) is set; the console's other bits (lane counts) are read by no program. | TODO | |
| `crates/victory-shader/src/emit/lower.rs:3059` | 16-bit vertex inputs (the fetch routine's) are not unpacked; pixel inputs are (`interp`). | TODO | |
| `crates/victory-shader/src/emit/lower.rs:3159` | triangles only; on lines and points the console's i and j are not checked against the host's weights. | TODO | |
| `crates/victory-shader/src/emit/lower.rs:3506` | dword 1 holds the address bits only (a buffer's stride and swizzle bits read 0); the computed part of the offset is taken as whole descriptors (32-byte multiples), as `heap_index` does. | TODO | |
| `crates/victory-shader/src/emit/lower.rs:3675` | an access of several dwords is taken not to cross the end of a host copy (the builder's records lie on their size). | TODO | |
| `crates/victory-shader/src/emit/lower.rs:4479` | below 2^-12 turns the hardware scales by a slightly different constant than 2π (a last-place difference). | TODO | |
| `crates/victory-shader/src/emit/lower.rs:4611` | "some lane of the quad is set" is taken as true: the lanes it adds only feed derivatives, and exports keep their own masks. Wrong if such a section had a side effect. | TODO | |
| `crates/victory-shader/src/emit/lower.rs:5107` | no wave size for mesh shaders in SM 6.6, so host lanes cannot follow the console's; emulate through group-shared memory if mesh programs turn out to need these. | TODO | |
| `crates/victory-shader/src/emit/lower.rs:5181` | the counts are the declared maxima and each lane first culls its primitive (all indices 0, zero area), so primitives past the program's count are culled and vertices past it unused; this costs the host the unused vertex and primitive slots. Pass the program's own counts if that shows in frame time. | TODO | |
| `crates/victory-shader/src/emit/lower.rs:5206` | outputs past the cap even at the lane count are still emitted with the lower maxima (drivers take them); refuse them here if one hangs. Experiment (`VICTORY_MESH_OUTPUT_STRICT=1`): they are refused. | TODO | |
| `crates/victory-shader/src/emit/lower.rs:5556` | a source lane that is not at this instruction on the host (it left through a branch, not a guard) is read as the host gives it; the console reads 0. | TODO | |
| `crates/victory-shader/src/emit/lower.rs:5589` | LDS addresses are taken as dword-aligned and in bounds (out-of-bounds console accesses read 0 and drop writes). | TODO | |
| `crates/victory-shader/src/emit/lower.rs:5803` | f32 denormals compare as kept; with the console's MODE flushing them they would order as ±0 first (MODE is not modelled). | TODO | |
| `crates/victory-shader/src/emit/lower.rs:6118` | the float MODE register is not modelled; s_swappc_b64 is the vertex fetch call, whose results arrive as inputs. | TODO | |
| `crates/victory-shader/src/emit/lower.rs:6940` | the base level reads 0 (the host view starts at its own level 0, so sizes the code shifts by it come out right) and the other dwords (addresses, formats, tiling) have no host value: reading them stops the program. | TODO | |
| `crates/victory-shader/src/emit/lower.rs:7276` | an image whose kind nothing tells is taken for a 2D texture (the title's reads are texel sizes of 2D textures: puddle masks, shadow masks); a host view of another kind would answer its own sizes. | TODO | |
| `crates/victory-shader/src/emit/lower.rs:7466` | the descriptor's channel swizzle is taken as the identity (it is for raw views and the descriptors code builds), elements of 4 bytes or more as dword-aligned. | TODO | |
| `crates/victory-shader/src/emit/lower.rs:8849` | DXIL has no fused f32 multiply-add; FMad lets the driver fuse. | TODO | |
| `crates/victory-shader/src/emit/lower.rs:9052` | the hardware reciprocal is approximate (1 ULP); this divides. | TODO | |
| `crates/victory-shader/src/emit/lower.rs:9500` | `VICTORY_FIRSTLANE_OWN=1` takes the host wave's own first lane, exact when the value is the same in every lane of the console wave (the ray-tracing builder's scans read a sum of scalars this way, inside a loop whose exit is per console wave). | TODO | |
| `crates/victory-shader/src/emit/lower.rs:10013` | the host's conversion unless `VICTORY_F16_NEAREST=1` ([`f16_nearest_on`]), which costs a dozen instructions per result. | TODO | |
| `crates/victory-shader/src/emit/lower.rs:10060` | fused f16 multiply-adds as FMad in f32 (the product is exact; the sum rounds twice). | TODO | |
| `crates/victory-shader/src/emit/lower.rs:10308` | no constant operand seen; its high half is not known. | TODO | |
| `crates/victory-shader/src/emit/lower.rs:10371` | FMad in f32, as v_fma_f16. | TODO | |
| `crates/victory-shader/src/emit/lower.rs:11576` | stores are taken to write all four channels of a float format (the console converts by the view's format). One channel goes into a view the program also counts in. | TODO | |
| `crates/victory-shader/src/emit/lower.rs:11910` | such a sample reads level 0 (a bias: that level). The console takes derivatives from neighbouring lanes there; compute them from the lanes if a resolve shows the wrong mip. | TODO | |
| `crates/victory-shader/src/emit/lower.rs:11972` | no anisotropic filtering for these samples (the longer axis alone decides the level). | TODO | |
| `crates/victory-shader/src/emit/lower.rs:12069` | the level is rounded to a whole one and coordinates past the edge are clamped, whatever the sampler says. | TODO | |
| `crates/victory-shader/src/emit/lower.rs:12169` | d16 results are taken as floats (rounded to nearest even); integer formats would keep 16 bits. | TODO | |
| `crates/victory-shader/src/emit/lower.rs:12202` | the view is taken as a cube, not a cube array (whose face operand adds 8 × the slice). | TODO | |
| `crates/victory-shader/src/emit/lower.rs:12378` | the console's CBV descriptor is taken to have a 16-byte stride, as the vector loads of whole rows suggest. | TODO | |
| `crates/victory-shader/src/emit/lower.rs:12580` | that is known of the views a program picks from the heap, not of those in its tables; and the handle stays a float one, whose bits the drivers pass. | TODO | |
| `crates/victory-shader/src/emit/lower.rs:12610` | typed stores are taken to write all four channels of a float format. | TODO | |
| `crates/victory-shader/src/emit/lower.rs:12891` | without the misc vector (point size, render target or viewport index) pos1 and pos2 carry the clip then cull distances, in semantic order. | TODO | |
| `crates/victory-shader/src/emit/lower.rs:13205` | a pixel loop past 128 iterations, a vertex or mesh loop past 256 or a compute loop past 4,096 is cut short. (They were 1,024, 4,096 and 65,536: the opening cutscene then lost the card 14-25 frames in, in a minimap or shadow mesh draw; with every loop cut at 64 it ran 90 frames. Raise them again once | TODO | |
| `crates/victory-shader/src/emit/lower.rs:13288` | only with `VICTORY_STRICT_JOINS` set (the offline pack listing sets it to count them): 40 programs that translate today stop by name with it (vehicle paint pixel shaders at "the first half of an interpolation", foliage mesh shaders); until those are lowered they keep drawing with that zero. | TODO | |
| `crates/victory-shader/src/emit/mod.rs:783` | older containers fall back to the group's lane count, right when the HLSL group fills its waves (the title's do). | TODO | |
| `crates/victory-shader/src/emit/rect.rs:26` | that the console takes a rectangle's flat values from its ! first vertex follows from the hardware's provoking-vertex setting for ! triangles; it is not measured on a rectangle. | TODO | |
| `crates/victory-shader/src/emit/sampler.rs:79` | a custom border colour (type 3, a table entry) becomes transparent black; upgrade when a title uses one. | TODO | |
| `crates/victory-shader/src/emit/shader.rs:526` | not the bound heap's own count, which a title heap smaller than 2,048 would need. | TODO | |
| `crates/victory-shader/src/emit/shader.rs:782` | every output is declared to depend on every input; the exact sets need a dependence pass. Drivers read this state. | TODO | |
| `crates/victory-shader/src/io.rs:283` | writes that may keep the old v5 in active lanes (v_fmac_f32's accumulator, a 16-bit half or d16 load, v_writelane_b32, v_movreld_b32, DPP that skips lanes) and partial-EXEC writes pass; check those too if a mesh program writes v5 that way. | TODO | |
| `crates/victory-shader/src/resource.rs:981` | with a computed term, a high half that adds anything to the base's is taken as the carry of that term (offsets below 4 GB). | TODO | |
| `crates/victory-shader/src/resource.rs:1888` | what a lane holds after some passes is not traced. | TODO | |
| `crates/victory-shader/src/resource.rs:1970` | only the accesses are proven. Any other per-lane work in the loop is taken to run once for each lane, as the access does; a body whose effect adds up over the passes a lane waits through (a counter before the mask is narrowed) would differ. Compilers put nothing but the reads, the compares and the a | TODO | |
| `crates/victory-shader/src/resource.rs:2147` | what else the VGPRs held (numbers, on paths or in lanes that never read them as a descriptor) is ignored, and a move is taken to leave either its source or what a move before it wrote; a descriptor read per lane from memory (`global_load_*`) refuses. | TODO | |
| `crates/victory-shader/src/resource.rs:2395` | only the one-operand ways the compiler makes a constant that has no inline form; fold more if descriptors need it. | TODO | |
| `crates/victory-shader/src/resource.rs:2399` | a literal below 2^31, whose high dword is 0 whether the hardware extends it with zeros or with its sign. | TODO | |
| `crates/victory-shader/src/structure.rs:931` | the atom names the pair by its low dword, and lowering takes the high dword written beside it. For halves written apart (two `s_mov_b32`) lanes 32-63 are wrong unless lowering has no value for the low dword (EXEC, compares) and refuses. Give the atom both dwords when such code turns up. | TODO | |
