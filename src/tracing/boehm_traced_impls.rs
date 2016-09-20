#![doc(hidden)]
use std::{mem, cell};
use tracing::{BoehmTraced, GcTracing, GC_WORDSZ};

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
    isize, i8, i16, i32, i64,
    usize, u8, u16, u32, u64,

    f32, f64,

    ()
}

// paradoxically, these don't count as having GC pointer words.
impl<T> BoehmTraced for *const T {
    #[inline]
    fn indicate_ptr_words(_: Option<*const T>, _: &mut [bool]) {}
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
        BoehmTraced::indicate_ptr_words(None::<T>, &mut words[..(l - 1)]);
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
            BoehmTraced::indicate_ptr_words(None::<T>, &mut words[1..])
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
            impl<T: BoehmTraced> BoehmTraced for [T; $n] {
                fn indicate_ptr_words(_dummy: Option<[T; $n]>, words: &mut [bool]) {
                    if $n == 0 { return }

                    let bits_per_step = 8 * mem::size_of::<[T; $n]>() / $n;
                    let words_per_step = bits_per_step / GC_WORDSZ();
                    if words_per_step > 0 {
                        for chunk in &mut words[..words_per_step * $n]
                            .chunks_mut(words_per_step) {
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
        fixedvec_lots!($([$x])* ; $( (2usize * $n + 1usize), (2usize * $n) ),*)
    }
}


// generate tracing info for all the short fixed length vectors.
// NB. this crashes rustdoc.
// fixedvec_lots!([1u] [2u] [4u] [16u] [32u] [64u]; 0)
// and some long ones
fixedvec!(100usize,
          1000usize,
          10_000usize,
          100_000usize,
          1_000_000usize);
