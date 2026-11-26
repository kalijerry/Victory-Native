//! Renderer port #2 (`VICTORY_PORT_DESCRIPTORS=1`, read in `victory-gpu`'s
//! `port_descriptors`): the title's own code that copies descriptor records
//! into its shader-visible heap as bytes tells the layer which slots it wrote.
//!
//! What the title does (game_scarlett_master.exe): each draw's and each
//! dispatch's descriptor tables are allocated from its frame allocator
//! (image +0x4d9f400; the allocator's memory is the placed shader-visible
//! view heap) and filled by two functions, +0x4d9efd0 (graphics) and
//! +0x4eec8e0 (compute), three loops each, one record at a time:
//! `and r, 7FE0h` (the slot's byte offset in the table), `vmovups ymm0,
//! [record]`, `vmovups [table + r], ymm0`. Found with `dumpbin /disasm` from
//! the write faults of run faults-6 (book records "the instruction at a made
//! the fault"); a search of all of `.text` for the same instruction shape
//! finds these six stores and no other.
//!
//! Each store becomes a jump to code of ours that does the store, then, if
//! the address lies in the heap `victory-gpu` names in [`Marks`], sets the
//! slot's byte in a map of slots and its 4 KiB chunk's byte in a map of
//! chunks, and jumps back. The flags are clobbered: at every site the next
//! instructions are a `movzx` and a `cmp`, which sets them before any use
//! (checked as part of the original bytes). Every register is put back.
//!
//! Four more sites (issue #301) are where a function that takes a table
//! from the frame allocator goes on once it is filled, and every slot the
//! table overlaps is marked ([`Kind::Range`]): +0x4d9f200, which copies a
//! ready list of records into it (`vmovups` with a displacement, or the
//! imported `memcpy`), and +0x4e51a90, +0x248d6d0 and +0x248d780, which have
//! a callback the caller passes fill it.
//!
//! Each site's original bytes (from the `and` to the `cmp`) are compared
//! before anything is written; a site whose bytes differ is left alone and
//! said so (another build of the title).

/// The cells the stubs read, `#[repr(C)]` in `victory-gpu`: the heap's first
/// byte, its length in bytes (0: none, nothing is marked), the map of slots
/// (a byte each), the map of chunks (a byte for 128 slots).
pub const MARKS_LEN: usize = 32;

/// A place in the title's code to hook: the window's offset in the image,
/// the hooked instructions' offset in the window and their length, what is
/// done there, and the window's bytes.
pub struct Site {
    pub window: usize,
    pub store: usize,
    pub len: usize,
    pub kind: Kind,
    pub bytes: &'static [u8],
}

/// What a site's code does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A `vmovups [base+index], ymm` of one record: its slot is marked.
    Store,
    /// Instructions after a table of `size` bytes (register) at `table`
    /// (register) was written whole: every slot it overlaps is marked, then
    /// the instructions run (`moved`; none of them may read the flags).
    Range { table: u8, size: u8, moved: Moved },
}

/// How a range site's instructions run in its stub.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Moved {
    /// As they are (none of them reads `rip`).
    AsIs,
    /// `mov eax, [rip+rel32]` (6 bytes): as `mov rax, address ; mov eax,
    /// [rax]`, the same `rax` and flags.
    LoadEax,
}

/// The code that does the moved instructions `bytes` at `at` as `moved`
/// says, elsewhere.
fn moved_code(moved: Moved, bytes: &[u8], at: usize) -> Option<Vec<u8>> {
    match moved {
        Moved::AsIs => Some(bytes.to_vec()),
        Moved::LoadEax => {
            let &[0x8b, 0x05, a, b, c, d] = bytes else { return None };
            let address = (at as u64 + 6).wrapping_add_signed(i64::from(i32::from_le_bytes([a, b, c, d])));
            let mut out = vec![0x48, 0xb8]; // mov rax, address
            out.extend(address.to_le_bytes());
            out.extend([0x8b, 0x00]); // mov eax, [rax]
            Some(out)
        }
    }
}

