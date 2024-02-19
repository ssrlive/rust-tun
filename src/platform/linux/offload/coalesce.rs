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

use crate::platform::linux::offload::packet::{IpPacket, TcpPacket, UdpPacket};
use crate::platform::linux::offload::tcp::TcpGroItem;
use crate::platform::linux::offload::udp::UdpGroItem;
use crate::{Error, Result};
use bytes::BufMut;
use etherparse::{IpHeaders, Ipv4Header, Ipv6Header};

pub(crate) enum CoalesceResult {
    InsufficientCapacity,
    PSHEnding,
    ItemInvalidChecksum,
    PacketInvalidChecksum,
    Success,
}

pub(crate) trait IpCoaleasceable {
    fn can_coalesce(&self, other: &Self) -> bool;
}

pub(crate) trait TransportCoaleasceable {
    type GroItem;

    fn can_coalesce(&self, gro_item: &Self::GroItem) -> bool;
    fn coalesce(&mut self, gro_item: &mut Self::GroItem) -> Result<()>;
}

impl IpCoaleasceable for Ipv4Header {
    #[inline]
    fn can_coalesce(&self, other: &Self) -> bool {
        if self.dscp != other.dscp {
            return false;
        }

        if self.ecn != other.ecn {
            return false;
        }

        if self.dont_fragment != other.dont_fragment {
            return false;
        }

        if self.time_to_live != other.time_to_live {
            return false;
        }

       true
    }
}

impl IpCoaleasceable for Ipv6Header {
    #[inline]
    fn can_coalesce(&self, other: &Self) -> bool {
        if self.traffic_class != other.traffic_class {
            return false;
        }

        if self.hop_limit != other.hop_limit {
            return false;
        }

        true
    }
}

impl IpCoaleasceable for IpHeaders {
    #[inline]
    fn can_coalesce(&self, other: &Self) -> bool {
        match (self, other) {
            (IpHeaders::Ipv4(self_ipv4_header, _), IpHeaders::Ipv4(other_ipv4_header, _)) => {
                self_ipv4_header.can_coalesce(other_ipv4_header)
            }
            (IpHeaders::Ipv6(self_ipv6_header, _), IpHeaders::Ipv6(other_ipv6_header, _)) => {
                self_ipv6_header.can_coalesce(other_ipv6_header)
            }
            _ => false,
        }
    }
}

impl TransportCoaleasceable for IpPacket<TcpPacket> {
    type GroItem = TcpGroItem;

    #[inline]
    fn can_coalesce(&self, gro_item: &TcpGroItem) -> bool {
        if !self.header.can_coalesce(&gro_item.ip_header) {
            return false;
        }

        let tcp_header = &self.transport.header;
        let data = &self.transport.data;

        if tcp_header.header_len() != tcp_header.header_len() {
            return false;
        }

        if tcp_header.options != tcp_header.options {
            return false;
        }

        let mut len = gro_item.data.len() as u32;
        len += gro_item.num_merged * gro_item.data.len() as u32;

        if tcp_header.sequence_number == gro_item.seq_num + len {
            if tcp_header.psh {
                return false;
            }

            if data.len() > gro_item.data.len() {
                return false;
            }

            return true;
        } else if tcp_header.sequence_number + data.len() as u32 == gro_item.seq_num {
            if tcp_header.psh {
                return false;
            }

            if data.len() < gro_item.data.len() {
                return false;
            }

            if data.len() > gro_item.data.len() && gro_item.num_merged > 0 {
                return false;
            }

            return true;
        }

        false
    }

    #[inline]
    fn coalesce(&mut self, gro_item: &mut TcpGroItem) -> Result<()> {
        let tcp_header = &self.transport.header;
        let data = &self.transport.data;

        if gro_item.data.remaining_mut() < self.transport.data.len() {
            return Err(Error::BufferTooSmall);
        }

        if tcp_header.psh {
            return Err(Error::OffloadTcpPshFlagSet);
        }

        // TODO: checksums?

        gro_item.seq_num = tcp_header.sequence_number;
        gro_item.data.chunk_mut().copy_from_slice(data);

        Ok(())
    }
}

impl TransportCoaleasceable for IpPacket<UdpPacket> {
    type GroItem = UdpGroItem;

    #[inline]
    fn can_coalesce(&self, gro_item: &UdpGroItem) -> bool {
        if !self.header.can_coalesce(&gro_item.ip_header) {
            return false;
        }

        let udp_header = &self.transport.header;

        if udp_header.length > gro_item.udp_header.length {
            return false;
        }

        true
    }

    #[inline]
    fn coalesce(&mut self, gro_item: &mut UdpGroItem) -> Result<()> {
        let data = &self.transport.data;

        if gro_item.data.remaining_mut() < data.len() {
            return Err(Error::BufferTooSmall);
        }

        if gro_item.num_merged == 0 && gro_item.checksum_known_invalid {
            return Err(Error::OffloadItemInvalidChecksum);
        }

        // TODO: checksums?

        gro_item.data.chunk_mut().copy_from_slice(data);
        gro_item.num_merged += 1;

        Ok(())
    }
}
