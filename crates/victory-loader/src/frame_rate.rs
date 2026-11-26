//! The title's frame rate (`VICTORY_FRAME_INTERVAL_MS`, off without it): the
//! rate Grand Theft Auto VI asks for is a setting of its own, and two
//! places in its code are changed at load to read another.
//!
//! What the title does (game_scarlett_master.exe 1.0.2.1, image offsets):
//!
//! - Its graphics settings hold a frame rate (+0x210 of the settings object,
//!   30 on Series X). Two functions that apply the settings, +0x1e0fb30 and
//!   +0x22434a0, turn it into a frame time: `frame_us = 1e6 /
//!   clamp(rate)`, 0 and 1000 or more being 1000 (the code's "no limit"),
//!   stored at +0x98f84ac.
//! - +0x247ab10 sets the frame interval from it: the display's rate is 60,
//!   or 120 if its mode list (the output's `GetDisplayModeList`) has a
//!   120 Hz mode (+0x2481ca0); the divisor is `min(display / rate, 4)`, or
//!   1 for a rate above the display's; the interval is a table at
//!   +0x61ece60 (`[16667, 16667, 33333, 50000, 66667]` at 60 Hz, `[8333,
//!   8333, 16667, 25000, 33333]` at 120 Hz) and goes to
//!   `SetFrameIntervalX` (device +0x270), with `ScheduleFrameEventX`
//!   (+0x278) after it. At 30 and 60 Hz: 33,333 us, the flat 33.3 ms of the
//!   menus and videos.
//! - +0x120530, once a flag (+0x7453ec9, set by +0x21b77c0) is up, is a
//!   limiter of its own on `QueryPerformanceCounter`: it waits until
//!   `1000 / trunc(1e6 / frame_us)` ms have passed since the last frame.
//! - +0x3eaf5d0 hands `1e6 / frame_us` to scripts, and +0x234490 scales
//!   per-frame amounts by it (1.0 below 60 except 0.7 at 40, 0.5 from 60).
//!
//! - The simulation: +0x22264d0 hands each of its timers the real time
//!   since the last frame (`QueryPerformanceCounter`); +0x2226040 makes the
//!   step of it, at most 1/15 s and at least `1 / rate` (+0x6849844, made
//!   at +0x22435de and +0x2241190 from +0x2241fd0, which reads the same
//!   setting). So at 30 every frame shorter than 33.3 ms still moves game
//!   time 33.3 ms: a fixed 30 Hz step for fast frames, delta time for slow
//!   ones.
//!
//! So reporting a 120 Hz display alone changes nothing at 30 (divisor 4,
//! 33,333 us). Here the two loads of the setting (`mov eax, [reg+210h]`)
//! become `mov eax, rate ; nop`, so the title's frame time, interval and
//! own limiter follow the rate asked for, and +0x2241fd0 returns the rate
//! too (1000 for no limit), so the shortest step follows; the setting
//! itself is left as it is. Each site's bytes, from the load to the store of the frame time, are
//! compared before anything is written; a site whose bytes differ is left
//! alone and said so (another build of the title).

/// The switch: the interval in milliseconds our frame clock ticks at
/// (`victory-gpu`'s `frame`), 0 for no wait at all. `None` if unset or
/// malformed (said why).
pub fn asked_interval_ms() -> Option<f64> {
    let spec = std::env::var("VICTORY_FRAME_INTERVAL_MS").ok()?;
    let ms = parse_interval_ms(&spec);
    if ms.is_none() {
        eprintln!("VIctory: VICTORY_FRAME_INTERVAL_MS={spec}: expected milliseconds from 0 (no wait) to 1000");
    }
    ms
}

/// `16.667` â†’ 16.667; 0 is allowed (no wait), more than a second is not.
pub fn parse_interval_ms(spec: &str) -> Option<f64> {
    spec.trim().parse::<f64>().ok().filter(|ms| ms.is_finite() && (0.0..=1000.0).contains(ms))
}

