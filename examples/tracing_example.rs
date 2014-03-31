extern crate boehm;

use std::mem;
use std::cell::RefCell;

// allow comparison between the precise and conservative modes.
#[cfg(not(conservative))]
use Gc = boehm::tracing::GcTracing;
#[cfg(conservative)]
use boehm::Gc;

static SIZE: uint = 100_000;

#[start]
fn main(_: int, _: **u8) -> int {
    boehm::init();

    // allocate a pile of pointers (we have to use a stack vector
    // rather than a ~[] because ~[] isn't a Gc root (yet))
    let mut ptrs = [None::<Gc<uint>>, .. SIZE];
    for (i, ptr) in ptrs.mut_iter().enumerate() {
        *ptr = Some(Gc::new(i));
    }

    // now we take all those points and convert them to `uint`s. These
    // have the same bitpattern as the pointers, and so a conservative
    // collector would have to assume that these are valid references
    // and thus not collect the allocations above (if these were the
    // last references).

    // Since Boehm is conservative on the stack (even in gc_typed.h
    // mode), we place the integers inside a Gc so that it can be
    // traced precisely.
    let uint_ptrs = Gc::new(RefCell::new([0u, .. SIZE]));
    let mut cell = uint_ptrs.borrow().borrow_mut();
    for (uint, ptr) in cell.mut_iter().zip(ptrs.iter()) {
        *uint = ptr.unwrap().borrow() as *uint as uint;
    }

    // this should do nothing, since we have one or two references to
    // each Gc (for precise vs conservative).
    boehm::collect();

    // will list a pile of allocations, i.e. the `ptrs`.
    boehm::debug_dump();

    // clear the on-stack pointers that we used to allocate with
    // originally (set_memory rather than just writing `None`s, since
    // the latter could just set the discriminant to 0, and not modify
    // the actuall pointer bit, which would leave the values on the
    // stack and confuse Boehm).
    unsafe {
        std::ptr::set_memory(ptrs.as_mut_ptr() as *mut u8, 0, mem::size_of_val(&ptrs));
    }

    // now `uint_ptrs` are the only "reference" to the first lot of
    // allocated objects, so, in precise mode, this collection should
    // eat them all. (In conservative mode the uint_ptrs look like
    // pointers and so the collection of pointers will not be
    // collected.)
    boehm::collect();
    // in conservative mode all of the allocationed chunks from above
    // are still listed; in precise mode, very few (and one of which
    // is the `uint_ptrs` allocation anyway).
    boehm::debug_dump();

    0
}
