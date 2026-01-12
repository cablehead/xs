# Nested enum utils

This crate provides a single attribute macro to provide conversions from enum cases to the enum itself or to some other type.

It only works with enums where each variant has a single unnamed element, and if each variant has a distinct type.

The most basic use is to provide conversions between the enum cases and the enum type itself. You could achieve something similar with the popular [derive_more] crate.

```rust
#[enum_conversions()]
enum Request {
  Get(GetRequest),
  Put(PutRequest),
}
```

A more advanced use, and the reason for this crate to exist, is to provide conversions between enum variants and any type that itself has a *conversion* to the enum. This allows to use nested enums, like you would in a complex protocol that has several subsystems.

```rust
#[enum_conversions(Request)]
enum StoreRequest {
  Get(GetRequest),
  Put(PutRequest),
}

#[enum_conversions(Request)]
enum NetworkRequest {
  Ping(PingRequest),
}

#[enum_conversions()]
enum Request {
  Store(StoreRequest),
  Network(NetworkRequest),
}
```

Here we define conversions from `GetRequest` to `StoreRequest`, from `StoreRequest` to `Request`, and then directly from `GetRequest` to `Request`, and corresponding [TryFrom] conversions in the other direction.

## Generated conversions

The generated [From] conversions are straightforward. Obviously it is always possible to convert from an enum case to the enum itself.

We also generate [TryFrom] conversions from the enum to each variant, as well as from a reference to the enum to a reference to the variant.

The conversions that take a value are different than the ones from [derive_more]: they return the unmodified input in the error case, allowing to chain conversion attempts.

```rust
let request = ...
match GetRequest::try_from(request) {
  Ok(get) => // handle get request
  Err(request) => {
    // I still got the request and can try something else
    match PutRequest::try_from(request) {
      ...
    }
  }
}
```

The conversions that take a reference just return a `&'static str` as the error type. References are `Copy`, so we can always retry anyway.

[From]: https://doc.rust-lang.org/std/convert/trait.From.html
[TryFrom]: https://doc.rust-lang.org/std/convert/trait.TryFrom.html
[derive_more]: https://crates.io/crates/derive_more