/// The frame rate the title is given for an interval: `1000 / ms` rounded
/// (16.667 â†’ 60, 8.333 â†’ 120, 33.333 â†’ 30), 0 for no wait (the title's own
/// "no limit"). `VICTORY_TITLE_FPS=<rate>` gives it outright, `off` leaves the
/// title as it is (only our clock changes).
pub fn title_rate(interval_ms: f64, title_fps: Option<&str>) -> Option<u32> {
    match title_fps.map(str::trim) {
        Some("off") => None,
        Some(rate) => rate.parse::<u32>().ok().filter(|r| *r <= 1000),
        None if interval_ms <= 0.0 => Some(0),
        None => Some(((1000.0 / interval_ms).round() as u32).clamp(1, 1000)),
    }
}

/// The frame time the title derives from a rate, in microseconds, as its
/// code computes it (single-precision division, truncated).
pub fn title_frame_us(rate: u32) -> u32 {
    let clamped = if rate == 0 || rate >= 1000 { 1000 } else { rate };
    (1_000_000.0_f32 / clamped as f32) as u32
}

/// The interval the title then asks for (`SetFrameIntervalX`), in
/// microseconds, on a display of 60 Hz (or 120 Hz).
pub fn title_interval_us(rate: u32, display_120: bool) -> u32 {
    const TABLE: [[u32; 5]; 2] = [[16667, 16667, 33333, 50000, 66667], [8333, 8333, 16667, 25000, 33333]];
    let fps = (1_000_000.0_f32 / title_frame_us(rate) as f32) as u32 & 0xffff;
    let display = if display_120 { 120 } else { 60 };
    let index = if fps <= display { (display / fps.max(1)).min(4) } else { 1 };
    TABLE[usize::from(display_120)][index as usize]
}

/// A read of the setting to change: its offset in the image, what it is,
/// and the bytes there (compared first; the replacement goes at the start).
pub struct Site {
    pub at: usize,
    pub kind: Kind,
    pub bytes: &'static [u8],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `mov eax, [reg+210h]` before the frame time is made of it: becomes
    /// `mov eax, rate ; nop` (6 bytes).
    Load,
    /// +0x2241fd0, the rate the timers' shortest step is made of (`1 /
    /// rate`, +0x6849844): `cmp dword [rax+210h], 0` becomes `mov eax, rate
    /// ; jmp` to its `vcvtsi2ss` (7 bytes). Left as it was, the timers step at
    /// least 1/30 s however short the frame: at 60 frames a second the game
    /// ran at twice its speed (run fps-1, menus). For no limit, 1000.
    SimRate,
}

/// The three reads (game_scarlett_master.exe 1.0.2.1, image offsets).
pub const SITES: [Site; 3] = [
    // +0x1e0fb30: mov eax, [rdi+210h] ... mov [+0x98f84ac], eax
    Site {
        at: 0x1e0fc62,
        kind: Kind::Load,
        bytes: &[
            0x8b, 0x87, 0x10, 0x02, 0x00, 0x00, 0xba, 0xe8, 0x03, 0x00, 0x00, 0xc5, 0xfa, 0x10, 0x0d, 0x9f, 0xa1, 0x34, 0x04, 0xb9, 0xe8,
            0x03, 0x00, 0x00, 0x3d, 0xe8, 0x03, 0x00, 0x00, 0x0f, 0x42, 0xd0, 0x85, 0xc0, 0x0f, 0x44, 0xd1, 0xc5, 0xea, 0x2a, 0xc2, 0xc5,
            0xf2, 0x5e, 0xc0, 0xc4, 0xe1, 0xfa, 0x2c, 0xc0, 0x89, 0x05, 0x12, 0x88, 0xae, 0x07,
        ],
    },
    // +0x22434a0: mov eax, [rsi+210h] ... mov [+0x98f84ac], eax
    Site {
        at: 0x224359a,
        kind: Kind::Load,
        bytes: &[
            0x8b, 0x86, 0x10, 0x02, 0x00, 0x00, 0xba, 0xe8, 0x03, 0x00, 0x00, 0xc5, 0xfa, 0x10, 0x0d, 0x67, 0x68, 0xf1, 0x03, 0xb9, 0xe8,
            0x03, 0x00, 0x00, 0x3d, 0xe8, 0x03, 0x00, 0x00, 0x0f, 0x42, 0xd0, 0x85, 0xc0, 0x0f, 0x44, 0xd1, 0xb1, 0xff, 0xc5, 0xea, 0x2a,
            0xc2, 0xc5, 0xf2, 0x5e, 0xc0, 0xc4, 0xe1, 0xfa, 0x2c, 0xc0, 0x89, 0x05, 0xd8, 0x4e, 0x6b, 0x07,
        ],
    },
    // +0x2241fd0: cmp dword [rax+210h], 0 ; je ; call ; mov eax, [rax+210h] ; vcvtsi2ss xmm0, xmm0, rax
    Site {
        at: 0x2241fdb,
        kind: Kind::SimRate,
        bytes: &[
            0x83, 0xb8, 0x10, 0x02, 0x00, 0x00, 0x00, 0x74, 0x12, 0xe8, 0xc7, 0xa1, 0xe0, 0x02, 0x8b, 0x80, 0x10, 0x02, 0x00, 0x00, 0xc4,
            0xe1, 0xfa, 0x2a, 0xc0,
        ],
    },
];

