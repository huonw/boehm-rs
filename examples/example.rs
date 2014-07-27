extern crate boehm = "boehm-rs";
use boehm::{init, Gc, heap_size};

#[start]
fn main(_: int, _: *const *const u8) -> int {
    init();

    for i in range(0u, 10000000) {
        Gc::new(i);
        if i % 100000 == 0 {
            println!("Heap size = {}", heap_size());
        }
    }
    0
}
