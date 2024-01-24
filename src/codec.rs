//            DO WHAT THE FUCK YOU WANT TO PUBLIC LICENSE
//                    Version 2, December 2004
//
// Copyleft (ↄ) meh. <meh@schizofreni.co> | http://meh.schizofreni.co
//
// Everyone is permitted to copy and distribute verbatim or modified
// copies of this license document, and changing it is allowed as long
// as the name is changed.
//
//            DO WHAT THE FUCK YOU WANT TO PUBLIC LICENSE
//   TERMS AND CONDITIONS FOR COPYING, DISTRIBUTION AND MODIFICATION
//
//  0. You just DO WHAT THE FUCK YOU WANT TO.

use crate::PACKET_INFORMATION_LENGTH;
use bytes::{BufMut, Bytes, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

/// Infer the protocol based on the first nibble in the packet buffer.
pub(crate) fn is_ipv6(buf: &[u8]) -> std::io::Result<bool> {
    use std::io::{Error, ErrorKind::InvalidData};
    if buf.is_empty() {
        return Err(Error::new(InvalidData, "Zero-length data"));
    }
    match buf[0] >> 4 {
        4 => Ok(false),
        6 => Ok(true),
        p => Err(Error::new(InvalidData, format!("IP version {}", p))),
    }
}

pub(crate) fn generate_packet_information(_packet_information: bool, _ipv6: bool) -> Option<Bytes> {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    const TUN_PROTO_IP6: [u8; PACKET_INFORMATION_LENGTH] = (libc::ETH_P_IPV6 as u32).to_be_bytes();
    #[cfg(any(target_os = "linux", target_os = "android"))]
    const TUN_PROTO_IP4: [u8; PACKET_INFORMATION_LENGTH] = (libc::ETH_P_IP as u32).to_be_bytes();

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    const TUN_PROTO_IP6: [u8; PACKET_INFORMATION_LENGTH] = (libc::AF_INET6 as u32).to_be_bytes();
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    const TUN_PROTO_IP4: [u8; PACKET_INFORMATION_LENGTH] = (libc::AF_INET as u32).to_be_bytes();

    #[cfg(unix)]
    if _packet_information {
        let mut buf = BytesMut::with_capacity(PACKET_INFORMATION_LENGTH);
        if _ipv6 {
            buf.put_slice(&TUN_PROTO_IP6);
        } else {
            buf.put_slice(&TUN_PROTO_IP4);
        }
        return Some(buf.freeze());
    }
    None
}

/// A TUN packet Encoder/Decoder.
#[derive(Debug, Default)]
pub struct TunPacketCodec;

impl TunPacketCodec {
    /// Create a new `TunPacketCodec` specifying whether the underlying
    ///  tunnel Device has enabled the packet information header.
    pub fn new() -> TunPacketCodec {
        TunPacketCodec
    }
}

impl Decoder for TunPacketCodec {
    type Item = Vec<u8>;
    type Error = std::io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if buf.is_empty() {
            return Ok(None);
        }
        let pkt = buf.split_to(buf.len());
        let bytes = pkt.freeze();
        Ok(Some(bytes.into()))
    }
}

impl Encoder<Vec<u8>> for TunPacketCodec {
    type Error = std::io::Error;

    fn encode(&mut self, item: Vec<u8>, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let bytes = item.as_slice();
        dst.reserve(bytes.len());
        dst.put(bytes);
        Ok(())
    }
}
