extern crate boehm;
use boehm::{init, Gc, heap_size};

#[start]
fn main(_: int, _: **u8) -> int {
    init();

    for i in range(0, 10000000) {
        Gc::new(i);
        if i % 100000 == 0 {
            println!("Heap size = {}", heap_size());
        }
    }
    0
}
