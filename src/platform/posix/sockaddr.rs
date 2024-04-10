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

use crate::error::{Error, Result};
use libc::{in_addr, sockaddr, sockaddr_in};
use std::{mem, net::Ipv4Addr, ptr};

/// # Safety
pub unsafe fn sockaddr_to_rs_addr(sa: &sockaddr_union) -> Option<std::net::SocketAddr> {
    match sa.addr_stor.ss_family as libc::c_int {
        libc::AF_INET => {
            let sa_in = sa.addr4;
            let ip = std::net::Ipv4Addr::from(sa_in.sin_addr.s_addr.to_ne_bytes());
            let port = u16::from_be(sa_in.sin_port);
            Some(std::net::SocketAddr::new(ip.into(), port))
        }
        libc::AF_INET6 => {
            let sa_in6 = sa.addr6;
            let ip = std::net::Ipv6Addr::from(sa_in6.sin6_addr.s6_addr);
            let port = u16::from_be(sa_in6.sin6_port);
            Some(std::net::SocketAddr::new(ip.into(), port))
        }
        _ => None,
    }
}

pub fn rs_addr_to_sockaddr(addr: std::net::SocketAddr) -> sockaddr_union {
    match addr {
        std::net::SocketAddr::V4(ipv4) => {
            let mut addr: sockaddr_union = unsafe { std::mem::zeroed() };
            #[cfg(any(target_os = "freebsd", target_os = "macos"))]
            {
                addr.addr4.sin_len = std::mem::size_of::<libc::sockaddr_in>() as u8;
            }
            addr.addr4.sin_family = libc::AF_INET as libc::sa_family_t;
            addr.addr4.sin_addr.s_addr = u32::from_ne_bytes(ipv4.ip().octets());
            addr.addr4.sin_port = ipv4.port().to_be();
            addr
        }
        std::net::SocketAddr::V6(ipv6) => {
            let mut addr: sockaddr_union = unsafe { std::mem::zeroed() };
            #[cfg(any(target_os = "freebsd", target_os = "macos"))]
            {
                addr.addr6.sin6_len = std::mem::size_of::<libc::sockaddr_in6>() as u8;
            }
            addr.addr6.sin6_family = libc::AF_INET6 as libc::sa_family_t;
            addr.addr6.sin6_addr.s6_addr = ipv6.ip().octets();
            addr.addr6.sin6_port = ipv6.port().to_be();
            addr
        }
    }
}

pub unsafe fn ipaddr_to_sockaddr<T: Into<std::net::IpAddr>>(
    src_addr: T,
    src_port: u16,
    mut target_addr: &mut libc::sockaddr,
) {
    let sa = rs_addr_to_sockaddr((src_addr.into(), src_port).into());
    ptr::copy_nonoverlapping(
        &sa as *const _ as *const libc::c_void,
        &mut target_addr as *mut _ as *mut libc::c_void,
        std::mem::size_of::<libc::__c_anonymous_ifr_ifru>(),
    );
}

#[repr(C)]
pub union sockaddr_union {
    pub addr_stor: libc::sockaddr_storage,
    pub addr6: libc::sockaddr_in6,
    pub addr4: libc::sockaddr_in,
    pub addr: libc::sockaddr,
}

/// A wrapper for `sockaddr_in`.
#[derive(Copy, Clone, Debug)]
pub struct SockAddr(sockaddr_in);

impl SockAddr {
    /// Create a new `SockAddr` from a generic `sockaddr`.
    pub fn new(value: &sockaddr) -> Result<Self> {
        if value.sa_family != libc::AF_INET as libc::sa_family_t {
            return Err(Error::InvalidAddress);
        }

        unsafe { Self::unchecked(value) }
    }

    /// # Safety
    ///  Create a new `SockAddr` and not check the source.
    pub unsafe fn unchecked(value: &sockaddr) -> Result<Self> {
        Ok(SockAddr(ptr::read(value as *const _ as *const _)))
    }

    /// # Safety
    /// Get a generic pointer to the `SockAddr`.
    pub unsafe fn as_ptr(&self) -> *const sockaddr {
        &self.0 as *const _ as *const sockaddr
    }
}

impl From<Ipv4Addr> for SockAddr {
    fn from(ip: Ipv4Addr) -> SockAddr {
        let octets = ip.octets();

        let mut addr = unsafe { mem::zeroed::<sockaddr_in>() };

        addr.sin_family = libc::AF_INET as libc::sa_family_t;
        addr.sin_port = 0;

        #[cfg(any(target_os = "freebsd", target_os = "macos"))]
        fn set_sin_len(addr: &mut sockaddr_in) {
            addr.sin_len = std::mem::size_of::<sockaddr_in>() as u8;
        }
        #[cfg(not(any(target_os = "freebsd", target_os = "macos")))]
        fn set_sin_len(_addr: &mut sockaddr_in) {}
        set_sin_len(&mut addr);

        addr.sin_addr = in_addr {
            s_addr: u32::from_ne_bytes(octets),
        };

        SockAddr(addr)
    }
}

impl From<SockAddr> for Ipv4Addr {
    fn from(addr: SockAddr) -> Ipv4Addr {
        let ip = addr.0.sin_addr.s_addr;
        let [a, b, c, d] = ip.to_ne_bytes();

        Ipv4Addr::new(a, b, c, d)
    }
}

impl From<SockAddr> for sockaddr {
    fn from(addr: SockAddr) -> sockaddr {
        unsafe { mem::transmute(addr.0) }
    }
}

impl From<SockAddr> for sockaddr_in {
    fn from(addr: SockAddr) -> sockaddr_in {
        addr.0
    }
}

#[test]
fn test_sockaddr() {
    let old = Ipv4Addr::new(127, 0, 0, 1);
    let addr = SockAddr::from(old);
    if cfg!(target_endian = "big") {
        assert_eq!(0x7f000001, addr.0.sin_addr.s_addr);
    } else if cfg!(target_endian = "little") {
        assert_eq!(0x0100007f, addr.0.sin_addr.s_addr);
    } else {
        unreachable!();
    }
    let ip = Ipv4Addr::from(addr);
    assert_eq!(ip, old);
}

#[test]
fn test_conversion() {
    let old = std::net::SocketAddr::new([127, 0, 0, 1].into(), 0x0208);
    let addr = rs_addr_to_sockaddr(old);
    unsafe {
        if cfg!(target_endian = "big") {
            assert_eq!(0x7f000001, addr.addr4.sin_addr.s_addr);
            assert_eq!(0x0208, addr.addr4.sin_port);
        } else if cfg!(target_endian = "little") {
            assert_eq!(0x0100007f, addr.addr4.sin_addr.s_addr);
            assert_eq!(0x0802, addr.addr4.sin_port);
        } else {
            unreachable!();
        }
    };
    let ip = unsafe { sockaddr_to_rs_addr(&addr).unwrap() };
    assert_eq!(ip, old);

    let old = std::net::SocketAddr::new(std::net::Ipv6Addr::LOCALHOST.into(), 0x0208);
    let addr = rs_addr_to_sockaddr(old);
    let ip = unsafe { sockaddr_to_rs_addr(&addr).unwrap() };
    assert_eq!(ip, old);
}
