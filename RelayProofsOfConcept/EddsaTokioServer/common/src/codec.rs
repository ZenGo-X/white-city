use serde::{Serialize, Deserialize};
use serde_json;
use tokio_core::io::{Codec, EasyBuf};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use std::io;
use std::marker::PhantomData;
use std::mem;

pub struct LengthPrefixedJson<In, Out>
    where In: Serialize + Deserialize,//<'a>,
          Out: Serialize + Deserialize//<'a>
{
    _in: PhantomData<In>,
    _out: PhantomData<Out>,
}

impl<In, Out> LengthPrefixedJson<In, Out>
    where In: Serialize + Deserialize,//<'a>,
          Out: Serialize + Deserialize//<'a>
{
    pub fn new() -> LengthPrefixedJson<In, Out> {
        LengthPrefixedJson {
            _in: PhantomData,
            _out: PhantomData,
        }
    }

}

// `LengthPrefixedJson` is a codec for sending and receiving serde_json serializable types. The
// over the wire format is a Big Endian u16 indicating the number of bytes in the JSON payload
// (not including the 2 u16 bytes themselves) followed by the JSON payload.
impl<In, Out> Codec for LengthPrefixedJson< In, Out>
    where In: Serialize + Deserialize,//<'a>,
          Out: Serialize + Deserialize//<'a>
{
    type In = In;
    type Out = Out;

    fn decode(&mut self, buf: &mut EasyBuf) -> io::Result<Option<Self::In>> {
        // Make sure we have at least the 2 u16 bytes we need.
        let mut c_buf = buf.clone();
        //println!("DECODING {:?}",buf);
        let msg_size = match buf.as_ref().read_u16::<BigEndian>() {
            Ok(msg_size) => msg_size,
            Err(_) => return Ok(None),
        };
//        println!("Message size is {:?}", msg_size);
        let hdr_size = mem::size_of_val(&msg_size);
        let msg_size = msg_size as usize + hdr_size;

        // Make sure our buffer has all the bytes indicated by msg_size.
        if buf.len() < msg_size {
            return Ok(None);
        }

        // Drain off the entire message.
        let buf = buf.drain_to(msg_size);

        // Trim off the u16 length bytes.
        let msg_buf = &buf.as_ref()[hdr_size..];

        // Decode!
        let msg = serde_json::from_slice(msg_buf)
            .map_err(|err| {println!("decode error: {:?}",err);io::Error::new(io::ErrorKind::InvalidData, err)});
        match msg {
            Ok(msg) => { Ok(Some(msg))},
            Err(e) => {
                let header_bytes = c_buf.drain_to(2);
                let element_size = match c_buf.as_ref().read_u16::<BigEndian>() {
                    Ok(msg_size) => msg_size,
                    Err(_) => return Ok(None),
                };
                //let element_size = c_buf.clone().as_slice()[3] as usize;
                let mut smaller_buf = c_buf.drain_to( element_size as usize + 2);
                //smaller_buf.drain_to(2);
                /// Afterwards `self` contains elements `[at, len)`, and the returned `EasyBuf`
                /// contains elements `[0, at)`.
                //println!("attempting to decode smaller buf:");
               // println!("-------------\n{:#?}\n-------------",smaller_buf);
                // Make sure we have at least the 2 u16 bytes we need.
                //println!("DECODING SMALLER {:?}",smaller_buf);
                let msg_size = match smaller_buf.as_ref().read_u16::<BigEndian>() {
                    Ok(msg_size) => msg_size,
                    Err(_) => return Ok(None),
                };
                let hdr_size = mem::size_of_val(&msg_size);
                let msg_size = msg_size as usize + hdr_size;

                // Make sure our buffer has all the bytes indicated by msg_size.
                if buf.len() < msg_size {
                    return Ok(None);
                }

                // Drain off the entire message.
                let buf = smaller_buf.drain_to(msg_size);

                // Trim off the u16 length bytes.
                let msg_buf = &buf.as_ref()[hdr_size..];

                // Decode!
                let msg: In = serde_json::from_slice(msg_buf)
                    .map_err(|err| {println!("inner decode error: {:?}",err);io::Error::new(io::ErrorKind::InvalidData, err)})?;
                Ok(Some(msg))
            }
        }
    }

    fn encode(&mut self, msg: Out, buf: &mut Vec<u8>) -> io::Result<()> {
        // Encode directly into `buf`.
//	println!("ENCODING {:?}", buf);
        serde_json::to_writer(buf, &msg)
            .map_err(|err| {println!("ENCODING ERROR");io::Error::new(io::ErrorKind::InvalidData, err)})?;

        let len = buf.len() as u16;

        // add space for our length
        for _ in 0..mem::size_of_val(&len) {
            buf.insert(0, 0);
        }

        // Insert our length bytes at the front of `buf`.
        let mut cursor: io::Cursor<&mut Vec<u8>> = io::Cursor::new(buf.as_mut());
        cursor.set_position(0);
        cursor.write_u16::<BigEndian>(len)
    }
}
