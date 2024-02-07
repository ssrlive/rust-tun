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

use crate::{Error, Result};
use std::{mem, ptr};

const TCP_FLAG_FIN: u8 = 0x01;
const TCP_FLAG_PSH: u8 = 0x08;
const TCP_FLAG_ACK: u8 = 0x10;

const VIRTIO_NET_HEADER_SIZE: usize = mem::size_of::<VirtioNetHeader>();

#[repr(C)]
struct VirtioNetHeader {
    flags: u8,
    gso_type: u8,
    header_len: u16,
    gso_size: u16,
    checksum_start: u16,
    checksum_offset: u16,
}

impl VirtioNetHeader {
    pub fn decode(data: &[u8]) -> Result<Self> {
        if data.len() < VIRTIO_NET_HEADER_SIZE {
            return Err(Error::BufferTooSmall);
        }

        unsafe {
            // SAFETY:
            // - data is valid to read
            // - data is aligned
            // - the bytes represent a valid header
            ptr::read(data[..VIRTIO_NET_HEADER_SIZE].as_ptr() as *const _)
        }
    }

    pub fn encode(&self, data: &mut [u8]) -> Result<()> {
        if data.len() < VIRTIO_NET_HEADER_SIZE {
            return Err(Error::BufferTooSmall);
        }

        unsafe {
            // SAFETY:
            // - data is valid to write
            // - data is aligned
            ptr::write(data[..VIRTIO_NET_HEADER_SIZE].as_mut_ptr() as *mut _, self);
        }

        Ok(())
    }
}
