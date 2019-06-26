//! This crate integrates [`serde_json`] into a Tokio codec ([`tokio_codec::Decoder`] and
//! [`Encoder`]).

use bytes::BytesMut;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::io;
use std::marker::PhantomData;
use tokio_codec::{Decoder, Encoder};

/// JSON-based codec.
#[derive(Clone, Debug)]
pub struct Codec<D, E> {
    pretty: bool,
    _priv: (PhantomData<D>, PhantomData<E>),
}

impl<D, E> Codec<D, E> {
    /// Creates a new `Codec`.
    ///
    /// `pretty` controls whether or not encoded values are pretty-printed.
    pub fn new(pretty: bool) -> Self {
        Self {
            pretty,
            _priv: (PhantomData, PhantomData),
        }
    }

    /// Set whether or not encoded values are pretty-printed.
    pub fn pretty(&mut self, pretty: bool) {
        self.pretty = pretty;
    }
}

impl<D, E> Default for Codec<D, E> {
    fn default() -> Self {
        Self::new(false)
    }
}

impl<D, E> Decoder for Codec<D, E>
where
    for<'de> D: Deserialize<'de>,
{
    type Item = D;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<D>, std::io::Error> {
        let slice = &src.clone();
        let mut de = serde_json::Deserializer::from_slice(slice).into_iter();
        match de.next() {
            Some(Ok(v)) => {
                src.advance(de.byte_offset());
                Ok(Some(v))
            }
            Some(Err(e)) => {
                if e.is_eof() {
                    Ok(None)
                } else {
                    Err(e.into())
                }
            }
            None => {
                // The remaining stream is whitespace; clear the buffer so Decoder::decode_eof
                // doesn't return an Err
                src.clear();
                Ok(None)
            }
        }
    }
}

impl<D, E> Encoder for Codec<D, E>
where
    E: Serialize,
{
    type Item = E;
    type Error = std::io::Error;

    fn encode(&mut self, item: E, dst: &mut BytesMut) -> Result<(), std::io::Error> {
        let writer = BytesWriter(dst);
        if self.pretty {
            serde_json::to_writer_pretty(writer, &item)?;
        } else {
            serde_json::to_writer(writer, &item)?;
        }
        Ok(())
    }
}

/// The [`Error`][`std::error::Error`] type for this crate.
///
/// This is necessary to not lose information about the error. [`Encoder`] requires that the Error
/// implement `From<std::io::Error>`, and while a [`serde_json::Error`] can possibly be an IO
/// error, there's no way to combine the two.
///
/// If you just want an [`io::Error`], `From<Error>` is implemented for it.
#[derive(Debug)]
pub enum Error {
    /// A [`io::Error`].
    Io(io::Error),
    /// A [`serde_json::Error`].
    Json(serde_json::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Io(e) => e.fmt(f),
            Error::Json(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Json(err)
    }
}

impl From<Error> for io::Error {
    fn from(err: Error) -> Self {
        match err {
            Error::Io(e) => e,
            Error::Json(e) => e.into(),
        }
    }
}

/// Wrapper for `&mut [BytesMut]` that provides Write.
///
/// See also:
/// * <https://github.com/vorner/tokio-serde-cbor/blob/a347107ad56f2ad8086998eb63ecb70b19f3b71d/src/lib.rs#L167-L181>
/// * <https://github.com/carllerche/bytes/issues/77>
struct BytesWriter<'a>(&'a mut BytesMut);

impl<'a> io::Write for BytesWriter<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.extend(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Codec;
    use bytes::{BufMut, BytesMut};
    use std::collections::HashMap;
    use tokio_codec::{Decoder, Encoder};

    #[test]
    fn decode_empty() {
        let mut buf = BytesMut::from(&b""[..]);
        let mut codec: Codec<(), ()> = Codec::default();
        assert_eq!(codec.decode(&mut buf).unwrap(), None);
    }

    #[test]
    fn decode() {
        let mut buf = BytesMut::from(&b"null null null"[..]);
        let mut codec: Codec<_, ()> = Codec::default();
        assert_eq!(codec.decode(&mut buf).unwrap(), Some(()));
        assert_eq!(codec.decode(&mut buf).unwrap(), Some(()));
        assert_eq!(codec.decode(&mut buf).unwrap(), Some(()));
        assert_eq!(codec.decode(&mut buf).unwrap(), None);
        assert!(buf.is_empty());
    }

    #[test]
    fn decode_partial() {
        let mut buf = BytesMut::from(&b"null null nu"[..]);
        let mut codec: Codec<_, ()> = Codec::default();
        assert_eq!(codec.decode(&mut buf).unwrap(), Some(()));
        assert_eq!(codec.decode(&mut buf).unwrap(), Some(()));
        assert_eq!(codec.decode(&mut buf).unwrap(), None);
        assert_eq!(buf, &b" nu"[..]);
        buf.put(&b"ll"[..]);
        assert_eq!(codec.decode(&mut buf).unwrap(), Some(()));
        assert!(buf.is_empty());
    }

    #[test]
    fn decode_eof_trailing_whitespae() {
        let mut buf = BytesMut::from(&b"null\n"[..]);
        let mut codec: Codec<_, ()> = Codec::default();
        assert_eq!(codec.decode_eof(&mut buf).unwrap(), Some(()));
        assert_eq!(codec.decode_eof(&mut buf).unwrap(), None);
        assert!(buf.is_empty());
    }

    #[test]
    fn decode_err() {
        let mut buf = BytesMut::from(&b"null butts"[..]);
        let mut codec: Codec<_, ()> = Codec::default();
        assert_eq!(codec.decode(&mut buf).unwrap(), Some(()));
        assert!(codec.decode(&mut buf).is_err());
    }

    #[test]
    fn encode() {
        let mut buf = BytesMut::new();
        let mut codec: Codec<(), _> = Codec::default();
        codec.encode((), &mut buf).unwrap();
        assert_eq!(buf, &b"null"[..]);
    }

    #[test]
    fn encode_pretty() {
        let mut buf = BytesMut::new();
        let mut codec: Codec<(), _> = Codec::default();
        let mut hmap = HashMap::new();
        hmap.insert("key", "value");
        codec.encode(hmap.clone(), &mut buf).unwrap();
        codec.pretty(true);
        hmap.insert("key", "value");
        codec.encode(hmap.clone(), &mut buf).unwrap();
        assert_eq!(
            String::from_utf8(buf.to_vec()).unwrap(),
            r#"{"key":"value"}{
  "key": "value"
}"#
        );
    }
}
