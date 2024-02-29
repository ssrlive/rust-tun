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

use libc::{
    self, c_char, c_short, ifreq, AF_INET, IFF_RUNNING, IFF_UP, IFNAMSIZ, O_RDWR, SOCK_DGRAM,
};
use std::{
    ffi::{CStr, CString},
    io::{self, Read, Write},
    mem,
    net::IpAddr,
    os::unix::io::{AsRawFd, IntoRawFd, RawFd},
    ptr,
};

use crate::{
    configuration::{Configuration, Layer},
    device::AbstractDevice,
    error::{Error, Result},
    platform::bsd::sys::*,
    platform::posix::{self, Fd, SockAddr, Tun},
};

/// A TUN device using the TUN/TAP Linux driver.
pub struct Device {
    tun_name: String,
    tun: Tun,
    ctl: Fd,
    packet_information: bool,
}

impl AsRef<dyn AbstractDevice + 'static> for Device {
    fn as_ref(&self) -> &(dyn AbstractDevice + 'static) {
        self
    }
}

impl AsMut<dyn AbstractDevice + 'static> for Device {
    fn as_mut(&mut self) -> &mut (dyn AbstractDevice + 'static) {
        self
    }
}

impl Device {
    /// Create a new `Device` for the given `Configuration`.
    pub fn new(config: &Configuration) -> Result<Self> {
        let mut device = unsafe {
            let dev = match config.tun_name.as_ref() {
                Some(tun_name) => {
                    let tun_name = CString::new(tun_name.clone())?;

                    if tun_name.as_bytes_with_nul().len() > IFNAMSIZ {
                        return Err(Error::NameTooLong);
                    }

                    Some(tun_name)
                }

                None => Some(CString::new("tun0")?),
            };

            let mut req: ifreq = mem::zeroed();

            if let Some(dev) = dev.as_ref() {
                ptr::copy_nonoverlapping(
                    dev.as_ptr() as *const c_char,
                    req.ifr_name.as_mut_ptr(),
                    dev.as_bytes().len(),
                );
            }

            //let device_type: c_short = config.layer.unwrap_or(Layer::L3).into();

            let queues_num = config.queues.unwrap_or(1);
            if queues_num != 1 {
                return Err(Error::InvalidQueuesNumber);
            }

            // low bits
            //req.ifr_ifru.ifru_flags[0] = 1;

            //high bits
            //req.ifr_ifru.ifru_flags[1] = 1;

			let dev_name = dev.unwrap().into_string().unwrap();

            let tun = {
				let device = format!("/dev/{dev_name}\0");
                let fd = libc::open(device.as_ptr() as *const _, O_RDWR);
                let tun = Fd::new(fd).map_err(|_| io::Error::last_os_error())?;
                if let Err(err) = siocsifflags(tun.0, &mut req as *mut _ as *mut _) {
                    dbg!("error in 96");
                    return Err(io::Error::from(err).into());
                }
                println!("{req:?}");
                tun
            };

            let mtu = config.mtu.unwrap_or(crate::DEFAULT_MTU);

            let ctl = Fd::new(libc::socket(AF_INET, SOCK_DGRAM, 0))?;

            let tun_name = CStr::from_ptr(req.ifr_name.as_ptr())
                .to_string_lossy()
                .to_string();
            Device {
                tun_name,
                tun: Tun::new(tun, mtu, false),
                ctl,
                packet_information: false,
            }
        };

        device.configure(config)?;

        Ok(device)
    }

    /// Prepare a new request.
    unsafe fn request(&self) -> ifreq {
        let mut req: ifreq = mem::zeroed();
        ptr::copy_nonoverlapping(
            self.tun_name.as_ptr() as *const c_char,
            req.ifr_name.as_mut_ptr(),
            self.tun_name.len(),
        );

        req
    }

    /// Split the interface into a `Reader` and `Writer`.
    pub fn split(self) -> (posix::Reader, posix::Writer) {
        (self.tun.reader, self.tun.writer)
    }

    /// Set non-blocking mode
    pub fn set_nonblock(&self) -> io::Result<()> {
        self.tun.set_nonblock()
    }
}

impl Read for Device {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.tun.read(buf)
    }

    fn read_vectored(&mut self, bufs: &mut [io::IoSliceMut<'_>]) -> io::Result<usize> {
        self.tun.read_vectored(bufs)
    }
}

impl Write for Device {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.tun.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.tun.flush()
    }

    fn write_vectored(&mut self, bufs: &[io::IoSlice<'_>]) -> io::Result<usize> {
        self.tun.write_vectored(bufs)
    }
}

impl AbstractDevice for Device {
    fn tun_name(&self) -> Result<String> {
        Ok(self.tun_name.clone())
    }

    fn set_tun_name(&mut self, _value: &str) -> Result<()> {
        Err(Error::InvalidName)
    }

    fn enabled(&mut self, value: bool) -> Result<()> {
        println!("invoke enabled");
        unsafe {
            let mut req = self.request();

            if let Err(err) = siocgifflags(self.ctl.as_raw_fd(), &mut req) {
                return Err(io::Error::from(err).into());
            }

            if value {
                req.ifr_ifru.ifru_flags[0] |= (IFF_UP | IFF_RUNNING) as c_short;
            } else {
                req.ifr_ifru.ifru_flags[0] &= !(IFF_UP as c_short);
            }

            if let Err(err) = siocsifflags(self.ctl.as_raw_fd(), &req) {
                return Err(io::Error::from(err).into());
            }

            Ok(())
        }
    }