/// The six record stores of the table fillers, the end of the copy of ready
/// tables, and the ends of the fills of tables by callbacks
/// (game_scarlett_master.exe 1.0.2.1, image offsets).
pub const SITES: [Site; 10] = [
    // +0x4d9efd0, loop 1: [rcx+rsi]
    Site { window: 0x4d9f04f, store: 10, len: 5, kind: Kind::Store, bytes: &[0x81, 0xe1, 0xe0, 0x7f, 0x00, 0x00, 0xc5, 0xfc, 0x10, 0x00, 0xc5, 0xfc, 0x11, 0x04, 0x31, 0x0f, 0xb6, 0x47, 0x0b, 0x39, 0xc5] },
    // loop 2: [rcx+rsi]
    Site { window: 0x4d9f10d, store: 10, len: 5, kind: Kind::Store, bytes: &[0x81, 0xe1, 0xe0, 0x7f, 0x00, 0x00, 0xc5, 0xfc, 0x10, 0x00, 0xc5, 0xfc, 0x11, 0x04, 0x31, 0x0f, 0xb6, 0x47, 0x0f, 0x41, 0x39, 0xc5] },
    // loop 3: [r9+rsi]
    Site { window: 0x4d9f19c, store: 12, len: 6, kind: Kind::Store, bytes: &[0x41, 0x81, 0xe1, 0xe0, 0x7f, 0x00, 0x00, 0xc4, 0xc1, 0x7c, 0x10, 0x02, 0xc4, 0xc1, 0x7c, 0x11, 0x04, 0x31, 0x44, 0x0f, 0xb6, 0x4f, 0x0d, 0x44, 0x39, 0xc8] },
    // +0x4eec8e0, loop 1: [r9+r8]
    Site { window: 0x4eec95d, store: 11, len: 6, kind: Kind::Store, bytes: &[0x41, 0x81, 0xe1, 0xe0, 0x7f, 0x00, 0x00, 0xc5, 0xfc, 0x10, 0x00, 0xc4, 0x81, 0x7c, 0x11, 0x04, 0x01, 0x0f, 0xb6, 0x42, 0x0b, 0x39, 0xc5] },
    // loop 2: [rdi+r8]
    Site { window: 0x4eeca08, store: 10, len: 6, kind: Kind::Store, bytes: &[0x81, 0xe7, 0xe0, 0x7f, 0x00, 0x00, 0xc5, 0xfc, 0x10, 0x06, 0xc4, 0xa1, 0x7c, 0x11, 0x04, 0x07, 0x0f, 0xb6, 0x72, 0x0d, 0x39, 0xf0] },
    // loop 3: [r10+r8]
    Site { window: 0x4eeca82, store: 11, len: 6, kind: Kind::Store, bytes: &[0x41, 0x81, 0xe2, 0xe0, 0x7f, 0x00, 0x00, 0xc5, 0xfc, 0x10, 0x00, 0xc4, 0x81, 0x7c, 0x11, 0x04, 0x02, 0x0f, 0xb6, 0x42, 0x0f, 0x41, 0x39, 0xc6] },
    // +0x4d9f200 (issue #301): a table of `r14` bytes is taken from the
    // frame allocator (`rbx`, 16-byte aligned) and a ready list of records
    // copied into it whole: four `vmovups [rbx+disp], ymm` for 128 bytes (a
    // form the search for the six above does not find), the imported `memcpy`
    // otherwise, whose wrapper finds no record on a slot's start when the
    // table begins half into one. Both ways meet at +0x4d9f272 (the
    // `memcpy` way jumps there): `mov rcx, r15 ; mov rdx, rsi` are moved, and
    // the `xor r9d, r9d` after them sets the flags.
    Site {
        window: 0x4d9f237,
        store: 0x3b,
        len: 6,
        kind: Kind::Range { table: 3, size: 14, moved: Moved::AsIs },
        bytes: &[
            0x49, 0x81, 0xfe, 0x80, 0x00, 0x00, 0x00, // cmp r14, 80h
            0x0f, 0x85, 0x39, 0x01, 0x00, 0x00, // jne +0x4d9f37d (the memcpy)
            0xc4, 0xc1, 0x7c, 0x10, 0x04, 0x24, // vmovups ymm0, [r12]
            0xc4, 0xc1, 0x7c, 0x10, 0x4c, 0x24, 0x20, // ymm1, [r12+20h]
            0xc4, 0xc1, 0x7c, 0x10, 0x54, 0x24, 0x40, // ymm2, [r12+40h]
            0xc4, 0xc1, 0x7c, 0x10, 0x5c, 0x24, 0x60, // ymm3, [r12+60h]
            0xc5, 0xfc, 0x11, 0x5b, 0x60, // vmovups [rbx+60h], ymm3
            0xc5, 0xfc, 0x11, 0x53, 0x40, // [rbx+40h], ymm2
            0xc5, 0xfc, 0x11, 0x4b, 0x20, // [rbx+20h], ymm1
            0xc5, 0xfc, 0x11, 0x03, // [rbx], ymm0
            0x4c, 0x89, 0xf9, // mov rcx, r15 (hooked)
            0x48, 0x89, 0xf2, // mov rdx, rsi (hooked)
            0x49, 0x89, 0xd8, // mov r8, rbx
            0x45, 0x31, 0xc9, // xor r9d, r9d
        ],
    },
    // +0x4e51a90 (issue #301): a table of `r14` bytes from the frame
    // allocator (`rbx`), filled by a callback the caller passes (`{context,
    // function}` at r15), any code at all. Both ways of calling it meet at
    // +0x4e51aee (the second jumps there): `mov rcx, rsi ; mov edx, edi` are
    // moved; nothing after them reads the flags before the call.
    Site {
        window: 0x4e51ad3,
        store: 0x1b,
        len: 5,
        kind: Kind::Range { table: 3, size: 14, moved: Moved::AsIs },
        bytes: &[
            0x48, 0x89, 0xc3, // mov rbx, rax (the table)
            0x49, 0x8b, 0x0f, // mov rcx, [r15]
            0x49, 0x8b, 0x47, 0x08, // mov rax, [r15+8]
            0x48, 0x85, 0xc9, // test rcx, rcx
            0x0f, 0x84, 0x85, 0x00, 0x00, 0x00, // je +0x4e51b6b
            0x48, 0x89, 0xda, // mov rdx, rbx
            0x4d, 0x89, 0xf0, // mov r8, r14
            0xff, 0xd0, // call rax
            0x48, 0x89, 0xf1, // mov rcx, rsi (hooked)
            0x89, 0xfa, // mov edx, edi (hooked)
            0x49, 0x89, 0xd8, // mov r8, rbx
            0x4d, 0x89, 0xf1, // mov r9, r14
            0x40, 0x88, 0x6c, 0x24, 0x28, // mov [rsp+28h], bpl
            0x4c, 0x89, 0x64, 0x24, 0x20, // mov [rsp+20h], r12
        ],
    },
    // +0x248d6d0 and its twin +0x248d780 (issue #301; another allocator at
    // +0x12300 for the twin): as above, the table in `r14`, its size in
    // `rdi`, filled by a callback; both calls meet at the `mov eax,
    // [rip+...]` (+0x248d71b, +0x248d7cb), which the stub does from its
    // address. The `sub r9d, eax` after it sets the flags.
    Site { window: 0x248d6fa, store: 0x21, len: 6, kind: Kind::Range { table: 14, size: 7, moved: Moved::LoadEax }, bytes: &CALLBACK_TABLE_A },
    Site { window: 0x248d7aa, store: 0x21, len: 6, kind: Kind::Range { table: 14, size: 7, moved: Moved::LoadEax }, bytes: &CALLBACK_TABLE_B },
];

