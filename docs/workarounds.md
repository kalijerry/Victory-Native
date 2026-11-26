# Workarounds and known defects

Victory AMD shows the title's own picture. Anything that departs from it, whether a stand-in, a
setting that shifts timing, or a defect not fixed yet, is listed here with its cause and the way
out. A workaround is a named switch and is logged when it acts. Nothing that changes the picture
is on by default unless it is measured identical.

## Stand-ins (not used by default)

| switch | what it does | why it is not a fix | status |
|---|---|---|---|
| `VICTORY_AMBIENT=<share>` / `constant:<level>` | puts an estimate of the indirect light where the title's ray-traced light is missing (`ambient.rs`) | it is light the title does not draw | removed from `scripts/play.ps1` defaults; with ray tracing on (`VICTORY_BVH_BUILDER=1`) it does not act anyway |
| `VICTORY_AMBIENT_SETTLE` | how fast that estimate settles | only acts with the stand-in | follows `VICTORY_AMBIENT` |

Without ray tracing, rooms lit only by bounced light are dark. That is what the title draws when
its rays find nothing. The fix is the ray tracing (`VICTORY_BVH_BUILDER=1`), not added light.

## Settings that shift timing (on in `scripts/play.ps1`, under review)

| switch | effect on the picture | cause it works around | way out |
|---|---|---|---|
| `VICTORY_TEXTURE_NO_WAIT=1` | a texture that arrives while the runtime remaps may show one frame late | the texture lock is contended by the runtime's remapping | measure the picture with and without; fix the lock contention |
| `VICTORY_SHOW_AFTER=1` | a new texture shows once the submission with its upload has finished | lists on the card read a texture while it is being made (page faults, device loss n08/n11/n15) | make the texture before its first use, so that nothing has to wait for it |

## Known defects (open)

| defect | where seen | what is known |
|---|---|---|
| trees and palms flicker at the start of shots, then settle | boat cutscene, mangroves | not stale descriptors (port 2 misses down to 6 a frame); measured with `flicker.py` |
| lamp-lit room lighting flickers | prison talk scene, interiors | cause open |
| shallow water shows regular dark stripes with stepped edges, cut by a straight line | boat ride after the cutscene | likely a texture read with a wrong mip, format or tiling; open |
| models and textures stay at low detail for long | intro | the title paces streaming by its own frames: about 3x slower at 10 fps than at 30 |
| ray-traced structures of moving objects partly corrupt | everywhere with ray tracing | ~450 stray pointers a frame followed by the rays (upstream finding) |

## Inherited behaviours that invent a result (on by default; to replace)

Found in the upstream code at 2de9980. Each one breaks "translate, don't fake" and is to be
replaced by a real implementation. Until then it stays listed here, and the log must say when
it acts.

| where | what it does today | why it is not faithful | the real implementation |
|---|---|---|---|
| `host.rs` frame line, `draw.rs` "a dispatch without a host pipeline dropped" | a draw or dispatch whose host pipeline is not made yet is dropped (counted per frame in the log) | the frame is drawn without work the title asked for: missing objects, wrong passes | have every pipeline before its first use: `victory warm` ahead, and when one is missing, wait for it within a budget (upstream #303) and log the wait |
| `list.rs` `ValueWait` | a wait on a value with an unknown comparison code, or on memory that is not mapped, counts as met | a wait the console would hold is let through: work runs before its input exists | decode every comparison code the console has; treat unmapped memory as an error and log it |
| `draw.rs` root views | a root view has no size, so constant buffers are taken as 64 KiB and other views as the rest of their buffer | an assumption about how far a shader reads, not what the console does | the console reads through an address with no bound: derive the reach from the shader's accesses, or map the title's memory in place (renderer port 1) |
| `descriptor/host.rs` `VICTORY_TEXTURE_AWAY_MS` | a texture whose memory is no longer mapped becomes a null view (reads zeros) | an unmapped texture on the console is a title bug or a streaming race, not black texels | log each case with the texture and the title's mapping calls; keep the last mapped contents only if the console's memory system would |
| `ambient.rs` (`VICTORY_AMBIENT*`) | stand-in indirect light; off by default here | light the title does not draw | ray tracing (`VICTORY_BVH_BUILDER=1`) |

Beyond these, the code marks 167 places of its own as `shortcut:` (narrower semantics than the
console). They are listed one by one in [shortcuts-audit.md](shortcuts-audit.md) to be classified
as FAKE, APPROXIMATION or OK and replaced in turn.
