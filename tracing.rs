#[allow(dead_code)];

//! Precise GC on the heap.
//!
//! Very slow.

use ffi;
use ffi::GC_word;
use std::{mem, vec, libc, cell};
use std::unstable::intrinsics;

// macros from gc_typed.h

/// The size of the words understood by the GC, in bits.
#[inline]
pub fn GC_WORDSZ() -> uint { 8 * mem::size_of::<GC_word>() }

fn GC_get_bit(bm: &[GC_word], index: uint) -> bool {
    let wrd_sz = GC_WORDSZ();
    ((bm[index / wrd_sz] >> (index % wrd_sz)) & 1) == 1
}
fn GC_set_bit(bm: &mut [GC_word], index: uint) {
    let wrd_sz = GC_WORDSZ();
    bm[index / wrd_sz] |= 1 << (index % wrd_sz);
}
fn GC_WORD_LEN<T>() -> uint { mem::size_of::<T>() / mem::size_of::<GC_word>() }

fn GC_BITMAP_SIZE<T>() -> uint { (GC_WORD_LEN::<T>() + GC_WORDSZ() - 1) / GC_WORDSZ() }

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
                if is_ptr { GC_set_bit(compressed, word_idx) }
            }
            unsafe {
                ffi::GC_make_descriptor(compressed.as_mut_ptr(), l as GC_word)
            }
        } }
    );

    if l < wrd_sz * 2 {
        go!([0 as GC_word, .. 2])
    } else {
        go!(vec::from_elem((l + wrd_sz - 1) / wrd_sz, 0 as GC_word))
    }
}

/// A pointer that uses type information to inform the GC about what
/// things could possibly be pointers, and what can just be ignored.
///
/// That is, run Boehm in precise-on-the-heap mode.
#[no_send]
#[deriving(Clone)]
pub struct GcTracing<T> {
    priv ptr: *mut T,
    //priv force_managed: Option<@()>
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
                ffi::GC_debug_malloc(size, bytes!("GcTracing", 0).as_ptr() as *i8, 0)
            } else {
                ffi::GC_malloc_explicitly_typed(size,
                                                BoehmTraced::get_tracing_descr(None::<T>))
            } as *mut T;

            if p.is_null() {
                fail!("Could not allocate")
            }
            intrinsics::move_val_init(&mut *p, value);
            GcTracing {
                ptr: p,
                //force_managed: None
            }
        }
    }

    #[inline]
    pub fn borrow<'r>(&'r self) -> &'r T {
        unsafe {
            &*self.ptr
        }
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
pub trait BoehmTraced {
    /// Construct the `GC_descr` of `Self`. This should not be
    /// overriden.
    fn get_tracing_descr(dummy: Option<Self>) -> ffi::GC_descr {
        let sz = mem::size_of::<Self>() * 8;
        let wrd_sz = GC_WORDSZ();
        let num_words = sz / wrd_sz;

        if num_words < 16 {
            let mut vec = [false, .. 16];
            BoehmTraced::indicate_ptr_words(dummy, vec);
            make_descriptor(vec.slice_to(num_words))
        } else {
            let mut vec = vec::from_elem(num_words, false);
            BoehmTraced::indicate_ptr_words(dummy, vec);
            make_descriptor(vec)
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

impl<T> BoehmTraced for GcTracing<T> {
    #[inline]
    fn indicate_ptr_words(_dummy: Option<GcTracing<T>>, words: &mut [bool]) {
        // GcTracing is one word, and is (clearly) a pointer relevant
        // to the GC.
        words[0] = true;
    }
}

// things that aren't pointers at all
macro_rules! no_ptr {
    ($($t:ty),*) => {
        $(
            impl BoehmTraced for $t {
                // no words are pointers
                #[inline]
                fn indicate_ptr_words(_: Option<$t>, _: &mut [bool]) {}
            }
            )*
    }
}

no_ptr! {
    int, i8, i16, i32, i64,
    uint, u8, u16, u32, u64,

    f32, f64,

    ()
}

// paradoxically, these don't count as having GC pointer words.
impl<T> BoehmTraced for *T {
    #[inline]
    fn indicate_ptr_words(_: Option<*T>, _: &mut [bool]) {}
}
impl<T> BoehmTraced for *mut T {
    #[inline]
    fn indicate_ptr_words(_: Option<*mut T>, _: &mut [bool]) {}
}

// for interior mutability
impl<T: BoehmTraced> BoehmTraced for cell::RefCell<T> {
    #[inline]
    fn indicate_ptr_words(_dummy: Option<cell::RefCell<T>>, words: &mut [bool]) {
        let l = words.len();
        // the last word is not a pointer, and is not part of the `T`.
        BoehmTraced::indicate_ptr_words(None::<T>, words.mut_slice_to(l - 1));
    }
}

// likely incorrect
impl<T: BoehmTraced> BoehmTraced for Option<T> {
    #[inline]
    fn indicate_ptr_words(_dummy: Option<Option<T>>, words: &mut [bool]) {
        // what's this "parametric polymorphism" thing? ;)
        let discr_size = mem::size_of::<Option<T>>() - mem::size_of::<T>();

        if discr_size * 8 >= GC_WORDSZ() {
            // we have a proper discriminant, so T might contain pointers
            BoehmTraced::indicate_ptr_words(None::<T>, words.mut_slice_from(1))
        } else {
            // we don't have a big discriminant, so we're either a
            // nullable pointer, or a small non-word aligned type. (In
            // the latter case, we don't contain any pointers so we
            // could probably actually elide this call... but we'll
            // just let the optimiser do that.)
            BoehmTraced::indicate_ptr_words(None::<T>, words)
        }
    }
}

// impls for fixed length vectors for a selection of lengths

macro_rules! fixedvec {
    ($($n:expr),*) => {
        $(
            impl<T: BoehmTraced> BoehmTraced for [T, .. $n] {
                fn indicate_ptr_words(_dummy: Option<[T, .. $n]>, words: &mut [bool]) {
                    if $n == 0 { return }

                    let bits_per_step = 8 * mem::size_of::<[T, .. $n]>() / $n;
                    let words_per_step = bits_per_step / GC_WORDSZ();
                    if words_per_step > 0 {
                        for chunk in words.mut_slice_to(words_per_step * $n)
                            .mut_chunks(words_per_step) {
                            BoehmTraced::indicate_ptr_words(None::<T>, chunk)
                        }
                    }
                }
            }
            )*
    }
}

macro_rules! fixedvec_lots {
    (; $($n:tt),*) => { fixedvec!($($n),*) };
    ([$e:expr] $([$x:expr])* ; $($n:tt),*) => {
        // binary expansion
        fixedvec_lots!($([$x])* ; $( (2 * $n + 1), (2 * $n) ),*)
    }
}


// generate tracing info for all the short fixed length vectors.
// NB. this crashes rustdoc.
fixedvec_lots!([1] [2] [4] [16] [32] [64]; 0)
// and some long ones
fixedvec!(100, 1000, 10_000, 100_000, 1_000_000)
