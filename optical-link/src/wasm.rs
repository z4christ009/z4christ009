//! Raw C ABI for the browser build.
//!
//! Deliberately no wasm-bindgen: the surface here is small enough that a plain
//! `extern "C"` boundary over linear memory is less machinery than a codegen
//! dependency, and it keeps exactly what crosses into JS visible in one file.
//!
//! Ownership contract, since nothing here is checked for you:
//!   * `alloc`/`dealloc` bracket every buffer JS hands in or reads out.
//!   * `sender_new`/`receiver_new` return opaque handles that must be released
//!     with the matching `_free`. They are leaked `Box`es, not indices.
//!   * Pointers returned by `*_result_ptr` borrow from the handle and are
//!     invalidated by any further call on it.
//!
//! Every entry point null-checks its handle, so a JS bug surfaces as a no-op
//! rather than a trap into undefined behaviour.

#![cfg(target_arch = "wasm32")]

use crate::config::LinkConfig;
use crate::geometry::{self, Image};
use crate::{Receiver, Sender};

fn preset(id: u32) -> LinkConfig {
    match id {
        0 => LinkConfig::robust(),
        2 => LinkConfig::fast(),
        _ => LinkConfig::balanced(),
    }
}

// ---------------------------------------------------------------------------
// Memory
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    let mut buf = Vec::<u8>::with_capacity(len);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

/// # Safety
/// `ptr` must come from [`alloc`] with the same `len`.
#[no_mangle]
pub unsafe extern "C" fn dealloc(ptr: *mut u8, len: usize) {
    if !ptr.is_null() {
        drop(Vec::from_raw_parts(ptr, 0, len));
    }
}

// ---------------------------------------------------------------------------
// Sender
// ---------------------------------------------------------------------------

pub struct TxState {
    cfg: LinkConfig,
    tx: Sender,
    frame: Vec<u8>,
}

/// # Safety
/// `data` and `name` must point to readable buffers of the given lengths.
#[no_mangle]
pub unsafe extern "C" fn sender_new(
    preset_id: u32,
    data: *const u8,
    data_len: usize,
    name: *const u8,
    name_len: usize,
) -> *mut TxState {
    if data.is_null() {
        return std::ptr::null_mut();
    }
    let cfg = preset(preset_id);
    let bytes = std::slice::from_raw_parts(data, data_len);
    let label = if name.is_null() || name_len == 0 {
        String::from("payload.bin")
    } else {
        String::from_utf8_lossy(std::slice::from_raw_parts(name, name_len)).into_owned()
    };

    // Session id from the payload so a reload mid-transfer looks like a new
    // session to any receiver still watching the old one.
    let session = (crate::frame::crc32(bytes) & 0xFFFF) as u16;

    let state = TxState {
        cfg,
        tx: Sender::new(cfg, bytes, &label, session),
        frame: vec![0u8; cfg.grid * cfg.grid * 4],
    };
    Box::into_raw(Box::new(state))
}

/// # Safety
/// `s` must be a live handle from [`sender_new`].
#[no_mangle]
pub unsafe extern "C" fn sender_free(s: *mut TxState) {
    if !s.is_null() {
        drop(Box::from_raw(s));
    }
}

#[no_mangle]
pub unsafe extern "C" fn sender_grid(s: *mut TxState) -> u32 {
    if s.is_null() {
        return 0;
    }
    (*s).cfg.grid as u32
}

/// Bytes of source payload each frame carries -- used for the sender's ETA.
#[no_mangle]
pub unsafe extern "C" fn sender_symbol_size(s: *mut TxState) -> u32 {
    if s.is_null() {
        return 0;
    }
    (*s).cfg.symbol_size() as u32
}

#[no_mangle]
pub unsafe extern "C" fn sender_frames(s: *mut TxState) -> u32 {
    if s.is_null() {
        return 0;
    }
    (*s).tx.frames_emitted() as u32
}

/// Render the next frame into the handle's internal RGBA buffer and return a
/// pointer to it. `grid * grid * 4` bytes, valid until the next call.
#[no_mangle]
pub unsafe extern "C" fn sender_next_frame(s: *mut TxState) -> *const u8 {
    if s.is_null() {
        return std::ptr::null();
    }
    let st = &mut *s;
    let symbols = st.tx.next_frame();
    geometry::render_rgba(&st.cfg, &symbols, &mut st.frame);
    st.frame.as_ptr()
}

// ---------------------------------------------------------------------------
// Receiver
// ---------------------------------------------------------------------------

