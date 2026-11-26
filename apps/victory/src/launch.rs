//! The launcher: before the title runs, the host pipelines of those earlier
//! runs drew with are made (`victory warm --used`, the same command as on the
//! command line, in a process of its own and its compilers in theirs), while
//! a window shows how far it is. The title then starts and its window takes
//! the loading window's place, where the loading window was.
//!
//! Only what is new or no longer valid is made: `warm` keeps what it made
//! under the fingerprint of the translator and the driver (`warm-done.txt`),
//! and a pipeline that failed (`warm-bad.txt`) is tried again once either
//! changed. Failures are shown on the screen and in the log, one line each.
//! Nothing of it goes on once the title runs: the screen waits for its
//! process to end before the title starts.
//!
//! `VICTORY_LAUNCH_WARM`: `used` (default) the pipelines earlier runs drew
//! with; `all` every pipeline the title ever created; `0` none, no window.
//!
//! Two windows, one place: a window belongs to the thread that made it and
//! only that thread receives its messages, so the title's window has to be
//! the title's own. The title's top-level windows are put where the loading
//! window is ([`victory_hle::modules::lifecycle::place_title_windows_at`]),
//! and the loading window goes when one of them shows. Its size is the one
//! the title's window had the run before (`launch-window.txt`).

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi as gdi;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::GetCurrentProcessId;
use windows_sys::Win32::UI::WindowsAndMessaging as wm;
use windows_sys::core::{PCWSTR, w};

/// The loading window's class; the presenter never takes a window of it for
/// the title's (`victory-gpu`, `title_window`).
pub const CLASS: PCWSTR = w!("VictoryNativeLoading");
const CLASS_NAME: &str = "VictoryNativeLoading";
const PLACE_FILE: &str = "launch-window.txt";
/// The failures shown at once (all of them are in the log and `warm-bad.txt`).
const SHOWN_FAILURES: usize = 5;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Stage {
    Looking,
    Making,
    Starting,
}

#[derive(Debug)]
struct Screen {
    stage: Stage,
    to_make: usize,
    known: usize,
    made_before: usize,
    failed_before: usize,
    made: usize,
    failed: usize,
    failures: Vec<String>,
    began: Instant,
    took: Option<Duration>,
    error: Option<String>,
}

impl Screen {
    fn new() -> Self {
        Screen {
            stage: Stage::Looking,
            to_make: 0,
            known: 0,
            made_before: 0,
            failed_before: 0,
            made: 0,
            failed: 0,
            failures: Vec::new(),
            began: Instant::now(),
            took: None,
            error: None,
        }
    }

    /// Takes in one line of `victory warm --progress`; false if it is not one
    /// of its steps.
    fn hear(&mut self, line: &str) -> bool {
        let mut words = line.splitn(2, ' ');
        let (word, rest) = (words.next().unwrap_or(""), words.next().unwrap_or(""));
        match word {
            "plan" => {
                let n: Vec<usize> = rest.split(' ').filter_map(|w| w.parse().ok()).collect();
                if let [to_make, known, made_before, failed_before] = n[..] {
                    *self = Screen { to_make, known, made_before, failed_before, stage: Stage::Making, ..Screen::new() };
                    self.began = Instant::now();
                }
            }
            "made" => self.made += 1,
            "failed" => {
                self.failed += 1;
                if self.failures.len() == SHOWN_FAILURES {
                    self.failures.remove(0);
                }
                self.failures.push(rest.to_owned());
            }
            "over" => self.took = Some(self.began.elapsed()),
            _ => return false,
        }
        true
    }

