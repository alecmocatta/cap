# cap

[![Crates.io](https://img.shields.io/crates/v/cap.svg?maxAge=86400)](https://crates.io/crates/cap)
[![MIT / Apache 2.0 licensed](https://img.shields.io/crates/l/cap.svg?maxAge=2592000)](#License)
[![Build Status](https://dev.azure.com/alecmocatta/cap/_apis/build/status/tests?branchName=master)](https://dev.azure.com/alecmocatta/cap/_build/latest?branchName=master)

[Docs](https://docs.rs/cap/0.1)

An allocator that can track and limit memory usage.

This crate provides a generic allocator that wraps another allocator, tracking memory usage and enabling limits to be set.

## Example

It can be used by declaring a static and marking it with the `#[global_allocator]` attribute:

```rust
use std::alloc;
use cap::Cap;

#[global_allocator]
static ALLOCATOR: Cap<alloc::System> = Cap::new(alloc::System, usize::max_value());

fn main() {
    // Set the limit to 30MiB.
    ALLOCATOR.set_limit(30 * 1024 * 1024).unwrap();
    // ...
    println!("Currently allocated: {}B", ALLOCATOR.allocated());
}
```

## License
Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE.txt](LICENSE-APACHE.txt) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT.txt](LICENSE-MIT.txt) or http://opensource.org/licenses/MIT)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
