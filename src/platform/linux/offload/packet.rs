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

use bytes::Bytes;
use etherparse::{IpHeaders, TcpHeader, UdpHeader};
use std::fmt::Debug;

#[derive(Clone, Debug)]
pub(crate) struct IpPacket<T: Clone + Debug> {
    pub(crate) header: IpHeaders,
    pub(crate) transport: T,
}

#[derive(Clone, Debug)]
pub(crate) struct TcpPacket {
    pub(crate) header: TcpHeader,
    pub(crate) data: Bytes,
}

#[derive(Clone, Debug)]
pub(crate) struct UdpPacket {
    pub(crate) header: UdpHeader,
    pub(crate) data: Bytes,
}
