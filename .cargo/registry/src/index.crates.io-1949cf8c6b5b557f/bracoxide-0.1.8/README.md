# {Bracoxide}

[![Tests](https://github.com/atahabaki/bracoxide/actions/workflows/rust.yml/badge.svg)](https://github.com/atahabaki/bracoxide/actions/workflows/rust.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Contributor Covenant](https://img.shields.io/badge/Contributor%20Covenant-2.1-4baaaa.svg)](CODE_OF_CONDUCT.md)
[![Documentation](https://docs.rs/bracoxide/badge.svg)](https://docs.rs/bracoxide)
[![Bracoxide Crate](https://img.shields.io/crates/v/bracoxide.svg)](https://crates.io/crates/bracoxide)

Bracoxide is a powerful Rust library for handling and expanding brace expansions.
It provides a simple and intuitive way to generate combinations and permutations
from brace patterns.

## Features

* __Brace Expansion__: Easily expand brace patterns into a list of all possible combinations.
* __Error Handling__: Comprehensive error handling for invalid brace patterns or expansion failures.
* __MIT Licensed__: Bracoxide is open-source and licensed under the MIT License.

## Installation

Add Bracoxide as a dependency:

```shell
cargo add bracoxide
```

## Usage

Import the bracoxide crate and start expanding brace patterns:

```rust
use bracoxide::explode;

fn main() {
    let content = "foo{1..3}bar";
    match explode(content) {
        Ok(expanded) => {
            // [`foo1bar`, `foo2bar`, `foo3bar`]
            println!("Expanded patterns: {:?}", expanded);
        }
        Err(error) => {
            eprintln!("Error occurred: {:?}", error);
        }
    }
}
```

For more details and advanced usage, please refer to the [API documentation](https://docs.rs/bracoxide).

## Contributing

```rust
match contribution {
    /// found a bug or encountered an issue
    Contribution::Issue => redirect!("https://github.com/atahabaki/bracoxide/issues"),
    /// propose any changes
    Contribution::Change => redirect!("https://github.com/atahabaki/bracoxide/pulls"),
    /// have a question or need help
    Contribution::Ask => redirect!("https://github.com/atahabaki/bracoxide/discussions"),
}
```

Contributions are welcome!
If you would like to contribute to this project, here are a few ways you can get involved:

- **Report Issues**: If you encounter any issues or bugs, please let us know by
[creating an issue](https://github.com/atahabaki/bracoxide/issues).
Provide a detailed description of the problem, including steps to reproduce it if possible.
- **Propose Changes**: If you have ideas for improvements or new features, we encourage you to
[submit a pull request](https://github.com/atahabaki/bracoxide/pulls).
We appreciate your contributions and will review and consider them.
- **Ask Questions**: If you have any questions or need help with the project, feel free to 
[start a discussion](https://github.com/atahabaki/bracoxide/discussions).
We'll be happy to assist you.

Please review our [contribution guidelines](Contributing.md) for more detailed information
on how to contribute effectively.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