    fn lines(&self) -> Vec<(String, u32)> {
        const PLAIN: u32 = 0x00ffffff;
        const DIM: u32 = 0x00e8c8f0;
        const BAD: u32 = 0x008080ff;
        let mut out = Vec::new();
        let done = self.made + self.failed;
        match self.stage {
            Stage::Looking => out.push(("Looking for the pipelines earlier runs met...".to_owned(), PLAIN)),
            Stage::Making => {
                out.push(("Preparing the shaders and pipelines met in earlier runs".to_owned(), PLAIN));
                let secs = self.began.elapsed().as_secs_f64();
                let left = if done > 0 && done < self.to_make {
                    format!(", about {:.0} s left", secs / done as f64 * (self.to_make - done) as f64)
                } else {
                    String::new()
                };
                out.push((format!("{done} of {}, {} failed, {secs:.0} s{left}", self.to_make, self.failed), PLAIN));
                out.push((
                    format!(
                        "{} known; {} ready from before with this translator and driver; {} failed before with them",
                        self.known, self.made_before, self.failed_before
                    ),
                    DIM,
                ));
            }
            Stage::Starting => {
                out.push(("Starting the title...".to_owned(), PLAIN));
                let took = self.took.map_or(String::new(), |t| format!(" in {:.0} s", t.as_secs_f64()));
                out.push((format!("{} pipelines made{took}, {} failed", self.made, self.failed), DIM));
            }
        }
        if let Some(error) = &self.error {
            out.push((format!("Preparation stopped: {error}"), BAD));
            out.push(("Pipelines not ready are made when a draw first needs them, as without it.".to_owned(), DIM));
        }
        if self.failed > 0 {
            out.push((format!("Failed ({} in all, listed in warm-bad.txt and the log):", self.failed), BAD));
            out.extend(self.failures.iter().map(|f| (f.clone(), BAD)));
        }
        out
    }

    fn fraction(&self) -> f64 {
        match self.stage {
            Stage::Looking => 0.0,
            Stage::Making if self.to_make == 0 => 1.0,
            Stage::Making => (self.made + self.failed) as f64 / self.to_make as f64,
            Stage::Starting => 1.0,
        }
    }
}

static SCREEN: Mutex<Option<Screen>> = Mutex::new(None);
static WORKER: Mutex<Option<Child>> = Mutex::new(None);
static WINDOW: AtomicUsize = AtomicUsize::new(0);
static PLACE: Mutex<Option<PathBuf>> = Mutex::new(None);
static CLOSED: AtomicBool = AtomicBool::new(false);

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// What `VICTORY_LAUNCH_WARM` asks for: `None` for nothing, else whether only
/// the pipelines earlier runs drew with.
fn asked(value: Option<&str>) -> Option<bool> {
    match value.map(str::trim) {
        Some("0") => None,
        Some("all") => Some(false),
        _ => Some(true),
    }
}

/// Opens the loading window, makes the pipelines and returns once they are
/// made (or their making failed, which is shown and logged): the title may
/// start. Closing the window before the title shows ends the process.
pub fn prepare() -> Result<()> {
    let Some(used) = asked(std::env::var("VICTORY_LAUNCH_WARM").ok().as_deref()) else {
        tracing::info!("pipelines not made ahead (VICTORY_LAUNCH_WARM=0)");
        return Ok(());
    };
    let cfg = victory_core::Config::load()?;
    let dir = cfg.cache_subdir("shaders")?.with_file_name("pipelines");
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    *lock(&PLACE) = Some(dir.join(PLACE_FILE));
    *lock(&SCREEN) = Some(Screen::new());
    let place = saved_place(&dir.join(PLACE_FILE)).unwrap_or_else(default_place);
    open_window(place)?;
    victory_hle::modules::lifecycle::place_title_windows_at(place.left, place.top);

    let outcome = make(used);
    let mut screen = lock(&SCREEN);
    if let Some(screen) = screen.as_mut() {
        if let Err(error) = &outcome {
            tracing::error!("pipelines made ahead: {error:#}; those not ready are made when a draw first needs them");
            screen.error = Some(format!("{error:#}"));
        }
        tracing::info!(
            made = screen.made,
            failed = screen.failed,
            to_make = screen.to_make,
            known = screen.known,
            seconds = screen.took.unwrap_or(screen.began.elapsed()).as_secs_f64(),
            "pipelines made ahead of the title"
        );
        screen.stage = Stage::Starting;
    }
    drop(screen);
    repaint();
    Ok(())
}

