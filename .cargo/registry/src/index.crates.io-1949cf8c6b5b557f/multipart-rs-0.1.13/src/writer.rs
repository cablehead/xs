use std::io::{self, Read, Write};
use uuid::Uuid;

pub struct MultipartWriter {
    pub boundary: String,
    pub data: Vec<u8>,
    first: bool,
}

impl MultipartWriter {
    pub fn new() -> MultipartWriter {
        MultipartWriter {
            boundary: format!("{}", Uuid::new_v4()),
            first: true,
            data: Vec::new(),
        }
    }

    pub fn new_with_boundary(boundary: &str) -> MultipartWriter {
        MultipartWriter {
            boundary: boundary.to_string(),
            first: true,
            data: Vec::new(),
        }
    }

    pub fn add(&mut self, mut reader: impl Read, headers: &str) -> io::Result<u64> {
        // writer for our buffer
        let mut writer = std::io::BufWriter::new(&mut self.data);

        // write the boundary
        if !self.first {
            writer.write_all(b"\r\n").unwrap();
        }

        // set first to false
        self.first = false;

        writer.write_all(b"--").unwrap();
        writer.write_all(self.boundary.as_bytes()).unwrap();
        writer.write_all(b"\r\n").unwrap();

        // write the content type
        writer.write_all(headers.as_bytes()).unwrap();

        // write an empty line
        writer.write_all(b"\r\n").unwrap();
        writer.write_all(b"\r\n").unwrap();

        // write the content
        io::copy(&mut reader, &mut writer)
    }

    pub fn finish(&mut self) {
        // writer for our buffer
        let mut writer = std::io::BufWriter::new(&mut self.data);

        // write the final boundary
        writer.write_all(b"\r\n").unwrap();
        writer.write_all(b"--").unwrap();
        writer.write_all(self.boundary.as_bytes()).unwrap();
        writer.write_all(b"--").unwrap();
    }
}
