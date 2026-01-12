[![MIT][s2]][l2] [![Latest Version][s1]][l1] [![docs][s3]][l3] [![Chat on Miaou][s4]][l4]

[s1]: https://img.shields.io/crates/v/umask.svg
[l1]: https://crates.io/crates/umask

[s2]: https://img.shields.io/badge/license-MIT-blue.svg
[l2]: LICENSE

[s3]: https://docs.rs/umask/badge.svg
[l3]: https://docs.rs/umask/

[s4]: https://miaou.dystroy.org/static/shields/room.svg
[l4]: https://miaou.dystroy.org/3

# umask

A light utility helping with unix mode representation, with strong types to avoid misusing constants.

The Mode struct implements `Display` and prints as `"rwxrwxrwx"`

### Import

In Cargo.toml:

    umask = "'2.0"


### Usage

```rust
use umask::*;

// You can build from a number:
assert_eq!("rw-r--r--", Mode::from(0b110100100).to_string());
assert_eq!("rw-r--r--", Mode::from(0o644).to_string());

// You may use `|` to combine class permissions:
let mu = Mode::from(0o640);
let mo = Mode::from(0o044);
assert_eq!("rw-r--r--", (mu | mo).to_string());
assert_eq!("---r-----", (mu & mo).to_string());

// You can use more semantic constructs:
let m = Mode::all()
    .without(ALL_EXEC);
assert_eq!("rw-rw-rw-", m.to_string());
let mut m = Mode::new()
    .with_class_perm(ALL, READ)
    .with_class_perm(USER, WRITE);
assert_eq!("rw-r--r--", m.to_string());
// (semantic functions can be used in const declarations)

// Or
m |= ALL_EXEC;
assert_eq!("rwxr-xr-x", m.to_string());
let m = ALL_READ | USER_WRITE;
assert_eq!("rw-r--r--", m.to_string());

// Displaying the mode can be done with the `Display`
// implementation but also bit per bit for more control
assert_eq!(
    m.to_string().chars().next().unwrap(), // first char: 'r' or '-'
    if m.has(USER_READ) { 'r' } else { '-' },
);

// The `Display` implementation shows the extra permission bits
// (setuid, setgid and sticky):
let mut m = Mode::all()
    .with_extra(STICKY)
    .with_extra(SETUID)
    .with_extra(SETGID);
assert_eq!("rwsrwsrwt", m.to_string());

// But you can remove those bits for display if you want the
// sometimes more familiar 'x' for execution:
assert_eq!("rwxrwxrwx", m.without_any_extra().to_string());

```