/// Runs `victory warm --progress` and follows it to its end.
fn make(used: bool) -> Result<()> {
    let exe = std::env::current_exe().context("could not locate victory.exe")?;
    let mut args = vec!["warm", "--progress"];
    if used {
        args.push("--used");
    }
    let mut child = Command::new(&exe)
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .with_context(|| format!("could not start {} warm", exe.display()))?;
    let out = child.stdout.take().context("no output of victory warm")?;
    *lock(&WORKER) = Some(child);
    if CLOSED.load(Ordering::Relaxed) {
        stop_worker();
    }
    for line in BufReader::new(out).lines().map_while(Result::ok) {
        let heard = lock(&SCREEN).as_mut().is_some_and(|s| s.hear(&line));
        if heard {
            if let Some(why) = line.strip_prefix("failed ") {
                tracing::warn!("pipeline made ahead failed: {why}");
            }
        } else {
            tracing::info!("warm: {line}");
        }
        repaint();
    }
    let status = lock(&WORKER).take().map(|mut c| c.wait()).transpose()?;
    match status {
        Some(s) if !s.success() => anyhow::bail!("victory warm ended with {s}"),
        _ => Ok(()),
    }
}

fn stop_worker() {
    if let Some(child) = lock(&WORKER).as_mut() {
        // Its compilers end with it: their reports have nowhere to go.
        let _ = child.kill();
    }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

fn saved_place(file: &Path) -> Option<RECT> {
    let text = std::fs::read_to_string(file).ok()?;
    parse_place(&text)
}

fn parse_place(text: &str) -> Option<RECT> {
    let n: Vec<i32> = text.split_whitespace().filter_map(|w| w.parse().ok()).collect();
    let &[left, top, right, bottom] = &n[..] else { return None };
    (right - left >= 320 && bottom - top >= 200).then_some(RECT { left, top, right, bottom })
}

fn default_place() -> RECT {
    let mut area = RECT { left: 0, top: 0, right: 1920, bottom: 1080 };
    // SAFETY: `area` receives the primary display's work area.
    unsafe { wm::SystemParametersInfoW(wm::SPI_GETWORKAREA, 0, (&raw mut area).cast(), 0) };
    let (w, h) = (1296.min(area.right - area.left), 759.min(area.bottom - area.top));
    let left = area.left + (area.right - area.left - w) / 2;
    let top = area.top + (area.bottom - area.top - h) / 2;
    RECT { left, top, right: left + w, bottom: top + h }
}

fn repaint() {
    let hwnd = WINDOW.load(Ordering::Relaxed) as HWND;
    if !hwnd.is_null() {
        // SAFETY: our window, or one already gone (then nothing happens).
        unsafe { gdi::InvalidateRect(hwnd, std::ptr::null(), 0) };
    }
}

/// The window on a thread of its own, which does nothing else: it answers
/// at once, whatever the compilers are doing.
fn open_window(place: RECT) -> Result<()> {
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("victory-loading".into())
        .spawn(move || {
            let hwnd = make_window(place);
            let _ = sender.send(!hwnd.is_null());
            if hwnd.is_null() {
                return;
            }
            WINDOW.store(hwnd as usize, Ordering::Relaxed);
            let mut msg: wm::MSG = unsafe { std::mem::zeroed() };
            // SAFETY: the message loop of this thread's window.
            unsafe {
                while wm::GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                    wm::TranslateMessage(&msg);
                    wm::DispatchMessageW(&msg);
                }
            }
            WINDOW.store(0, Ordering::Relaxed);
        })
        .context("could not start the loading window's thread")?;
    if !receiver.recv().unwrap_or(false) {
        anyhow::bail!("could not make the loading window");
    }
    Ok(())
}

