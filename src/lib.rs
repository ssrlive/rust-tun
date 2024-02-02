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

mod error;
use std::net::Ipv4Addr;

pub use crate::error::{BoxError, Error, Result};

mod address;
pub use crate::address::IntoAddress;

mod device;
pub use crate::device::AbstractDevice;

mod configuration;
pub use crate::configuration::{Configuration, Layer};

pub mod platform;
pub use crate::platform::create;

#[cfg(feature = "async")]
pub mod r#async;
#[cfg(feature = "async")]
pub use r#async::*;

pub fn configure() -> Configuration {
    Configuration::default()
}

#[cfg(unix)]
pub const DEFAULT_MTU: u16 = 1500;
#[cfg(windows)]
pub const DEFAULT_MTU: u16 = 0xFFFF; // 65535

pub const PACKET_INFORMATION_LENGTH: usize = 4;

pub fn netmask2prefix(netmask: Ipv4Addr) -> u32 {
    let mut n = u32::from_be_bytes(netmask.octets());
    let mut i = 0;
    while n > 0 {
        if n & 0x1 != 0 {
            i += 1;
        }
        n >>= 1;
    }
    i
}

pub fn startip_from_cidr(ip: Ipv4Addr, prefix: u32) -> Ipv4Addr {
    let mask = !(0xFFFFFFFFu32 >> prefix);
    let n = u32::from_be_bytes(ip.octets());
    Ipv4Addr::from((n & mask).to_be_bytes())
}