/// Title data worth seeing while it runs (`VICTORY_TIMER_LOG=1`): the frame
/// time (+0x98f84ac) and the limiter's flag (+0x7453ec9).
pub const FRAME_US: usize = 0x98f84ac;
pub const LIMITER_ON: usize = 0x7453ec9;
/// The title's timers (each set up at +0x22261a9 with a step of 0.05 s and
/// 50 ms): +0 and +4 times, +0xc the step in seconds, +0x10 in milliseconds.
pub const TIMERS: [usize; 7] = [0xa943788, 0x856c6b4, 0xabbef40, 0xa958358, 0xa958380, 0x74fa558, 0xa943768];

/// What goes at a site of `kind` for `rate`.
pub fn replacement(kind: Kind, rate: u32) -> Vec<u8> {
    match kind {
        // mov eax, rate ; nop
        Kind::Load => {
            let r = rate.to_le_bytes();
            vec![0xb8, r[0], r[1], r[2], r[3], 0x90]
        }
        // mov eax, rate ; jmp +13 (to the vcvtsi2ss, 20 bytes on)
        Kind::SimRate => {
            let r = (if rate == 0 { 1000 } else { rate }).to_le_bytes();
            vec![0xb8, r[0], r[1], r[2], r[3], 0xeb, 0x0d]
        }
    }
}

/// What became of a site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Patched {
    Done,
    /// Its bytes are not the ones expected: left alone.
    Differs,
    Failed(&'static str),
}

/// Makes each site of `sites` in the image of `size` bytes at `base` load
/// `rate`. What became of each, said in the log too.
///
/// # Safety
/// The image stays mapped and no thread runs its code yet.
#[cfg(windows)]
pub unsafe fn apply(base: usize, size: usize, rate: u32, sites: &[Site]) -> Vec<Patched> {
    sites
        .iter()
        .map(|site| {
            let code = replacement(site.kind, rate);
            // SAFETY: as this function's.
            let done = unsafe { write(base, size, site.at, site.bytes, &code) };
            let offset = format_args!("{:#x}", site.at);
            match done {
                Patched::Done => tracing::info!(site = %offset, kind = ?site.kind, rate, "frame rate: the title's read of its frame rate setting patched (VICTORY_FRAME_INTERVAL_MS)"),
                Patched::Differs => tracing::warn!(site = %offset, "frame rate: not patched, the title's bytes there are not the ones expected (another build?)"),
                Patched::Failed(why) => tracing::warn!(site = %offset, why, "frame rate: not patched"),
            }
            done
        })
        .collect()
}

