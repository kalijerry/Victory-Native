//! Renderer port #5: the title's constant buffers read by the card where the
//! title has them (`VICTORY_PORT_CBV=1`, off by default; read in [`on`]).
//!
//! What the console allows: a constant buffer anywhere on 16 bytes. What
//! D3D12 asks: a constant buffer view on 256. So the layer made every
//! constant buffer a draw's tables name afresh at each submission: a copy of
//! its bytes (out of title memory, on the CPU) into upload memory of its own
//! on 256, and a view of that ([`crate::draw`]'s tables, `defer_constants`):
//! ~23,000 a frame of the city, ~9 MB, and as many steps of the submitting
//! thread. With renderer port 1 (`VICTORY_PORT_CONSTANTS=1`) the memory they
//! are copied out of is the card's to read in place already.
//!
//! What is done here instead: shaders are translated to read constant buffers
//! as raw bytes (`victory_shader::emit::constants_raw`): a table's through
//! the raw twin of its slot, a root one as a root SRV. A raw view starts on 16
//! bytes, a root SRV on 4. So a constant buffer of a table in title memory in
//! place is a raw view of the title's bytes, made in the twin of its slot,
//! and nothing is copied or done at submission; a root one is bound at its
//! address as root SRVs are. Constant buffers anywhere else are copied as
//! before (the twin of their slot is a raw view of the copy, as it was),
//! unless `VICTORY_PORT_CBV_COPIES=1` is also set (off by default): then a
//! table's constant buffer in any host copy is viewed through its raw twin
//! too, with a normal [`crate::shadow::Shadow::refresh`] for CPU writes,
//! avoiding `Step::Fill` scratch copies, `card_steps` bounce copies and
//! waits for flights that wrote the buffer on the card.
//! The twin of a title slot that holds a constant buffer view is a raw view
//! of its bytes too (it was a constant buffer view: a raw read through one
//! on AMD has no end).
//!
//! What it assumes: what the card reads of a constant buffer at a draw is
//! what the title's CPU wrote before it submitted (or what earlier GPU work
//! wrote into that host copy in queue order, with `VICTORY_PORT_CBV_COPIES=1`),
//! as for every other title buffer in place (renderer port 1) or host copy:
//! before, the copy at submission was taken then whatever the title wrote
//! after. The title writes its constants into rings it waits on its fences for.

use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

/// Whether renderer port #5 is on: the shaders' side
/// (`victory_shader::emit::constants_raw`) and the host's views follow the
/// same switch.
pub(crate) fn on() -> bool {
    victory_shader::emit::constants_raw()
}

/// Whether a table's constant buffer in title memory in place is given to the
/// card where it is (on with the port; `VICTORY_PORT_CBV_PLACE=0`: copied as
/// before, and read through the raw twin all the same). With
/// `VICTORY_AB=<s>,VICTORY_PORT_CBV_PLACE` (diagnostics builds) on and off
/// while the title runs: both sides measured in one run.
pub(crate) fn in_place_on() -> bool {
    on() && crate::toggled!("VICTORY_PORT_CBV_PLACE")
}

/// Whether a table's constant buffer in a host copy (not in place) is viewed
/// directly through its raw twin instead of copied into scratch upload memory
/// (`VICTORY_PORT_CBV_COPIES=1`, off by default; requires `VICTORY_PORT_CBV=1`).
/// With `VICTORY_AB=<s>,VICTORY_PORT_CBV_COPIES` (diagnostics builds) on and
/// off while the title runs: both sides measured in one run.
pub(crate) fn copies_on() -> bool {
    if !on() {
        return false;
    }
    #[cfg(feature = "diagnostics")]
    if let Some(pick) = crate::ab::pick("VICTORY_PORT_CBV_COPIES") {
        return pick;
    }
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| copies_value(std::env::var("VICTORY_PORT_CBV_COPIES").ok().as_deref()))
}

fn copies_value(value: Option<&str>) -> bool {
    value.is_some_and(|v| v.trim() == "1")
}

/// Whether `VICTORY_PORT_CBV_COPIES` is on or toggled in an A/B run.
fn copies_named() -> bool {
    #[cfg(feature = "diagnostics")]
    if crate::ab::bit("VICTORY_PORT_CBV_COPIES").is_some() {
        return true;
    }
    copies_on()
}

/// How a table's constant buffer is bound under renderer port 5.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TableUse {
    /// View title memory in place directly via its raw twin (no refresh).
    InPlace,
    /// View a host copy directly via its raw twin and defer a normal refresh.
    HostCopy,
    /// Copy into scratch upload memory (`Step::Fill`) as before.
    Copy,
}