    fn address(&self) -> Result<IpAddr> {
        unsafe {
            let mut req = self.request();

            if let Err(err) = siocgifaddr(self.ctl.as_raw_fd(), &mut req) {
                return Err(io::Error::from(err).into());
            }

            Ok(IpAddr::V4(
                SockAddr::new(&req.ifr_ifru.ifru_addr).map(Into::into)?,
            ))
        }
    }

    fn set_address(&mut self, value: IpAddr) -> Result<()> {
        println!("set_address");
        let IpAddr::V4(value) = value else {
            unimplemented!("do not support IPv6 yet")
        };
        unsafe {
            let mut req = self.request();
            req.ifr_ifru.ifru_addr = SockAddr::from(value).into();

            if let Err(err) = siocsifaddr(self.ctl.as_raw_fd(), &req) {
                return Err(io::Error::from(err).into());
            }

            Ok(())
        }
    }

    fn destination(&self) -> Result<IpAddr> {
        unsafe {
            let mut req = self.request();

            if let Err(err) = siocgifdstaddr(self.ctl.as_raw_fd(), &mut req) {
                return Err(io::Error::from(err).into());
            }

            Ok(IpAddr::V4(
                SockAddr::new(&req.ifr_ifru.ifru_dstaddr).map(Into::into)?,
            ))
        }
    }

    fn set_destination(&mut self, value: IpAddr) -> Result<()> {
        println!("set_destination");
        let IpAddr::V4(value) = value else {
            unimplemented!("do not support IPv6 yet")
        };
        unsafe {
            let mut req = self.request();
            req.ifr_ifru.ifru_dstaddr = SockAddr::from(value).into();

            if let Err(err) = siocsifdstaddr(self.ctl.as_raw_fd(), &req) {
                return Err(io::Error::from(err).into());
            }

            Ok(())
        }
    }

    fn broadcast(&self) -> Result<IpAddr> {
        unsafe {
            let mut req = self.request();

            if let Err(err) = siocgifbrdaddr(self.ctl.as_raw_fd(), &mut req) {
                return Err(io::Error::from(err).into());
            }

            Ok(IpAddr::V4(
                SockAddr::new(&req.ifr_ifru.ifru_broadaddr).map(Into::into)?,
            ))
        }
    }

    fn set_broadcast(&mut self, value: IpAddr) -> Result<()> {
        println!("set_broadcast");
        let IpAddr::V4(value) = value else {
            unimplemented!("do not support IPv6 yet")
        };
        unsafe {
            let mut req = self.request();
            req.ifr_ifru.ifru_broadaddr = SockAddr::from(value).into();

            if let Err(err) = siocsifbrdaddr(self.ctl.as_raw_fd(), &req) {
                return Err(io::Error::from(err).into());
            }

            Ok(())
        }
    }

    fn netmask(&self) -> Result<IpAddr> {
        unsafe {
            let mut req = self.request();

            if let Err(err) = siocgifnetmask(self.ctl.as_raw_fd(), &mut req) {
                return Err(io::Error::from(err).into());
            }

            Ok(IpAddr::V4(
                SockAddr::new(&req.ifr_ifru.ifru_addr).map(Into::into)?,
            ))
        }
    }

    fn set_netmask(&mut self, value: IpAddr) -> Result<()> {
        println!("set_netmask");
        let IpAddr::V4(value) = value else {
            unimplemented!("do not support IPv6 yet")
        };
        unsafe {
            let mut req = self.request();
            req.ifr_ifru.ifru_addr = SockAddr::from(value).into();

            if let Err(err) = siocsifnetmask(self.ctl.as_raw_fd(), &req) {
                return Err(io::Error::from(err).into());
            }

            Ok(())
        }
    }

    fn mtu(&self) -> Result<u16> {
        unsafe {
            let mut req = self.request();

            if let Err(err) = siocgifmtu(self.ctl.as_raw_fd(), &mut req) {
                return Err(io::Error::from(err).into());
            }

            req.ifr_ifru
                .ifru_mtu
                .try_into()
                .map_err(|_| Error::TryFromIntError)
        }
    }

    fn set_mtu(&mut self, value: u16) -> Result<()> {
        println!("set_mtu");
        unsafe {
            let mut req = self.request();
            req.ifr_ifru.ifru_mtu = value as i32;

            if let Err(err) = siocsifmtu(self.ctl.as_raw_fd(), &req) {
                return Err(io::Error::from(err).into());
            }
            self.tun.set_mtu(value);
            Ok(())
        }
    }

    fn packet_information(&self) -> bool {
        self.packet_information
    }
}

impl AsRawFd for Device {
    fn as_raw_fd(&self) -> RawFd {
        self.tun.as_raw_fd()
    }
}

impl IntoRawFd for Device {
    fn into_raw_fd(self) -> RawFd {
        self.tun.into_raw_fd()
    }
}

impl From<Layer> for c_short {
    fn from(layer: Layer) -> Self {
        match layer {
            Layer::L2 => 2,
            Layer::L3 => 3,
        }
    }
}
