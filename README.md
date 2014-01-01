# Using the Boehm GC from Rust

[![Build Status](https://travis-ci.org/huonw/boehm-rs.png)](https://travis-ci.org/huonw/boehm-rs)

A very basic wrapper that provides a `Gc<T>` type, implemented by
binding to the
[Boehm-Demers-Weiser garbage collector](http://www.hpl.hp.com/personal/Hans_Boehm/gc/). See
`example.rs` and `lowlevel_example.rs` for some examples.

## Warning

This is not correct and shouldn't be used/should only be used very
carefully, because I don't think the Rust compiler provides enough
hooks yet to make something like `~[Gc<T>]` completely work.

To illustrate, the following program is trying to make a vector
`~[Gc 0, Gc 1, ..., Gc 1_000_000]`, so it should print `0`... but it
prints `895221` for me; the vector doesn't act as a root that Boehm
can understand, and so it's free to GC the first one and reuse that
memory for one of the later allocations. Oops.

```rust
extern mod boehm;

#[start]
fn main(_: int, _: **u8) -> int {
    boehm::init();

    let mut v = std::vec::from_fn(1_000_000, boehm::Gc::new);
    println!("{}", *v[0].borrow());

    0
}
```

I think this could be somewhat fixed with some trickery with
`#[no_std]` and `#[lang="malloc"]` and so on, but that's beyond the
time I've been able to allocate (no pun intended) to this so far.

## Todo

- Fix the above
- Use
  [the typed inferface](http://www.hpl.hp.com/personal/Hans_Boehm/gc/gc_source/gc_typedh.txt)
  for more precise collection (at the very least, working out a way to
  use malloc_atomic where appropriate would be good)

## License

Dual Apache v2.0 and MIT, like Rust itself.
