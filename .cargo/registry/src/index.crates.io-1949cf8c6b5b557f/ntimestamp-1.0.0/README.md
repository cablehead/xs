# ntimestamp

Strictly monotonic unix timestamp in microseconds.

## Features

- Strict monotonicity
- Sortable encoding
- Unique Id ~ish
- String representation

### Strict monotonicity

Unlike calling [SystemTime::now], calling [Timestamp::now] is guaranteed to always create an increasing
timestamp, never moving back in time, nor repeating the same timestamp even if you call it many timest within
the same microsecond.

### Sortable encoding

Always encoded as Big-endian u64, so that it can be used as sortable keys.

### String representation

If you enable `base32` feature, you can also get sortable utf-8 representation of 13 characters, that are also easy to copy visually. 
for example `0032992ANQB5G`.

If you enable `httpdate` you can format [Timestamp] as [http date format](https://www.rfc-editor.org/rfc/rfc7231#section-7.1.1.1), or parse an http date to a timestamp.

### Unique Id

While it can't be used as a globally unique Id, it is unique within the same process.

#### Clock Ids

If you use it concurrently through different processes, each process will create a unique one-byte `clock_id`.

This means that if you setup your custom [TimestampFactory], you can have up to `256` processes all generating unique timestamps.

Otherwise, if you use the [DEFAULT_FACTORY], which sets the [TimestampFactory::clock_id] randomly, your chance of having unique
clock ids is relative to how many processes are you running, and how often do you restart these processes.
