use std::{
    pin::Pin,
    str,
    task::{Context, Poll},
};

use bytes::{Buf, Bytes, BytesMut};
use futures_core::{stream::BoxStream, Stream};
use futures_util::StreamExt;

use crate::{error::MultipartError, multipart_type::MultipartType};

#[derive(PartialEq, Debug)]
enum InnerState {
    /// Stream eof
    Eof,

    /// Skip data until first boundary
    FirstBoundary,

    /// Reading boundary
    Boundary,

    /// Reading Headers,
    Headers,
}

pub struct MultipartItem {
    /// Headers
    pub headers: Vec<(String, String)>,

    /// Data
    pub data: BytesMut,
}

impl MultipartItem {
    pub fn get_mime_type(&self) -> Result<mime::Mime, MultipartError> {
        let content_type = self
            .headers
            .iter()
            .find(|(key, _)| key.to_lowercase() == "content-type");

        if content_type.is_none() {
            return Err(MultipartError::InvalidContentType);
        }

        let ct = content_type
            .unwrap()
            .1
            .parse::<mime::Mime>()
            .map_err(|_e| MultipartError::InvalidContentType)?;

        Ok(ct)
    }

    pub fn get_file_name(&self) -> Option<String> {
        let content_disposition = self
            .headers
            .iter()
            .find(|(key, _)| key.to_lowercase() == "content-disposition")?;

        let cd = &content_disposition.1;
        let parts: Vec<&str> = cd.split(";").collect();
        let filename = parts
            .iter()
            .find(|p| p.trim().starts_with("filename="))
            .map(|p| p.trim().split("=").collect::<Vec<&str>>()[1].to_string());

        filename
    }
}

pub struct MultipartReader<'a, E> {
    pub boundary: String,
    pub multipart_type: MultipartType,
    /// Inner state
    state: InnerState,
    stream: BoxStream<'a, Result<Bytes, E>>,
    buf: BytesMut,
    pending_item: Option<MultipartItem>,
}

impl<'a, E> MultipartReader<'a, E> {
    pub fn from_stream_with_boundary_and_type<S>(
        stream: S,
        boundary: &str,
        multipart_type: MultipartType,
    ) -> Result<MultipartReader<'a, E>, MultipartError>
    where
        S: Stream<Item = Result<Bytes, E>> + 'a + Send,
    {
        Ok(MultipartReader {
            stream: stream.boxed(),
            boundary: boundary.to_string(),
            multipart_type: multipart_type,
            state: InnerState::FirstBoundary,
            pending_item: None,
            buf: BytesMut::new(),
        })
    }

    pub fn from_data_with_boundary_and_type(
        data: &[u8],
        boundary: &str,
        multipart_type: MultipartType,
    ) -> Result<MultipartReader<'a, E>, MultipartError>
    where
        E: std::error::Error + 'a + Send,
    {
        let stream = futures_util::stream::iter(vec![Ok(Bytes::copy_from_slice(data))]);
        MultipartReader::from_stream_with_boundary_and_type(stream, boundary, multipart_type)
    }

    pub fn from_stream_with_headers<S>(
        stream: S,
        headers: &[(String, String)],
    ) -> Result<MultipartReader<'a, E>, MultipartError>
    where
        S: Stream<Item = Result<Bytes, E>> + 'a + Send,
        E: std::error::Error,
    {
        // Search for the content-type header
        let content_type = headers
            .iter()
            .find(|(key, _)| key.to_lowercase() == "content-type");

        if content_type.is_none() {
            return Err(MultipartError::NoContentType);
        }

        let ct = content_type
            .unwrap()
            .1
            .parse::<mime::Mime>()
            .map_err(|_e| MultipartError::InvalidContentType)?;
        let boundary = ct
            .get_param(mime::BOUNDARY)
            .ok_or(MultipartError::InvalidBoundary)?;

        if ct.type_() != mime::MULTIPART {
            return Err(MultipartError::InvalidContentType);
        }

        let multipart_type = ct
            .subtype()
            .as_str()
            .parse::<MultipartType>()
            .map_err(|_| MultipartError::InvalidMultipartType)?;

        Ok(MultipartReader {
            stream: stream.boxed(),
            boundary: boundary.to_string(),
            multipart_type: multipart_type,
            state: InnerState::FirstBoundary,
            pending_item: None,
            buf: BytesMut::new(),
        })
    }

    pub fn from_data_with_headers(
        data: &[u8],
        headers: &Vec<(String, String)>,
    ) -> Result<MultipartReader<'a, E>, MultipartError>
    where
        E: std::error::Error + 'a + Send,
    {
        let stream = futures_util::stream::iter(vec![Ok(Bytes::copy_from_slice(data))]);
        MultipartReader::from_stream_with_headers(stream, headers)
    }

    fn is_final_boundary(&self, data: &[u8]) -> bool {
        let boundary = format!("--{}--", self.boundary);
        data.starts_with(boundary.as_bytes())
    }

    // TODO: make this RFC compliant
    fn is_boundary(&self, data: &[u8]) -> bool {
        let boundary = format!("--{}", self.boundary);
        data.starts_with(boundary.as_bytes())
    }
}