pub const STATUS_LOCKED: u32 = 1 << 0;
pub const STATUS_VALID: u32 = 1 << 1;
pub const STATUS_COMPLETE: u32 = 1 << 2;

pub struct RxState {
    cfg: LinkConfig,
    rx: Receiver,
    out: Option<Vec<u8>>,
    name: String,
}

#[no_mangle]
pub extern "C" fn receiver_new(preset_id: u32) -> *mut RxState {
    let cfg = preset(preset_id);
    Box::into_raw(Box::new(RxState {
        cfg,
        rx: Receiver::new(cfg),
        out: None,
        name: String::new(),
    }))
}

/// # Safety
/// `r` must be a live handle from [`receiver_new`].
#[no_mangle]
pub unsafe extern "C" fn receiver_free(r: *mut RxState) {
    if !r.is_null() {
        drop(Box::from_raw(r));
    }
}

/// Feed one camera frame as RGBA. Returns a bitfield of `STATUS_*`.
///
/// # Safety
/// `rgba` must point to at least `w * h * 4` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn receiver_push(
    r: *mut RxState,
    rgba: *const u8,
    w: usize,
    h: usize,
) -> u32 {
    if r.is_null() || rgba.is_null() || w == 0 || h == 0 {
        return 0;
    }
    let st = &mut *r;
    if st.out.is_some() {
        return STATUS_COMPLETE;
    }

    let before_locked = st.rx.stats.frames_locked;
    let before_valid = st.rx.stats.frames_valid;

    let img = Image::from_rgba(std::slice::from_raw_parts(rgba, w * h * 4), w, h);
    let factor = geometry::detect_factor(w, h);
    let done = st.rx.push_image_with_factor(&img, factor);

    let mut status = 0;
    if st.rx.stats.frames_locked > before_locked {
        status |= STATUS_LOCKED;
    }
    if st.rx.stats.frames_valid > before_valid {
        status |= STATUS_VALID;
    }
    if let Some(d) = done {
        st.name = st
            .rx
            .manifest()
            .map(|m| m.name.clone())
            .unwrap_or_else(|| "received.bin".into());
        st.out = Some(d);
        status |= STATUS_COMPLETE;
    }
    status
}

/// Stats snapshot. Writes 11 `u32`s to `out`:
/// seen, locked, valid, rejected, ecc_corrected, ecc_uncorrectable, manifests,
/// symbols_accepted, total_len, symbols_needed, complete.
///
/// # Safety
/// `out` must point to at least 11 writable `u32`s.
#[no_mangle]
pub unsafe extern "C" fn receiver_stats(r: *mut RxState, out: *mut u32) {
    if r.is_null() || out.is_null() {
        return;
    }
    let st = &*r;
    let s = st.rx.stats;
    let total = st.rx.manifest().map(|m| m.total_len).unwrap_or(0);
    let symbol = st.cfg.symbol_size() as u32;
    let needed = if total > 0 && symbol > 0 {
        total.div_ceil(symbol)
    } else {
        0
    };

    let vals = [
        s.frames_seen as u32,
        s.frames_locked as u32,
        s.frames_valid as u32,
        s.frames_rejected as u32,
        s.ecc_corrected_shards as u32,
        s.ecc_uncorrectable_shards as u32,
        s.manifests_seen as u32,
        s.symbols_accepted as u32,
        total,
        needed,
        st.out.is_some() as u32,
    ];
    std::ptr::copy_nonoverlapping(vals.as_ptr(), out, vals.len());
}

#[no_mangle]
pub unsafe extern "C" fn receiver_result_len(r: *mut RxState) -> usize {
    if r.is_null() {
        return 0;
    }
    (*r).out.as_ref().map(|v| v.len()).unwrap_or(0)
}

/// Borrows from the handle; invalidated by any further call on it.
#[no_mangle]
pub unsafe extern "C" fn receiver_result_ptr(r: *mut RxState) -> *const u8 {
    if r.is_null() {
        return std::ptr::null();
    }
    (*r).out
        .as_ref()
        .map(|v| v.as_ptr())
        .unwrap_or(std::ptr::null())
}

#[no_mangle]
pub unsafe extern "C" fn receiver_name_len(r: *mut RxState) -> usize {
    if r.is_null() {
        return 0;
    }
    (&*r).name.len()
}

#[no_mangle]
pub unsafe extern "C" fn receiver_name_ptr(r: *mut RxState) -> *const u8 {
    if r.is_null() {
        return std::ptr::null();
    }
    (&*r).name.as_ptr()
}
