#![crate_id="boehm#0.1"]
#![crate_type="rlib"]
#![feature(globs, macro_rules)]

use std::{libc, mem};
use std::kinds::marker;
use std::intrinsics;

#[allow(dead_code)]
#[allow(non_camel_case_types)]
pub mod ffi;

pub mod tracing;

/// Initialise the GC. This should be called before using any other
/// functions and on the main thread for maximum portability (some
/// platforms don't require this to be called at all).
///
/// FIXME: initialise automagically somehow
/// FIXME: this should be doing the full equivalent of the GC_INIT()
/// macro.
pub fn init() {
    unsafe { ffi::GC_init(); }
}

/// Number of bytes in the garbage collection heap.
pub fn heap_size() -> uint {
    unsafe { ffi::GC_get_heap_size() as uint }
}

/// Force a garbage collection.
pub fn collect() {
    unsafe { ffi::GC_gcollect(); }
}

/// Dump some debugging/diagnostic information to stdout.
pub fn debug_dump() {
    unsafe { ffi::GC_dump(); }
}

/// A garbage collected pointer.
#[deriving(Clone)]
pub struct Gc<T> {
    ptr: *mut T,
    mark: marker::NoSend
}

impl<T: 'static> Gc<T> {
    pub fn new(value: T) -> Gc<T> {
        unsafe {
            let size = mem::size_of::<T>() as libc::size_t;
            let p = if cfg!(debug) {
                ffi::GC_debug_malloc(size, bytes!("Gc", 0).as_ptr() as *i8, 0)
            } else {
                ffi::GC_malloc(size)
            } as *mut T;
            if p.is_null() {
                fail!("Could not allocate")
            }
            intrinsics::move_val_init(&mut *p, value);
            Gc { ptr: p, mark: marker::NoSend }
        }
    }

    pub fn borrow<'r>(&'r self) -> &'r T {
        unsafe {
            &*self.ptr
        }
    }
}
