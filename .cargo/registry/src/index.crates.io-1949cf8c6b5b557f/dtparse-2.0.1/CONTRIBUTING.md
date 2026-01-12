# Contributing

The `dtparse` crate is better for the contributions made by members of the open source community,
and seeks to make it easy to contribute back to the community it comes from. The goals are
fairly straight-forward, but here are the ways that would be most beneficial:

## Bug Reports

The testing suite for `dtparse` is built using tests derived from the [`dateutil`](https://github.com/dateutil/dateutil)
package in Python. Some Rust-specific behavior may show up though, for example in how
Rust handles nanoseconds where Python's standard library will only go to microseconds.

If you believe that behavior is improper, you are encouraged to file an issue; there are no dumb
issues or suggestions, and the world is a better place for having your input.

## Testing/Fuzzing

`dtparse`'s history as a port of Python software has led to some behavior being shown in Rust
that would not otherwise be an issue in Python. Testing for these issues to prevent panics
is greatly appreciated, and some great work has already happened surrounding fuzzing.

New test cases built either by fuzzers or humans are welcome.

## Feature Requests

Handling weird date formats and quirks is the name of the game. Any ideas on how to improve that
or utilities useful in handling the mapping of human time to computers is appreciated.

Writing code to implement the feature is never mandatory (though always appreciated); if there's
something you believe `dtparse` should do that it doesn't currently support, let's make that happen.

# Development Setup

The setup requirements for `dtparse` should be fairly straightforward - the project can be built
and deployed using only the `cargo` tool in Rust.

Much of the test coee is generated from Python code, and then the generated versions are stored
in version control. Thi is to ensure that all users can run the tests even without
installing Python or the other necessary packages.

To regenerate the tests, please use Python 3.6 with the `dateutil` package installed, and run:

- `python build_pycompat.py`
- `python build_pycompat_tokenizer.py`
