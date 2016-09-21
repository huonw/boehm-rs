#![allow(dead_code)]
#![allow(non_snake_case)]

//! Precise GC on the heap.
//!
//! Very slow.

use libc;

use ffi;
use ffi::GC_word;
use std::mem;
use std::intrinsics;

// macros from gc_typed.h

/// The size of the words understood by the GC, in bits.
#[inline]
pub fn GC_WORDSZ() -> usize {
    8 * mem::size_of::<GC_word>()
}

fn GC_get_bit(bm: &[GC_word], index: usize) -> bool {
    let wrd_sz = GC_WORDSZ();
    ((bm[index / wrd_sz] >> (index % wrd_sz)) & 1) == 1
}
fn GC_set_bit(bm: &mut [GC_word], index: usize) {
    let wrd_sz = GC_WORDSZ();
    bm[index / wrd_sz] |= 1 << (index % wrd_sz);
}
fn GC_WORD_LEN<T>() -> usize {
    mem::size_of::<T>() / mem::size_of::<GC_word>()
}

fn GC_BITMAP_SIZE<T>() -> usize {
    (GC_WORD_LEN::<T>() + GC_WORDSZ() - 1) / GC_WORDSZ()
}

/// Construct a tracing descriptor out of the `bitmap`, which should
/// be true for each word that is possibly a pointer.
pub fn make_descriptor(bitmap: &[bool]) -> ffi::GC_descr {
    // TODO, should make sure `bm` is long enough
    let wrd_sz = GC_WORDSZ();
    let l = bitmap.len();
    macro_rules! go (
        ($cmprs:expr) => { {
            let mut compressed = $cmprs;
            for (word_idx, &is_ptr) in bitmap.iter().enumerate() {
                if is_ptr { GC_set_bit(&mut compressed, word_idx) }
            }
            unsafe {
                ffi::GC_make_descriptor(compressed.as_mut_ptr(), l)
            }
        } }
    );

    if l < wrd_sz * 2 {
        go!([0 as GC_word; 2])
    } else {
        go!(vec![0 as GC_word; (l + wrd_sz - 1) / wrd_sz])
    }
}

/// A pointer that uses type information to inform the GC about what
/// things could possibly be pointers, and what can just be ignored.
///
/// That is, run Boehm in precise-on-the-heap mode.
#[derive(Clone, Copy)]
pub struct GcTracing<T> {
    ptr: *mut T,
}

impl<T: BoehmTraced> GcTracing<T> {
    /// Create a new GcTracing.
    ///
    /// NB. this extracts the type information at runtime, for each
    /// allocation, and so is quite slow.
    ///
    /// TODO: fix that (requires compiler hooks)
    pub fn new(value: T) -> GcTracing<T> {
        unsafe {
            let size = mem::size_of::<T>() as libc::size_t;

            let p = if cfg!(debug) {
                ffi::GC_debug_malloc(size, b"GcTracing\x00".as_ptr() as *const i8, 0)
            } else {
                ffi::GC_malloc_explicitly_typed(size, BoehmTraced::get_tracing_descr(None::<T>))
            } as *mut T;

            if p.is_null() {
                panic!("Could not allocate")
            }
            intrinsics::move_val_init(&mut *p, value);
            GcTracing { ptr: p }
        }
    }

    #[inline]
    pub fn borrow<'r>(&'r self) -> &'r T {
        unsafe { &*self.ptr }
    }
}

/// Values that the precise-on-heap Boehm collector can understand.
///
/// This trait is a stop-gap until the compiler itself can generate
/// such information, since writing these by hand is annoying, and
/// nearly impossible to get correct without dirty hacks to find
/// alignment of fields and extract (for example) the enum
/// optimisation that have occurred (and even then, they're likely to
/// no be correct).
pub trait BoehmTraced: Sized {
    /// Construct the `GC_descr` of `Self`. This should not be
    /// overriden.
    fn get_tracing_descr(dummy: Option<Self>) -> ffi::GC_descr {
        let sz = mem::size_of::<Self>() * 8;
        let wrd_sz = GC_WORDSZ();
        let num_words = sz / wrd_sz;

        if num_words < 16 {
            let mut vec = [false; 16];
            BoehmTraced::indicate_ptr_words(dummy, &mut vec);
            make_descriptor(&vec[..num_words])
        } else {
            let mut vec = vec![false; num_words];
            BoehmTraced::indicate_ptr_words(dummy, vec.as_mut_slice());
            make_descriptor(&vec[..])
        }
    }

    /// Mark which words within `Self` can possibly hold relevant
    /// pointers (do *not* explicitly mark which words are not
    /// pointers).
    ///
    /// E.g. `struct Foo { x: uint, y: GcTracing<uint>, z:
    /// GcTracing<uint> }` should explicitly set `words[1]` and
    /// `words[2]` to `true` but leave `words[0]` untouched.
    ///
    /// As long as `get_tracing_descr` is not overridden,
    /// `words` is guaranteed to be large enough to hold all the words
    /// in the current type.
    fn indicate_ptr_words(_dummy: Option<Self>, words: &mut [bool]);
}

// no-one needs to see the hacks.
mod boehm_traced_impls;