impl<'a, E> Stream for MultipartReader<'a, E> {
    type Item = Result<MultipartItem, MultipartError>;

    fn poll_next(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        let finder = memchr::memmem::Finder::new("\r\n");

        loop {
            while let Some(idx) = finder.find(&this.buf) {
                match this.state {
                    InnerState::FirstBoundary => {
                        // Check if the last line was a boundary
                        if this.is_boundary(&this.buf[..idx]) {
                            this.state = InnerState::Headers;
                        };
                    }
                    InnerState::Boundary => {
                        // Check if the last line was a boundary
                        if this.is_boundary(&this.buf[..idx]) {
                            let final_boundary = this.is_final_boundary(&this.buf[..idx]);

                            // If we have a pending item, return it
                            if let Some(mut item) = this.pending_item.take() {
                                // Remove last 2 bytes from the data (which were a newline sequence)
                                item.data.truncate(item.data.len() - 2);
                                // Skip to the next line
                                this.buf.advance(2 + idx);
                                if final_boundary {
                                    this.state = InnerState::Eof;
                                } else {
                                    this.state = InnerState::Headers;
                                }
                                return std::task::Poll::Ready(Some(Ok(item)));
                            }

                            this.state = InnerState::Headers;
                            this.pending_item = Some(MultipartItem {
                                headers: vec![],
                                data: BytesMut::new(),
                            });
                        };

                        // Add the data to the pending item
                        this.pending_item
                            .as_mut()
                            .unwrap()
                            .data
                            .extend(&this.buf[..idx + 2])
                    }
                    InnerState::Headers => {
                        // Check if we have a pending item or we should create one
                        if this.pending_item.is_none() {
                            this.pending_item = Some(MultipartItem {
                                headers: vec![],
                                data: BytesMut::new(),
                            });
                        }

                        // Read the header line and split it into key and value
                        let header = match str::from_utf8(&this.buf[..idx]) {
                            Ok(h) => h,
                            Err(_) => {
                                this.state = InnerState::Eof;
                                return std::task::Poll::Ready(Some(Err(
                                    MultipartError::InvalidItemHeader,
                                )));
                            }
                        };

                        // This is no header anymore, we are at the end of the headers
                        if header.trim().is_empty() {
                            this.buf.advance(2 + idx);
                            this.state = InnerState::Boundary;
                            continue;
                        }

                        let header_parts: Vec<&str> = header.split(": ").collect();
                        if header_parts.len() != 2 {
                            this.state = InnerState::Eof;
                            return std::task::Poll::Ready(Some(Err(
                                MultipartError::InvalidItemHeader,
                            )));
                        }

                        // Add header entry to the pending item
                        this.pending_item
                            .as_mut()
                            .unwrap()
                            .headers
                            .push((header_parts[0].to_string(), header_parts[1].to_string()));
                    }
                    InnerState::Eof => {
                        return std::task::Poll::Ready(None);
                    }
                }

                // Skip to the next line
                this.buf.advance(2 + idx);
            }

            // Read more data from the stream
            match Pin::new(&mut this.stream).poll_next(cx) {
                Poll::Ready(Some(Ok(data))) => {
                    this.buf.extend_from_slice(&data);
                }
                Poll::Ready(None) => {
                    this.state = InnerState::Eof;
                    return std::task::Poll::Ready(None);
                }
                Poll::Ready(Some(Err(_e))) => {
                    this.state = InnerState::Eof;
                    return std::task::Poll::Ready(Some(Err(MultipartError::PollingDataFailed)));
                }
                Poll::Pending => {
                    return std::task::Poll::Pending;
                }
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[futures_test::test]
    async fn valid_request() {
        let headermap = vec![(
            "Content-Type".to_string(),
            "multipart/form-data; boundary=974767299852498929531610575".to_string(),
        )];
        // Lines must end with CRLF
        let data = b"--974767299852498929531610575\r
Content-Disposition: form-data; name=\"text\"\r
\r
text default\r
--974767299852498929531610575\r
Content-Disposition: form-data; name=\"file1\"; filename=\"a.txt\"\r
Content-Type: text/plain\r
\r
Content of a.txt.\r
\r\n--974767299852498929531610575\r
Content-Disposition: form-data; name=\"file2\"; filename=\"a.html\"\r
Content-Type: text/html\r
\r
<!DOCTYPE html><title>Content of a.html.</title>\r
\r
--974767299852498929531610575--\r\n";

        assert!(
            MultipartReader::<std::io::Error>::from_data_with_headers(data, &headermap).is_ok()
        );
        assert!(
            MultipartReader::<std::io::Error>::from_data_with_boundary_and_type(
                data,
                "974767299852498929531610575",
                MultipartType::FormData
            )
            .is_ok()
        );

        // Poll all the items from the reader
        let mut reader =
            MultipartReader::<std::io::Error>::from_data_with_headers(data, &headermap).unwrap();
        assert_eq!(reader.multipart_type, MultipartType::FormData);
        let mut items = vec![];

        loop {
            match reader.next().await {
                Some(Ok(item)) => items.push(item),
                None => break,
                Some(Err(e)) => panic!("Error: {:?}", e),
            }
        }

        assert_eq!(items.len(), 3);
    }
}
