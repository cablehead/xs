# ansi-str [![Build Status](https://github.com/zhiburt/ansi-str/actions/workflows/ci.yml/badge.svg?style=for-the-badge)](https://github.com/zhiburt/ansi-str/actions) [![codecov](https://codecov.io/gh/zhiburt/ansi-str/branch/master/graph/badge.svg?token=8VGEM3ZT1T)](https://codecov.io/gh/zhiburt/ansi-str) [![Crate](https://img.shields.io/crates/v/ansi-str)](https://crates.io/crates/ansi-str) [![docs.rs](https://img.shields.io/badge/docs.rs-ansi--str-66c2a5?&color=blue&logo=docs.rs)](https://docs.rs/ansi-str/*/ansi_str/)

This is a library provides a set of methods to work with strings escaped with ansi code sequences.

It's an agnostic library in regard to different color libraries.
Therefore it can be used with any library.

## Usage

```rust
use ansi_str::AnsiStr;

pub fn main() {
    let text = "\u{1b}[1m\u{1b}[31;46mWhen the night has come\u{1b}[0m\u{1b}[0m";

    let cut = text.ansi_get(5..).expect("ok");

    println!("{}", text);
    println!("{}", cut);
}
```

Running this code will result in the following output.

![image](https://user-images.githubusercontent.com/20165848/151773080-d588a474-f43c-47b3-a29d-a92f19554907.png)

##### [For more examples, you check out the `examples` directory](https://github.com/zhiburt/ansi-str/tree/master/examples).

### Note

The library has derivatived from [zhiburt/ansi-cut](https://github.com/zhiburt/ansi-cut)