/// Puts `code` at `at` in the image of `size` bytes at `base` if the bytes
/// there are `bytes` (`code` no longer than them).
///
/// # Safety
/// The image stays mapped and no thread runs its code yet.
#[cfg(windows)]
unsafe fn write(base: usize, size: usize, at: usize, bytes: &[u8], code: &[u8]) -> Patched {
    use windows_sys::Win32::System::Diagnostics::Debug::FlushInstructionCache;
    use windows_sys::Win32::System::Memory::{PAGE_EXECUTE_READWRITE, VirtualProtect};
    use windows_sys::Win32::System::Threading::GetCurrentProcess;
    let len = code.len();
    if bytes.len() < len || at.checked_add(bytes.len()).is_none_or(|end| end > size) {
        return Patched::Failed("outside the image");
    }
    let at = base + at;
    // SAFETY: inside the mapped image (checked above).
    let there = unsafe { std::slice::from_raw_parts(at as *const u8, bytes.len()) };
    if there != bytes {
        return Patched::Differs;
    }
    let mut old = 0;
    // SAFETY: the site's bytes, made writable for this and put back;
    // nothing runs them yet.
    unsafe {
        if VirtualProtect(at as *const _, len, PAGE_EXECUTE_READWRITE, &mut old) == 0 {
            return Patched::Failed("Windows would not make it writable");
        }
        std::ptr::copy_nonoverlapping(code.as_ptr(), at as *mut u8, len);
        VirtualProtect(at as *const _, len, old, &mut old);
        FlushInstructionCache(GetCurrentProcess(), at as *const _, len);
    }
    Patched::Done
}

// The simulation's longest step (`VICTORY_LONGEST_STEP=<seconds>`, off
// without it).
//
// +0x2226040 makes each timer's step of the real time since the last frame:
// at most 1/15 s (+0x6159ec0), at least `1 / rate`. Below 15 frames a second
// the title's time therefore runs slower than the clock on the wall: at 7
// frames a second (143 ms) it moves 66.7 ms a frame, less than half. Sound
// plays on at its own pace, and cutscenes, kept to it, go back and forth
// (seen as the picture jumping back a second, then forward). The console
// never runs that slowly; a port that does runs its time at the wall's pace
// up to a longer step instead.
//
// The step's ceiling is a load of that constant, shared with 64 other
// reads, so it is not changed: the load (`vmovss xmm3, [rip+d]`, 8 bytes)
// reads another constant of the title's read-only data, one of
// [`STEPS`], whose value is compared first like the load's bytes.

/// The load of the ceiling in +0x2226040 and its bytes.
pub const STEP_LOAD_AT: usize = 0x2226045;
pub const STEP_LOAD: [u8; 8] = [0xc5, 0xfa, 0x10, 0x1d, 0x73, 0x3e, 0xf3, 0x03];
/// Constants of the title's read-only data the load can read instead: the
/// seconds and where they are (image offsets). The first is its own.
pub const STEPS: [(f32, usize); 5] = [
    (1.0 / 15.0, 0x6159ec0),
    (0.1, 0x5fc7020),
    (0.125, 0x615a6b8),
    (0.2, 0x6159f8c),
    (0.25, 0x6159ca4),
];

/// The switch: the longest step asked for, one of [`STEPS`] (`None` if
/// unset, or not one of them: said why).
pub fn asked_longest_step() -> Option<(f32, usize)> {
    let spec = std::env::var("VICTORY_LONGEST_STEP").ok()?;
    let step = longest_step(&spec);
    if step.is_none() {
        eprintln!(
            "VIctory: VICTORY_LONGEST_STEP={spec}: expected one of 0.1, 0.125, 0.2, 0.25 (seconds)"
        );
    }
    step
}

/// `0.25` → the constant of 0.25 s; only the title's own constants.
pub fn longest_step(spec: &str) -> Option<(f32, usize)> {
    let secs = spec.trim().parse::<f32>().ok()?;
    STEPS.iter().copied().find(|(s, _)| (s - secs).abs() < 1e-6)
}