/// The window of +0x248d6d0's join (`rel` its two `rip`-relative loads).
const fn callback_table(rel: [[u8; 4]; 2]) -> [u8; 0x3e] {
    let [[a, b, c, d], [e, f, g, h]] = rel;
    [
        0x49, 0x89, 0xc6, // mov r14, rax (the table)
        0x49, 0x8b, 0x0f, // mov rcx, [r15]
        0x49, 0x8b, 0x47, 0x08, // mov rax, [r15+8]
        0x48, 0x85, 0xc9, // test rcx, rcx
        0x74, 0x0a, // je: the second call
        0x4c, 0x89, 0xf2, // mov rdx, r14
        0x49, 0x89, 0xf8, // mov r8, rdi
        0xff, 0xd0, // call rax
        0xeb, 0x08, // jmp to the join
        0x4c, 0x89, 0xf1, // mov rcx, r14
        0x48, 0x89, 0xfa, // mov rdx, rdi
        0xff, 0xd0, // call rax
        0x8b, 0x05, a, b, c, d, // mov eax, [rip+...] (hooked)
        0x4c, 0x8b, 0x05, e, f, g, h, // mov r8, [rip+...]
        0x45, 0x89, 0xf1, // mov r9d, r14d
        0x48, 0x89, 0xd9, // mov rcx, rbx
        0x89, 0xf2, // mov edx, esi
        0x48, 0x89, 0x7c, 0x24, 0x20, // mov [rsp+20h], rdi
        0x41, 0x29, 0xc1, // sub r9d, eax
    ]
}

const CALLBACK_TABLE_A: [u8; 0x3e] = callback_table([[0x97, 0x1a, 0x15, 0x05], [0x10, 0x1a, 0x15, 0x05]]);
const CALLBACK_TABLE_B: [u8; 0x3e] = callback_table([[0xe7, 0x19, 0x15, 0x05], [0x60, 0x19, 0x15, 0x05]]);

/// `mov to, from` (64 bits, registers by number).
fn mov(to: u8, from: u8) -> [u8; 3] {
    [0x48 | (from >> 3) << 2 | (to >> 3), 0x89, 0xc0 | (from & 7) << 3 | (to & 7)]
}