/// Chooses how to bind a table's constant buffer given the switch states and
/// whether a host copy exists and is in place.
pub(crate) fn table_use(in_place_enabled: bool, copies_enabled: bool, has_shadow: bool, shadow_in_place: bool) -> TableUse {
    if !has_shadow {
        return TableUse::Copy;
    }
    if shadow_in_place && in_place_enabled {
        TableUse::InPlace
    } else if !shadow_in_place && copies_enabled {
        TableUse::HostCopy
    } else {
        TableUse::Copy
    }
}

/// Since the last line: tables' constant buffers given in place, their bytes;
/// copied as before because not in place (or no copy), because not on 16 bytes;
/// title slots' constant buffers whose twin is a raw view; tables' constant
/// buffers viewed directly in host copies, their bytes.
static STATS: [AtomicU64; 7] = [const { AtomicU64::new(0) }; 7];

/// A table's constant buffer of `size` bytes was given in place.
pub(crate) fn in_place(size: u32) {
    STATS[0].fetch_add(1, Relaxed);
    STATS[1].fetch_add(u64::from(size), Relaxed);
}

/// A table's constant buffer of `size` bytes was viewed directly in its host copy.
pub(crate) fn in_copy(size: u32) {
    STATS[5].fetch_add(1, Relaxed);
    STATS[6].fetch_add(u64::from(size), Relaxed);
}

/// A table's constant buffer was copied as before: `aligned` false when its
/// buffer could not be viewed on 16 bytes.
pub(crate) fn copied(aligned: bool) {
    STATS[if aligned { 2 } else { 3 }].fetch_add(1, Relaxed);
}

/// A title slot's constant buffer view got a raw twin.
pub(crate) fn twinned() {
    STATS[4].fetch_add(1, Relaxed);
}

/// Whether a constant buffer at host address `gpu` can be a raw view of
/// those bytes: on 16 bytes.
pub(crate) fn raw_ok(gpu: u64) -> bool {
    gpu.is_multiple_of(16)
}

/// For the frame-rate line.
pub(crate) fn log(frames: f64) {
    if !on() {
        return;
    }
    let s: Vec<f64> = STATS.iter().map(|s| s.swap(0, Relaxed) as f64 / frames.max(1e-9)).collect();
    if s[5] > 0.0 || copies_named() {
        tracing::info!(
            "renderer port 5 (VICTORY_PORT_CBV=1, VICTORY_PORT_CBV_COPIES=1) per frame: {:.0} tables' constant buffers read in place ({:.1} KB), \
             {:.0} in host copies ({:.1} KB, refreshed, no fill step); copied as before: {:.0} without copy, {:.0} not on 16 bytes; \
             {:.0} title slots' constant buffers with a raw twin",
            s[0],
            s[1] / 1024.0,
            s[5],
            s[6] / 1024.0,
            s[2],
            s[3],
            s[4],
        );
    } else {
        tracing::info!(
            "renderer port 5 (VICTORY_PORT_CBV=1) per frame: {:.0} tables' constant buffers read by the card in place ({:.1} KB not copied, \
             no step at submission); copied as before: {:.0} not in place, {:.0} not on 16 bytes; {:.0} title slots' constant buffers with a raw twin",
            s[0],
            s[1] / 1024.0,
            s[2],
            s[3],
            s[4],
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_views_need_sixteen_bytes() {
        assert!(raw_ok(0x3500_0010));
        assert!(raw_ok(0x3500_0100));
        assert!(!raw_ok(0x3500_0008));
        assert!(!raw_ok(0x3500_0004));
    }

    #[test]
    fn copies_switch_is_off_by_default_and_on_only_for_one() {
        assert!(!copies_value(None));
        assert!(!copies_value(Some("")));
        assert!(!copies_value(Some("0")));
        assert!(!copies_value(Some("2")));
        assert!(!copies_value(Some("true")));
        assert!(copies_value(Some("1")));
        assert!(copies_value(Some(" 1 ")));
    }

    #[test]
    fn table_use_selects_in_place_host_copy_or_fallback() {
        for in_place_enabled in [false, true] {
            for copies_enabled in [false, true] {
                for shadow_in_place in [false, true] {
                    assert_eq!(
                        table_use(in_place_enabled, copies_enabled, false, shadow_in_place),
                        TableUse::Copy,
                        "no shadow always falls back to copy"
                    );
                }
            }
        }
        assert_eq!(table_use(true, false, true, true), TableUse::InPlace);
        assert_eq!(table_use(true, true, true, true), TableUse::InPlace);
        assert_eq!(table_use(false, true, true, true), TableUse::Copy);
        assert_eq!(table_use(false, false, true, true), TableUse::Copy);

        assert_eq!(table_use(true, false, true, false), TableUse::Copy);
        assert_eq!(table_use(true, true, true, false), TableUse::HostCopy);
        assert_eq!(table_use(false, true, true, false), TableUse::HostCopy);
        assert_eq!(table_use(false, false, true, false), TableUse::Copy);
    }
}
