# Multipart-RS

A simple, zero-allocation, streaming, async multipart reader & writer for Rust

## Reading multipart

```rust
let headermap = vec![(
    "Content-Type".to_string(),
    "multipart/form-data; boundary=--974767299852498929531610575".to_string(),
)];
// Lines must end with CRLF
let data = b"--974767299852498929531610575\r
Content-Disposition: form-data; name=\"afile\"; filename=\"a.txt\"\r
\r
Content of a.txt.\r
--974767299852498929531610575\r
Content-Disposition: form-data; name=\"bfile\"; filename=\"b.txt\"\r
Content-Type: text/plain\r
\r
Content of b.txt.\r
--974767299852498929531610575--\r\n";

let reader = MultipartReader::from_data_with_headers(data, &headermap);

loop {
    match reader.next().await {
        Some(Ok(item)) => println!(item),
        Some(Err(e)) => panic!("Error: {:?}", e),
        None => break,
    }
}
```