/// The code at `to` for a range site: every slot of the marked heap that the
/// `size` bytes at `table` overlap is marked (a table that begins half into a
/// slot marks it too), then `moved` (the site's instructions, as they run
/// here), then back to `resume`. The registers it uses are put back; `table`
/// and `size` may not be one of them, nor `rsp`.
fn range_stub(table: u8, size: u8, moved: &[u8], resume: usize, to: usize, marks: usize) -> Option<Vec<u8>> {
    // rax, rcx, rdx, r8 and rsp.
    if [table, size].iter().any(|r| matches!(r, 0 | 1 | 2 | 4 | 8) || *r > 15) {
        return None;
    }
    let mut out = vec![0x50, 0x51, 0x52, 0x41, 0x50]; // push rax ; push rcx ; push rdx ; push r8
    out.extend(mov(0, table)); // mov rax, table
    out.extend(mov(8, size)); // mov r8, size
    out.extend([0x48, 0xba]); // mov rdx, marks
    out.extend((marks as u64).to_le_bytes());
    out.extend([0x48, 0x2b, 0x02]); // sub rax, [rdx]: the table's offset in the heap
    out.extend([0x4d, 0x85, 0xc0]); // test r8, r8
    let zero = out.len();
    out.extend([0x74, 0]); // jz done
    out.extend([0x48, 0x3b, 0x42, 0x08]); // cmp rax, [rdx+8]
    let outside = out.len();
    out.extend([0x73, 0]); // jae done (before the heap too: wrapped)
    out.extend([0x49, 0x01, 0xc0]); // add r8, rax: its end
    out.extend([0x4c, 0x3b, 0x42, 0x08]); // cmp r8, [rdx+8]
    out.extend([0x76, 0x04]); // jbe +4
    out.extend([0x4c, 0x8b, 0x42, 0x08]); // mov r8, [rdx+8]: no further than the heap
    out.extend([0x49, 0xff, 0xc8]); // dec r8: its last byte
    out.extend([0x48, 0xc1, 0xe8, 0x05]); // shr rax, 5: the first slot
    out.extend([0x49, 0xc1, 0xe8, 0x05]); // shr r8, 5: the last
    let looped = out.len();
    out.extend([0x48, 0x8b, 0x4a, 0x10]); // mov rcx, [rdx+16]
    out.extend([0xc6, 0x04, 0x01, 0x01]); // mov byte [rcx+rax], 1
    out.extend(mov(1, 0)); // mov rcx, rax
    out.extend([0x48, 0xc1, 0xe9, 0x07]); // shr rcx, 7: the chunk
    out.extend([0x48, 0x03, 0x4a, 0x18]); // add rcx, [rdx+24]
    out.extend([0xc6, 0x01, 0x01]); // mov byte [rcx], 1
    out.extend([0x48, 0xff, 0xc0]); // inc rax
    out.extend([0x4c, 0x39, 0xc0]); // cmp rax, r8
    let back = looped as i64 - (out.len() as i64 + 2);
    out.extend([0x76, i8::try_from(back).ok()? as u8]); // jbe loop
    let done = out.len();
    out.extend([0x41, 0x58, 0x5a, 0x59, 0x58]); // pop r8 ; pop rdx ; pop rcx ; pop rax
    for jump in [zero, outside] {
        out[jump + 1] = i8::try_from(done - (jump + 2)).ok()? as u8;
    }
    out.extend(moved);
    out.extend(crate::console_insn::jump(to + out.len(), resume)?);
    Some(out)
}

/// `lea rax, [base+index]` for a `vmovups [base+index], ymm0` (VEX, ModRM
/// 04h with a SIB byte, no displacement, the stack pointer not its base):
/// `None` for another form.
fn lea_of(store: &[u8]) -> Option<[u8; 4]> {
    let (x, b, op_at) = match *store.first()? {
        0xc5 => (false, false, 2),
        0xc4 => {
            let p = *store.get(1)?;
            if p & 0x1f != 1 {
                return None;
            }
            (p & 0x40 == 0, p & 0x20 == 0, 3)
        }
        _ => return None,
    };
    let (op, modrm, sib) = (*store.get(op_at)?, *store.get(op_at + 1)?, *store.get(op_at + 2)?);
    if op != 0x11 || modrm != 0x04 || store.len() != op_at + 3 {
        return None;
    }
    // No base (mode 0, base 101), or the stack pointer as base, or rsp as index (none).
    if sib & 7 == 5 || (sib & 7 == 4 && !b) || (sib >> 3 & 7 == 4 && !x) {
        return None;
    }
    Some([0x48 | u8::from(x) << 1 | u8::from(b), 0x8d, 0x04, sib])
}

/// The code at `to` for the store `store` at `at`: the store, the marks, back
/// to `at + store.len()`.
fn stub(store: &[u8], at: usize, to: usize, marks: usize) -> Option<Vec<u8>> {
    let lea = lea_of(store)?;
    let mut out = store.to_vec();
    out.push(0x50); // push rax
    out.extend(lea); // lea rax, [base+index]
    out.extend([0x52, 0x51]); // push rdx ; push rcx
    out.extend([0x48, 0xba]); // mov rdx, marks
    out.extend((marks as u64).to_le_bytes());
    out.extend([0x48, 0x2b, 0x02]); // sub rax, [rdx]
    out.extend([0x48, 0x3b, 0x42, 0x08]); // cmp rax, [rdx+8]
    out.extend([0x73, 0x18]); // jae past the marks
    out.extend([0x48, 0xc1, 0xe8, 0x05]); // shr rax, 5: the slot
    out.extend([0x48, 0x8b, 0x4a, 0x10]); // mov rcx, [rdx+16]
    out.extend([0xc6, 0x04, 0x01, 0x01]); // mov byte [rcx+rax], 1
    out.extend([0x48, 0xc1, 0xe8, 0x07]); // shr rax, 7: the chunk
    out.extend([0x48, 0x8b, 0x4a, 0x18]); // mov rcx, [rdx+24]
    out.extend([0xc6, 0x04, 0x01, 0x01]); // mov byte [rcx+rax], 1
    out.extend([0x59, 0x5a, 0x58]); // pop rcx ; pop rdx ; pop rax
    let back = crate::console_insn::jump(to + out.len(), at + store.len())?;
    out.extend(back);
    Some(out)
}

/// What became of a site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hooked {
    Done,
    /// Its bytes are not the ones expected: left alone.
    Differs,
    /// Outside the image, no room for code near it, or Windows refused.
    Failed(&'static str),
}

