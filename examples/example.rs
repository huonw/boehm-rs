#![feature(start)]
extern crate boehm;
use boehm::{init, Gc, heap_size};

#[start]
fn main(_: isize, _: *const *const u8) -> isize {
    init();

    for i in 0..10000000 {
        Gc::new(i);
        if i % 100000 == 0 {
            println!("Heap size = {}", heap_size());
        }
    }
    0
}