/// The load of the ceiling made to read the constant at `target`.
pub fn step_load(target: usize) -> [u8; 8] {
    let disp = (target as i64 - (STEP_LOAD_AT + STEP_LOAD.len()) as i64) as i32;
    let d = disp.to_le_bytes();
    [
        STEP_LOAD[0],
        STEP_LOAD[1],
        STEP_LOAD[2],
        STEP_LOAD[3],
        d[0],
        d[1],
        d[2],
        d[3],
    ]
}

/// Makes the simulation's step at most `step` seconds: the load of its
/// ceiling ([`STEP_LOAD`] at [`STEP_LOAD_AT`]) reads the constant at
/// `target`, once it holds `step`. Said in the log.
///
/// # Safety
/// The image stays mapped and no thread runs its code yet.
#[cfg(windows)]
pub unsafe fn apply_longest_step(base: usize, size: usize, step: (f32, usize)) -> Patched {
    // SAFETY: as this function's.
    unsafe { apply_longest_step_at(base, size, STEP_LOAD_AT, &STEP_LOAD, step) }
}

/// [`apply_longest_step`] for a load at `at` whose bytes are `bytes`.
///
/// # Safety
/// As [`apply_longest_step`]'s.
#[cfg(windows)]
unsafe fn apply_longest_step_at(
    base: usize,
    size: usize,
    at: usize,
    bytes: &[u8; 8],
    (step, target): (f32, usize),
) -> Patched {
    let holds = target.checked_add(4).is_some_and(|end| end <= size)
        // SAFETY: inside the mapped image (checked).
        && unsafe { (base as *const u8).add(target).cast::<f32>().read_unaligned() } == step;
    let disp = (target as i64 - (at + bytes.len()) as i64) as i32;
    let mut code = *bytes;
    code[4..].copy_from_slice(&disp.to_le_bytes());
    let done = if holds {
        // SAFETY: as this function's.
        unsafe { write(base, size, at, bytes, &code) }
    } else {
        Patched::Differs
    };
    match done {
        Patched::Done => tracing::info!(
            step,
            "simulation: the longest step is {step} s (VICTORY_LONGEST_STEP; the title's own is 1/15 s)"
        ),
        Patched::Differs => tracing::warn!(
            "simulation: the longest step not changed, the title's bytes are not the ones expected (another build?)"
        ),
        Patched::Failed(why) => tracing::warn!(why, "simulation: the longest step not changed"),
    }
    done
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intervals_and_rates() {
        assert_eq!(parse_interval_ms("16.667"), Some(16.667));
        assert_eq!(parse_interval_ms(" 0 "), Some(0.0));
        assert_eq!(parse_interval_ms("-1"), None);
        assert_eq!(parse_interval_ms("fast"), None);
        assert_eq!(parse_interval_ms("1001"), None);
        assert_eq!(title_rate(16.667, None), Some(60));
        assert_eq!(title_rate(8.333, None), Some(120));
        assert_eq!(title_rate(33.333, None), Some(30));
        assert_eq!(title_rate(0.0, None), Some(0));
        assert_eq!(title_rate(0.5, None), Some(1000));
        assert_eq!(title_rate(16.667, Some("off")), None);
        assert_eq!(title_rate(0.0, Some("90")), Some(90));
        assert_eq!(title_rate(0.0, Some("lots")), None);
    }

    /// The title's own arithmetic: its 30 is the 33,333 us the log shows,
    /// on a 60 Hz display and on a 120 Hz one.
    #[test]
    fn the_titles_intervals() {
        assert_eq!(title_frame_us(30), 33333);
        assert_eq!(title_frame_us(60), 16666);
        assert_eq!(title_frame_us(0), 1000);
        assert_eq!(title_frame_us(5000), 1000);
        assert_eq!(title_interval_us(30, false), 33333);
        assert_eq!(title_interval_us(30, true), 33333);
        assert_eq!(title_interval_us(40, true), 25000);
        assert_eq!(title_interval_us(60, false), 16667);
        assert_eq!(title_interval_us(60, true), 16667);
        assert_eq!(title_interval_us(120, true), 8333);
        assert_eq!(title_interval_us(120, false), 16667);
        assert_eq!(title_interval_us(0, false), 16667);
        assert_eq!(title_interval_us(0, true), 8333);
        assert_eq!(title_interval_us(20, false), 50000);
    }


    #[test]
    fn the_sites_are_reads_of_the_setting() {
        for site in SITES.iter().filter(|s| s.kind == Kind::Load) {
            // mov eax, [rdi/rsi+210h]
            assert_eq!(site.bytes[0], 0x8b);
            assert!(matches!(site.bytes[1], 0x86 | 0x87));
            assert_eq!(&site.bytes[2..6], &[0x10, 0x02, 0x00, 0x00]);
            // mov edx, 1000 follows; the window ends with the store of eax.
            assert_eq!(&site.bytes[6..11], &[0xba, 0xe8, 0x03, 0x00, 0x00]);
            assert_eq!(&site.bytes[site.bytes.len() - 6..site.bytes.len() - 4], &[0x89, 0x05]);
            // RIP-relative: the store is of +0x98f84ac.
            let disp = i32::from_le_bytes(site.bytes[site.bytes.len() - 4..].try_into().unwrap());
            let end = site.at + site.bytes.len();
            assert_eq!(end as i64 + i64::from(disp), FRAME_US as i64);
        }
        let sim = SITES.iter().find(|s| s.kind == Kind::SimRate).unwrap();
        // cmp dword [rax+210h], 0 ; ... ; mov eax, [rax+210h] at 14 ; vcvtsi2ss at 20
        assert_eq!(&sim.bytes[..7], &[0x83, 0xb8, 0x10, 0x02, 0x00, 0x00, 0x00]);
        assert_eq!(&sim.bytes[14..20], &[0x8b, 0x80, 0x10, 0x02, 0x00, 0x00]);
        assert_eq!(&sim.bytes[20..], &[0xc4, 0xe1, 0xfa, 0x2a, 0xc0]);
        let code = replacement(Kind::SimRate, 60);
        assert_eq!(code.len() + usize::from(code[6]), 20, "the jump lands on the vcvtsi2ss");
        assert_eq!(replacement(Kind::Load, 60), [0xb8, 60, 0, 0, 0, 0x90]);
        assert_eq!(replacement(Kind::SimRate, 0)[1..5], 1000u32.to_le_bytes());
    }

    #[cfg(windows)]
    #[test]
    fn sites_take_the_rate_and_others_are_left_alone() {
        use windows_sys::Win32::System::Memory::{MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_EXECUTE_READWRITE, VirtualAlloc, VirtualFree};
        // SAFETY: fresh memory of our own, written and run inside it.
        unsafe {
            let place = VirtualAlloc(std::ptr::null(), 0x10000, MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE) as *mut u8;
            assert!(!place.is_null());
            let bytes = SITES[0].bytes;
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), place.add(0x100), bytes.len());
            let mut other = SITES[1].bytes.to_vec();
            other[2] = 0x14; // [rsi+214h]
            std::ptr::copy_nonoverlapping(other.as_ptr(), place.add(0x200), other.len());
            // The simulation's rate as a function: rax = settings (+0x210 = 30),
            // the window, ret.
            let settings = [30u32; 0x100];
            let sim = SITES[2].bytes;
            let mut f = vec![0x48, 0xb8];
            f.extend((settings.as_ptr() as u64).to_le_bytes());
            let window_at = f.len();
            f.extend(sim);
            f.push(0xc3);
            std::ptr::copy_nonoverlapping(f.as_ptr(), place.add(0x300), f.len());
            let mine = [
                Site { at: 0x100, kind: Kind::Load, bytes },
                Site { at: 0x200, kind: Kind::Load, bytes: SITES[1].bytes },
                Site { at: 0x300 + window_at, kind: Kind::SimRate, bytes: sim },
                Site { at: 0xfff0, kind: Kind::Load, bytes },
            ];
            let done = apply(place as usize, 0x10000, 60, &mine);
            assert_eq!(done[..3], [Patched::Done, Patched::Differs, Patched::Done]);
            assert!(matches!(done[3], Patched::Failed(_)));
            let after = std::slice::from_raw_parts(place.add(0x100), bytes.len());
            assert_eq!(after[..6], replacement(Kind::Load, 60)[..]);
            assert_eq!(after[6..], bytes[6..]);
            assert_eq!(std::slice::from_raw_parts(place.add(0x200), other.len()), &other[..]);
            let rate: extern "system" fn() -> f32 = std::mem::transmute(place.add(0x300));
            assert_eq!(rate(), 60.0);
            VirtualFree(place.cast(), 0, MEM_RELEASE);
        }
    }

    #[test]
    fn longest_steps() {
        assert_eq!(longest_step("0.25"), Some((0.25, 0x6159ca4)));
        assert_eq!(longest_step(" 0.1 "), Some((0.1, 0x5fc7020)));
        assert_eq!(longest_step("0.3"), None);
        assert_eq!(longest_step("long"), None);
        // The load as the title has it reads its own 1/15 s.
        assert_eq!(step_load(STEPS[0].1), STEP_LOAD);
        let load = step_load(0x6159ca4);
        assert_eq!(&load[..4], &STEP_LOAD[..4]);
        assert_eq!(
            i32::from_le_bytes(load[4..].try_into().unwrap()),
            0x6159ca4 - 0x222604d
        );
    }

    #[cfg(windows)]
    #[test]
    fn the_step_load_reads_the_constant_asked_for() {
        use windows_sys::Win32::System::Memory::{
            MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_EXECUTE_READWRITE, VirtualAlloc, VirtualFree,
        };
        // SAFETY: fresh memory of our own, written and run inside it.
        unsafe {
            let place = VirtualAlloc(
                std::ptr::null(),
                0x10000,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_EXECUTE_READWRITE,
            ) as *mut u8;
            assert!(!place.is_null());
            // At 0x100: vmovss xmm3, [rip+d] reading 1/15 at 0x800; vmovaps
            // xmm0, xmm3; ret. 0.25 at 0x900, 0.2 at 0xa00.
            let own = 0x800usize;
            let mut load = STEP_LOAD;
            load[4..].copy_from_slice(&((own as i32) - 0x108).to_le_bytes());
            let mut f = load.to_vec();
            f.extend([0xc5, 0xf8, 0x28, 0xc3, 0xc3]);
            std::ptr::copy_nonoverlapping(f.as_ptr(), place.add(0x100), f.len());
            place.add(own).cast::<f32>().write_unaligned(1.0 / 15.0);
            place.add(0x900).cast::<f32>().write_unaligned(0.25);
            place.add(0xa00).cast::<f32>().write_unaligned(0.5);
            let step: extern "system" fn() -> f32 =
                std::mem::transmute::<*mut u8, extern "system" fn() -> f32>(place.add(0x100));
            assert_eq!(step(), 1.0 / 15.0);
            // A constant that does not hold the step asked for: left alone.
            let mut bytes = STEP_LOAD;
            bytes[4..].copy_from_slice(&load[4..]);
            assert_eq!(
                apply_longest_step_with(place as usize, &bytes, (0.2, 0xa00)),
                Patched::Differs
            );
            assert_eq!(step(), 1.0 / 15.0);
            assert_eq!(
                apply_longest_step_with(place as usize, &bytes, (0.25, 0x900)),
                Patched::Done
            );
            assert_eq!(step(), 0.25);
            VirtualFree(place.cast(), 0, MEM_RELEASE);
        }
    }

    /// `apply_longest_step` on a load at 0x100 whose bytes are `bytes`.
    #[cfg(windows)]
    unsafe fn apply_longest_step_with(base: usize, bytes: &[u8; 8], step: (f32, usize)) -> Patched {
        // SAFETY: the caller's memory.
        unsafe { apply_longest_step_at(base, 0x10000, 0x100, bytes, step) }
    }
}