fn make_window(place: RECT) -> HWND {
    // SAFETY: registers our class once and makes a window of it.
    unsafe {
        let instance = GetModuleHandleW(std::ptr::null());
        let class = wm::WNDCLASSEXW {
            cbSize: size_of::<wm::WNDCLASSEXW>() as u32,
            style: wm::CS_HREDRAW | wm::CS_VREDRAW,
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            hCursor: wm::LoadCursorW(std::ptr::null_mut(), wm::IDC_ARROW),
            lpszClassName: CLASS,
            ..std::mem::zeroed()
        };
        wm::RegisterClassExW(&class);
        let hwnd = wm::CreateWindowExW(
            0,
            CLASS,
            w!("Victory Native"),
            wm::WS_OVERLAPPEDWINDOW | wm::WS_VISIBLE,
            place.left,
            place.top,
            place.right - place.left,
            place.bottom - place.top,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            instance,
            std::ptr::null(),
        );
        if !hwnd.is_null() {
            wm::SetTimer(hwnd, 1, 100, None);
        }
        hwnd
    }
}

extern "system" fn window_proc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match message {
        wm::WM_TIMER => {
            let starting = lock(&SCREEN).as_ref().is_some_and(|s| s.stage == Stage::Starting);
            if starting && let Some(title) = title_window(hwnd) {
                keep_place(title);
                // SAFETY: our window, on its own thread.
                unsafe { wm::DestroyWindow(hwnd) };
                return 0;
            }
            // SAFETY: our window.
            unsafe { gdi::InvalidateRect(hwnd, std::ptr::null(), 0) };
            0
        }
        wm::WM_ERASEBKGND => 1,
        wm::WM_PAINT => {
            paint(hwnd);
            0
        }
        wm::WM_CLOSE => {
            // Closed before the title showed: nothing of the title has to be
            // kept, and nothing should go on.
            CLOSED.store(true, Ordering::Relaxed);
            tracing::warn!("the loading window was closed: stopping before the title shows");
            stop_worker();
            std::process::exit(0);
        }
        wm::WM_DESTROY => {
            // SAFETY: ends this thread's message loop.
            unsafe { wm::PostQuitMessage(0) };
            0
        }
        // SAFETY: the default handling for our window.
        _ => unsafe { wm::DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

/// A visible top-level window of this process that is not a loading window.
fn title_window(ours: HWND) -> Option<HWND> {
    struct Look {
        ours: HWND,
        found: HWND,
    }
    extern "system" fn visit(hwnd: HWND, look: LPARAM) -> i32 {
        // SAFETY: `look` is the `Look` below, alive for the enumeration.
        let look = unsafe { &mut *(look as *mut Look) };
        let mut process = 0;
        let mut class = [0u16; 64];
        // SAFETY: a window handle from the enumeration, and buffers of ours.
        let (mine, visible, n) = unsafe {
            wm::GetWindowThreadProcessId(hwnd, &mut process);
            (process == GetCurrentProcessId(), wm::IsWindowVisible(hwnd) != 0, wm::GetClassNameW(hwnd, class.as_mut_ptr(), 64))
        };
        let loading = String::from_utf16_lossy(&class[..n.max(0) as usize]) == CLASS_NAME;
        if hwnd != look.ours && mine && visible && !loading {
            look.found = hwnd;
            return 0;
        }
        1
    }
    let mut look = Look { ours, found: std::ptr::null_mut() };
    // SAFETY: the callback only writes `look`, which outlives the call.
    unsafe { wm::EnumWindows(Some(visit), &raw mut look as LPARAM) };
    (!look.found.is_null()).then_some(look.found)
}

/// The title's window's place, for the next run's loading window.
fn keep_place(title: HWND) {
    let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    // SAFETY: a window of this process; `r` is ours.
    if unsafe { wm::GetWindowRect(title, &mut r) } == 0 {
        return;
    }
    if let Some(file) = lock(&PLACE).as_ref() {
        let _ = std::fs::write(file, format!("{} {} {} {}\n", r.left, r.top, r.right, r.bottom));
    }
}

/// `0x00bbggrr` between `a` and `b` at `t` (0..=1).
fn mix(a: u32, b: u32, t: f64) -> u32 {
    let ch = |c: u32, shift: u32| ((c >> shift) & 0xff) as f64;
    let at = |shift: u32| ((ch(a, shift) + (ch(b, shift) - ch(a, shift)) * t.clamp(0.0, 1.0)).round() as u32) << shift;
    at(0) | at(8) | at(16)
}

/// The sky from top to bottom: night violet, magenta, sunset orange
/// (`0x00bbggrr`).
const SKY: [u32; 3] = [0x003a0f24, 0x007a1fc2, 0x00407cff];

fn sky(t: f64) -> u32 {
    if t < 0.6 { mix(SKY[0], SKY[1], t / 0.6) } else { mix(SKY[1], SKY[2], (t - 0.6) / 0.4) }
}

fn paint(hwnd: HWND) {
    let (lines, fraction) = match lock(&SCREEN).as_ref() {
        Some(s) => (s.lines(), s.fraction()),
        None => (Vec::new(), 0.0),
    };
    // SAFETY: painting our own window into a bitmap of its size, then onto it;
    // every object made here is deleted here.
    unsafe {
        let mut ps: gdi::PAINTSTRUCT = std::mem::zeroed();
        let screen = gdi::BeginPaint(hwnd, &mut ps);
        let mut client = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        wm::GetClientRect(hwnd, &mut client);
        let (w, h) = (client.right.max(1), client.bottom.max(1));
        let dc = gdi::CreateCompatibleDC(screen);
        let bitmap = gdi::CreateCompatibleBitmap(screen, w, h);
        let old_bitmap = gdi::SelectObject(dc, bitmap);
        let fill = |r: RECT, colour: u32| {
            let brush = gdi::CreateSolidBrush(colour);
            gdi::FillRect(dc, &r, brush);
            gdi::DeleteObject(brush);
        };
        // The sky, in bands.
        let band = 3;
        for y in (0..h).step_by(band as usize) {
            fill(RECT { left: 0, top: y, right: w, bottom: y + band }, sky(f64::from(y) / f64::from(h)));
        }
        // A low sun, cut by the lines of the horizon.
        let radius = h / 4;
        let (cx, cy) = (w * 7 / 10, h * 7 / 10);
        for y in (cy - radius)..cy.min(h) {
            let dy = f64::from(cy - y);
            let half = (f64::from(radius).powi(2) - dy * dy).max(0.0).sqrt() as i32;
            let below = f64::from(y - (cy - radius)) / f64::from(radius);
            // Gaps that widen towards the horizon.
            let gap = (below * 8.0) as i32;
            if gap > 0 && (y - (cy - radius)).rem_euclid(14) < gap.min(10) {
                continue;
            }
            fill(RECT { left: cx - half, top: y, right: cx + half, bottom: y + 1 }, mix(0x0060e6ff, 0x008a3dff, below));
        }
        // Ground: dark water below the horizon.
        fill(RECT { left: 0, top: cy, right: w, bottom: h }, 0x00301020);
        for k in 0..6 {
            let y = cy + (h - cy) * k * k / 36;
            fill(RECT { left: 0, top: y, right: w, bottom: y + 1 }, 0x00803a90);
        }
        let font = |height: i32, weight: i32, italic: u32, face: PCWSTR| {
            gdi::CreateFontW(height, 0, 0, 0, weight, italic, 0, 0, 1, 0, 0, 5, 0, face)
        };
        let title_h = (h / 7).clamp(32, 120);
        let small_h = (h / 42).clamp(14, 26);
        let big = font(-title_h, 800, 1, w!("Bahnschrift SemiBold Condensed"));
        let small = font(-small_h, 600, 0, w!("Bahnschrift"));
        gdi::SetBkMode(dc, gdi::TRANSPARENT as _);
        let margin = w / 12;
        let text = |s: &str, top: i32, height: i32, colour: u32, f| {
            let old = gdi::SelectObject(dc, f);
            let mut t = wide(s);
            let flags = gdi::DT_LEFT | gdi::DT_SINGLELINE | gdi::DT_END_ELLIPSIS | gdi::DT_NOPREFIX;
            // A soft shadow, then the text.
            gdi::SetTextColor(dc, 0x00200818);
            let mut r = RECT { left: margin + 2, top: top + 2, right: w - margin + 2, bottom: top + height + 2 };
            gdi::DrawTextW(dc, t.as_mut_ptr(), -1, &mut r, flags);
            gdi::SetTextColor(dc, colour);
            let mut r = RECT { left: margin, top, right: w - margin, bottom: top + height };
            gdi::DrawTextW(dc, t.as_mut_ptr(), -1, &mut r, flags);
            gdi::SelectObject(dc, old);
        };
        let line = small_h * 3 / 2;
        let mut y = h / 8;
        text("VICTORY NATIVE", y, title_h * 5 / 4, 0x00ffffff, big);
        y += title_h * 5 / 4 + line / 2;
        // The bar: a dark track, then pink to orange as far as it has come.
        let bar = RECT { left: margin, top: y, right: w - margin, bottom: y + (line / 2).max(8) };
        fill(bar, 0x00301028);
        let filled = bar.left + (f64::from(bar.right - bar.left) * fraction.clamp(0.0, 1.0)) as i32;
        for x in (bar.left..filled).step_by(2) {
            let t = f64::from(x - bar.left) / f64::from((bar.right - bar.left).max(1));
            fill(RECT { left: x, top: bar.top, right: x + 2, bottom: bar.bottom }, mix(0x00b040ff, 0x0040a8ff, t));
        }
        y += line;
        for (s, colour) in &lines {
            text(s, y, line, *colour, small);
            y += line;
        }
        gdi::BitBlt(screen, 0, 0, w, h, dc, 0, 0, gdi::SRCCOPY);
        gdi::SelectObject(dc, old_bitmap);
        gdi::DeleteObject(bitmap);
        gdi::DeleteObject(big);
        gdi::DeleteObject(small);
        gdi::DeleteDC(dc);
        gdi::EndPaint(hwnd, &ps);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_screen_follows_warm() {
        let mut s = Screen::new();
        assert_eq!(s.stage, Stage::Looking);
        assert!(s.hear("plan 4 100 90 1"));
        assert_eq!((s.stage, s.to_make, s.known, s.made_before, s.failed_before), (Stage::Making, 4, 100, 90, 1));
        assert!(s.hear("made 0123"));
        assert!(s.hear("failed 4567 hung for 180 s"));
        assert!(!s.hear("4 pipelines known, ..."));
        assert_eq!((s.made, s.failed), (1, 1));
        assert_eq!(s.failures, ["4567 hung for 180 s"]);
        assert!((s.fraction() - 0.5).abs() < 1e-9);
        assert!(s.lines().iter().any(|(l, _)| l.contains("4567 hung")));
        assert!(s.hear("over"));
        assert!(s.took.is_some());
    }

    #[test]
    fn only_the_last_failures_are_shown_all_are_counted() {
        let mut s = Screen::new();
        s.hear("plan 10 10 0 0");
        for k in 0..8 {
            s.hear(&format!("failed k{k} why"));
        }
        assert_eq!(s.failed, 8);
        assert_eq!(s.failures.len(), SHOWN_FAILURES);
        assert_eq!(s.failures[0], "k3 why");
    }

    #[test]
    fn nothing_to_make_is_complete() {
        let mut s = Screen::new();
        s.hear("plan 0 50 50 0");
        assert_eq!(s.fraction(), 1.0);
    }

    #[test]
    fn colours_mix_by_channel() {
        assert_eq!(mix(0x00000000, 0x00ffffff, 0.0), 0);
        assert_eq!(mix(0x00000000, 0x00ffffff, 1.0), 0x00ffffff);
        assert_eq!(mix(0x00000010, 0x00000030, 0.5), 0x00000020);
        assert_eq!(sky(0.0), SKY[0]);
        assert_eq!(sky(1.0), SKY[2]);
    }

    #[test]
    fn the_switch() {
        assert_eq!(asked(None), Some(true));
        assert_eq!(asked(Some("used")), Some(true));
        assert_eq!(asked(Some("all")), Some(false));
        assert_eq!(asked(Some(" 0")), None);
    }

    #[test]
    fn a_saved_place_is_four_numbers_of_a_usable_size() {
        let r = parse_place("10 20 1306 779\n").unwrap();
        assert_eq!((r.left, r.top, r.right, r.bottom), (10, 20, 1306, 779));
        assert!(parse_place("10 20 30 40").is_none());
        assert!(parse_place("garbage").is_none());
    }
}
