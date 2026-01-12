# is_debug 
The crate by Rust that get build model is debug.

### use function
```TOML
[dependencies]
is_debug = "1"
```

```rust
use is_debug::{is_debug, is_release};

fn main() {
	println!("{}", is_debug());

	println!("{}", is_release());
}
```