/// Hooks `sites` in the image of `size` bytes at `base`; the stubs read the
/// cells at `marks` ([`MARKS_LEN`] bytes). What became of each, said in the
/// log too.
///
/// # Safety
/// The image stays mapped and no thread runs its code yet; `marks` stays
/// valid for as long as the image runs.
pub unsafe fn apply(base: usize, size: usize, marks: usize, sites: &[Site]) -> Vec<Hooked> {
    use windows_sys::Win32::System::Diagnostics::Debug::FlushInstructionCache;
    use windows_sys::Win32::System::Memory::{PAGE_EXECUTE_READWRITE, VirtualProtect};
    use windows_sys::Win32::System::Threading::GetCurrentProcess;
    let mut room: Option<(usize, usize)> = None;
    let mut out = Vec::new();
    for site in sites {
        let done = (|| {
            if site.window.checked_add(site.bytes.len()).is_none_or(|end| end > size) || site.store + site.len > site.bytes.len() {
                return Hooked::Failed("outside the image");
            }
            let window = base + site.window;
            // SAFETY: inside the mapped image (checked above).
            let there = unsafe { std::slice::from_raw_parts(window as *const u8, site.bytes.len()) };
            if there != site.bytes {
                return Hooked::Differs;
            }
            let at = window + site.store;
            if room.is_none() {
                room = crate::console_insn::near(at).map(|place| (place, 0));
            }
            let Some((place, used)) = room.as_mut() else {
                return Hooked::Failed("no room for code near the title's");
            };
            let to = *place + *used;
            let store = &site.bytes[site.store..site.store + site.len];
            let body = match site.kind {
                Kind::Store => stub(store, at, to, marks),
                Kind::Range { table, size, moved } if site.len >= 5 => moved_code(moved, store, at).and_then(|code| range_stub(table, size, &code, at + site.len, to, marks)),
                Kind::Range { .. } => None,
            };
            let (Some(body), Some(jump)) = (body, crate::console_insn::jump(at, to)) else {
                return Hooked::Failed("not a store of the expected form, or out of reach");
            };
            if *used + body.len() > crate::console_insn::STUBS_LEN {
                return Hooked::Failed("no room for code near the title's");
            }
            let mut old = 0;
            // SAFETY: our own code memory, which nothing runs yet; then the
            // store, whose page is made writable for it and put back, and
            // which nothing runs either.
            unsafe {
                std::ptr::copy_nonoverlapping(body.as_ptr(), to as *mut u8, body.len());
                if VirtualProtect(at as *const _, site.len, PAGE_EXECUTE_READWRITE, &mut old) == 0 {
                    return Hooked::Failed("Windows would not make it writable");
                }
                std::ptr::write_bytes(at as *mut u8, 0x90, site.len);
                std::ptr::copy_nonoverlapping(jump.as_ptr(), at as *mut u8, jump.len());
                VirtualProtect(at as *const _, site.len, old, &mut old);
                FlushInstructionCache(GetCurrentProcess(), at as *const _, site.len);
            }
            *used += body.len().next_multiple_of(16);
            Hooked::Done
        })();
        let offset = format_args!("{:#x}", site.window + site.store);
        match done {
            Hooked::Done => tracing::info!(site = %offset, "renderer port 2: the title's descriptor record store at title+site patched to tell the layer its slot (VICTORY_PORT_DESCRIPTORS)"),
            Hooked::Differs => tracing::warn!(site = %offset, "renderer port 2: not patched, the title's bytes there are not the ones expected (another build?)"),
            Hooked::Failed(why) => tracing::warn!(site = %offset, why, "renderer port 2: not patched"),
        }
        out.push(done);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The sites of single record stores.
    fn stores() -> impl Iterator<Item = &'static Site> {
        SITES.iter().filter(|s| s.kind == Kind::Store)
    }

    #[test]
    fn the_sites_lea_their_stores_address() {
        let leas: Vec<[u8; 4]> = stores().map(|s| lea_of(&s.bytes[s.store..s.store + s.len]).unwrap()).collect();
        assert_eq!(leas[0], [0x48, 0x8d, 0x04, 0x31]); // lea rax, [rcx+rsi]
        assert_eq!(leas[1], [0x48, 0x8d, 0x04, 0x31]);
        assert_eq!(leas[2], [0x49, 0x8d, 0x04, 0x31]); // [r9+rsi]
        assert_eq!(leas[3], [0x4b, 0x8d, 0x04, 0x01]); // [r9+r8]
        assert_eq!(leas[4], [0x4a, 0x8d, 0x04, 0x07]); // [rdi+r8]
        assert_eq!(leas[5], [0x4b, 0x8d, 0x04, 0x02]); // [r10+r8]
        // Each window: the store follows the record's load and is followed
        // by a movzx and the cmp that sets the flags.
        for s in stores() {
            let after = &s.bytes[s.store + s.len..];
            let movzx = if after[0] == 0x0f { 0 } else { 1 };
            assert_eq!(&after[movzx..movzx + 2], &[0x0f, 0xb6]);
            assert_eq!(after[after.len() - 2], 0x39, "cmp");
        }
        // A store with a displacement, of the stack, of another kind: not taken.
        assert!(lea_of(&[0xc5, 0xfc, 0x11, 0x44, 0x31, 0x08]).is_none());
        assert!(lea_of(&[0xc5, 0xfc, 0x11, 0x04, 0x24]).is_none());
        assert!(lea_of(&[0xc5, 0xfc, 0x10, 0x04, 0x31]).is_none());
    }

    /// The marks `victory-gpu` keeps, for a test.
    #[repr(C)]
    struct Marks {
        start: u64,
        len: u64,
        slots: *mut u8,
        chunks: *mut u8,
    }

    /// Each site's window in code of a test's, twice, one copy hooked, both
    /// run: the record is stored as before, the registers the window leaves
    /// are the same, and the slot and its chunk are marked when the table is
    /// in the heap, not otherwise.
    #[test]
    fn a_hooked_site_stores_and_marks_its_slot() {
        use windows_sys::Win32::System::Memory::{MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READWRITE, VirtualAlloc};
        // mov to, from (64 bits)
        let mov = |to: u8, from: u8| [0x48 | (from >> 3) << 2 | (to >> 3), 0x89, 0xc0 | (from & 7) << 3 | (to & 7)];
        for site in stores() {
            let store = &site.bytes[site.store..site.store + site.len];
            let lea = lea_of(store).unwrap();
            let base = lea[3] & 7 | (lea[0] & 1) << 3;
            let index = lea[3] >> 3 & 7 | (lea[0] & 2) << 2;
            let load_at = site.store - if site.bytes[site.store - 5] == 0xc4 { 5 } else { 4 };
            let load = &site.bytes[load_at..site.store];
            let load_reg = if load[0] == 0xc4 { load[4] & 7 | 8 } else { load[3] & 7 };
            // fn(offset: rcx, table: rdx, record: r8, scratch: r9) -> rax:
            // the store's address as the window leaves its registers.
            let mut code: Vec<u8> = vec![0x56, 0x57, 0x53]; // push rsi ; push rdi ; push rbx
            code.extend(mov(11, 1)); // r11 = offset
            code.extend(mov(3, 2)); // rbx = table
            code.extend(mov(6, 8)); // rsi = record
            for scratch in [7, 10, 2] {
                code.extend(mov(scratch, 9)); // rdi, r10, rdx: what the movzx reads
            }
            // The window's `and` masks the offset's register; the table is in
            // the other. (The record first: rsi may be one of them.)
            let rex = usize::from(site.bytes[0] == 0x41);
            let offset = site.bytes[rex + 1] & 7 | if rex == 1 { 8 } else { 0 };
            assert!(offset == base || offset == index);
            let table = if offset == base { index } else { base };
            code.extend(mov(load_reg, 6));
            code.extend(mov(table, 3));
            code.extend(mov(offset, 11));
            let at = code.len();
            code.extend(site.bytes);
            code.extend(lea); // lea rax, [base+index]
            code.extend([0x5b, 0x5f, 0x5e]); // pop rbx ; pop rdi ; pop rsi
            code.extend([0xc5, 0xf8, 0x77, 0xc3]); // vzeroupper ; ret
            type F = unsafe extern "system" fn(u64, *mut u8, *const u8, *mut u8) -> u64;
            // SAFETY: fresh memory for code and data, one copy changed before it runs.
            unsafe {
                let place = VirtualAlloc(std::ptr::null(), 0x10000, MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE) as usize;
                assert_ne!(place, 0);
                let (plain, hooked) = (place + 0x100 - at, place + 0x1100 - at);
                std::ptr::copy_nonoverlapping(code.as_ptr(), plain as *mut u8, code.len());
                std::ptr::copy_nonoverlapping(code.as_ptr(), hooked as *mut u8, code.len());
                let heap = VirtualAlloc(std::ptr::null(), 0x10000, MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE) as *mut u8;
                let scratch = [0u8; 64];
                let (mut slots, mut chunks) = (vec![0u8; 0x8000 / 32], vec![0u8; 0x8000 / 32 / 128]);
                let marks = Box::new(Marks { start: heap as u64 + 0x2000, len: 0x8000, slots: slots.as_mut_ptr(), chunks: chunks.as_mut_ptr() });
                let mine = [Site { window: 0x1100, ..*site }];
                assert_eq!(apply(place, 0x10000, &raw const *marks as usize, &mine), [Hooked::Done]);
                assert_eq!(*((place + 0x1100 + site.store) as *const u8), 0xe9);
                let (plain, hooked): (F, F) = (std::mem::transmute(plain), std::mem::transmute(hooked));
                let record: [u8; 32] = std::array::from_fn(|k| k as u8 ^ 0x5a);
                // In the heap: slot 0x40 + 3 of its start.
                let table = heap.add(0x2000 + 0x40 * 32);
                let first = plain(0x60 | 0x8000_0000, table.add(0x4000), record.as_ptr(), scratch.as_ptr().cast_mut());
                let second = hooked(0x60 | 0x8000_0000, table, record.as_ptr(), scratch.as_ptr().cast_mut());
                assert_eq!(first.wrapping_sub(0x4000), second, "the registers the window leaves");
                assert_eq!(std::slice::from_raw_parts(table.add(0x60), 32), &record);
                assert_eq!(std::slice::from_raw_parts(table.add(0x4060), 32), &record);
                let slot = 0x40 + 3;
                assert_eq!(slots[slot], 1);
                assert_eq!(slots.iter().filter(|&&b| b != 0).count(), 1);
                assert_eq!(chunks[slot / 128], 1);
                // Before the heap: stored, not marked.
                slots.fill(0);
                chunks.fill(0);
                let below = heap.add(0x1000);
                hooked(0x20, below, record.as_ptr(), scratch.as_ptr().cast_mut());
                assert_eq!(std::slice::from_raw_parts(below.add(0x20), 32), &record);
                assert!(slots.iter().chain(&chunks).all(|&b| b == 0));
                drop(marks);
            }
        }
    }

    #[test]
    fn other_bytes_are_left_alone() {
        use windows_sys::Win32::System::Memory::{MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READWRITE, VirtualAlloc};
        // SAFETY: fresh memory of our own.
        unsafe {
            let place = VirtualAlloc(std::ptr::null(), 0x10000, MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE) as *mut u8;
            let mut bytes = SITES[0].bytes.to_vec();
            bytes[3] = 0x7e; // and ecx, 7EE0h
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), place.add(0x100), bytes.len());
            let mine = [Site { window: 0x100, ..SITES[0] }];
            assert_eq!(apply(place as usize, 0x10000, 0, &mine), [Hooked::Differs]);
            assert_eq!(std::slice::from_raw_parts(place.add(0x100), bytes.len()), &bytes[..]);
            let outside = [Site { window: 0xfff0, ..SITES[0] }];
            assert!(matches!(apply(place as usize, 0x10000, 0, &outside)[..], [Hooked::Failed(_)]));
        }
    }

    /// The range sites (issue #301), each in code of a test's, plain and
    /// hooked, entered at its hooked instructions with every register set:
    /// the window leaves the same registers both ways, and every slot a table
    /// overlaps is marked (half into a slot too, cut at the heap's end), none
    /// for a table outside the heap or of no bytes. A window's `rip`-relative
    /// loads are pointed at the test's own memory.
    #[test]
    fn a_range_site_marks_every_slot_its_table_overlaps() {
        use windows_sys::Win32::System::Memory::{MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READWRITE, VirtualAlloc};
        let ranges: Vec<&Site> = SITES.iter().filter(|s| matches!(s.kind, Kind::Range { .. })).collect();
        assert_eq!(ranges.len(), 4);
        for site in ranges {
            let Kind::Range { table, size, moved } = site.kind else { unreachable!() };
            let mut bytes = site.bytes.to_vec();
            if moved == Moved::LoadEax {
                // mov eax, [rip+...] (hooked) and the mov r8, [rip+...] after
                // it: to the window's start + 0xc00 and + 0xc08.
                let (eax, r8) = (site.store, site.store + 6);
                assert_eq!(&bytes[eax..eax + 2], &[0x8b, 0x05]);
                assert_eq!(&bytes[r8..r8 + 3], &[0x4c, 0x8b, 0x05]);
                bytes[eax + 2..eax + 6].copy_from_slice(&(0xc00 - (eax as i32 + 6)).to_le_bytes());
                bytes[r8 + 3..r8 + 7].copy_from_slice(&(0xc08 - (r8 as i32 + 7)).to_le_bytes());
            }
            let bytes: &'static [u8] = Box::leak(bytes.into_boxed_slice());
            // fn(table: rcx, size: rdx, out: r8): every register but rsp set,
            // the table's and the size's from the arguments, into the
            // window at its hooked instructions; every register out after it.
            let mut code: Vec<u8> = vec![0x53, 0x55, 0x56, 0x57, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56, 0x41, 0x57]; // push rbx, rbp, rsi, rdi, r12-r15
            code.extend([0x48, 0x83, 0xec, 0x48]); // sub rsp, 48h
            code.extend([0x4c, 0x89, 0x44, 0x24, 0x40]); // mov [rsp+40h], r8
            code.extend(mov(table, 1));
            code.extend(mov(size, 2));
            for reg in (0u8..16).filter(|r| ![4, table, size].contains(r)) {
                code.extend([0x48 | reg >> 3, 0xb8 | (reg & 7)]); // mov reg, imm64
                code.extend((0x0101_0101_0101_0101u64 * (u64::from(reg) + 1)).to_le_bytes());
            }
            code.push(0xe9); // jmp to the hooked instructions
            code.extend((site.store as i32).to_le_bytes());
            let at = code.len();
            code.extend(bytes);
            code.push(0x50); // push rax
            code.extend([0x48, 0x8b, 0x44, 0x24, 0x48]); // mov rax, [rsp+48h]: out
            for reg in 1u8..16 {
                code.extend([0x48 | (reg >> 3) << 2, 0x89, 0x40 | (reg & 7) << 3, 8 * reg]); // mov [rax+8*reg], reg
            }
            code.push(0x59); // pop rcx: rax
            code.extend([0x48, 0x89, 0x08]); // mov [rax], rcx
            code.extend([0x48, 0x83, 0xc4, 0x48]); // add rsp, 48h
            code.extend([0x41, 0x5f, 0x41, 0x5e, 0x41, 0x5d, 0x41, 0x5c, 0x5f, 0x5e, 0x5d, 0x5b, 0xc3]); // pop r15-r12, rdi, rsi, rbp, rbx ; ret
            type F = unsafe extern "system" fn(u64, u64, *mut [u64; 16]);
            // SAFETY: fresh memory for code and data, one copy changed before it runs.
            unsafe {
                let place = VirtualAlloc(std::ptr::null(), 0x10000, MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE) as usize;
                assert_ne!(place, 0);
                let (plain, hooked) = (place + 0x100 - at, place + 0x1100 - at);
                std::ptr::copy_nonoverlapping(code.as_ptr(), plain as *mut u8, code.len());
                std::ptr::copy_nonoverlapping(code.as_ptr(), hooked as *mut u8, code.len());
                for window in [place + 0x100, place + 0x1100] {
                    ((window + 0xc00) as *mut u32).write(0xdead_beef);
                    ((window + 0xc08) as *mut u64).write(0x0123_4567_89ab_cdef);
                }
                let (mut slots, mut chunks) = (vec![0u8; 1024], vec![0u8; 1024 / 128]);
                let start = 0x7_0000_2000u64;
                let marks = Box::new(Marks { start, len: 1024 * 32, slots: slots.as_mut_ptr(), chunks: chunks.as_mut_ptr() });
                let mine = [Site { window: 0x1100, bytes, ..*site }];
                assert_eq!(apply(place, 0x10000, &raw const *marks as usize, &mine), [Hooked::Done]);
                assert_eq!(*((place + 0x1100 + site.store) as *const u8), 0xe9);
                let (plain, hooked) = (std::mem::transmute::<usize, F>(plain), std::mem::transmute::<usize, F>(hooked));
                let mut run = |at: u64, n: u64| -> (Vec<usize>, Vec<usize>) {
                    slots.fill(0);
                    chunks.fill(0);
                    let (mut a, mut b) = ([0u64; 16], [0u64; 16]);
                    hooked(at, n, &mut a);
                    let set = |m: &[u8]| m.iter().enumerate().filter(|(_, b)| **b != 0).map(|(i, _)| i).collect();
                    let marked = (set(&slots), set(&chunks));
                    plain(at, n, &mut b);
                    assert_eq!(a, b, "the registers the window leaves, at {:#x}", site.window);
                    if moved == Moved::LoadEax {
                        assert_eq!(a[0], 0xdead_beef, "rax from the load");
                    }
                    marked
                };
                // Half into slot 0x40: 128 bytes overlap five slots.
                assert_eq!(run(start + 0x40 * 32 + 16, 128), (vec![0x40, 0x41, 0x42, 0x43, 0x44], vec![0]));
                // On a slot: three slots, in chunk 2.
                assert_eq!(run(start + 0x100 * 32, 0x60), (vec![0x100, 0x101, 0x102], vec![2]));
                // Over the heap's end: its last slot only.
                assert_eq!(run(start + 1023 * 32, 128), (vec![1023], vec![7]));
                // No bytes, before the heap, after it: nothing.
                assert_eq!(run(start + 0x40 * 32, 0), (vec![], vec![]));
                assert_eq!(run(start - 64, 128), (vec![], vec![]));
                assert_eq!(run(start + 1024 * 32, 128), (vec![], vec![]));
                drop(marks);
            }
        }
    }

    /// `mov eax, [rip+rel]` done from elsewhere loads from the same address.
    #[test]
    fn a_moved_load_reads_the_same_address() {
        let code = moved_code(Moved::LoadEax, &[0x8b, 0x05, 0xf0, 0xff, 0xff, 0xff], 0x1000).unwrap();
        assert_eq!(&code[..2], &[0x48, 0xb8]);
        assert_eq!(u64::from_le_bytes(code[2..10].try_into().unwrap()), 0x1000 + 6 - 0x10);
        assert_eq!(&code[10..], &[0x8b, 0x00]);
        assert!(moved_code(Moved::LoadEax, &[0x48, 0x89, 0xf1, 0x89, 0xfa, 0x90], 0x1000).is_none());
        assert_eq!(moved_code(Moved::AsIs, &[0x48, 0x89, 0xf1], 0x1000).unwrap(), [0x48, 0x89, 0xf1]);
    }

    /// A range site's registers may not be the ones its code uses.
    #[test]
    fn a_range_stub_refuses_the_registers_it_uses() {
        for reg in [0, 1, 2, 4, 8, 16] {
            assert!(range_stub(reg, 14, &[0x90; 5], 0x1000, 0x2000, 0).is_none());
            assert!(range_stub(3, reg, &[0x90; 5], 0x1000, 0x2000, 0).is_none());
        }
        assert!(range_stub(3, 14, &[0x90; 5], 0x1000, 0x2000, 0).is_some());
    }
}
