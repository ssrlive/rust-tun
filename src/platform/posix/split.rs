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

use std::io::{self, Read, Write};

use std::os::unix::io::{AsRawFd, IntoRawFd, RawFd};
use std::sync::Arc;

use crate::platform::posix::Fd;
use bytes::BufMut;
use libc;

use crate::PACKET_INFORMATION_LENGTH;

#[derive(Clone)]
pub(crate) struct TunInfo {
    mtu: usize,
    packet_information: bool,
}
/// Read-only end for a file descriptor.
pub struct Reader {
    pub(crate) fd: Arc<Fd>,
    pub(crate) offset: usize,
    pub(crate) buf: Vec<u8>,
}

impl Reader {
    pub(crate) fn set_mtu(&mut self, value: usize) {
        self.buf.reserve(value + self.offset);
    }
}

/// Write-only end for a file descriptor.
pub struct Writer {
    pub(crate) fd: Arc<Fd>,
    pub(crate) offset: usize,
    pub(crate) buf: Vec<u8>,
}

impl Read for Reader {
    fn read(&mut self, mut buf: &mut [u8]) -> io::Result<usize> {
        unsafe {
            let amount = libc::read(
                self.fd.as_raw_fd(),
                self.buf.as_mut_ptr() as *mut _,
                self.buf.len(),
            );

            if amount < 0 {
                return Err(io::Error::last_os_error());
            }
            let amount = amount as usize;
            buf.put_slice(&self.buf[self.offset..amount]);
            Ok(amount - self.offset)
        }
    }

    // default implementation is sufficient
    // fn read_vectored(&mut self, bufs: &mut [io::IoSliceMut<'_>]) -> io::Result<usize> {
    //     unsafe {
    //         let mut msg: libc::msghdr = mem::zeroed();
    //         // msg.msg_name: NULL
    //         // msg.msg_namelen: 0
    //         msg.msg_iov = bufs.as_mut_ptr().cast();
    //         msg.msg_iovlen = bufs.len().min(libc::c_int::MAX as usize) as _;

    //         let n = libc::recvmsg(self.fd.as_raw_fd(), &mut msg, 0);
    //         if n < 0 {
    //             return Err(io::Error::last_os_error());
    //         }

    //         Ok(n as usize)
    //     }
    // }
}

impl Write for Writer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let buf = if self.offset != 0 {
            if let Some(header) =
                crate::codec::generate_packet_information(true, crate::codec::is_ipv6(buf)?)
            {
                (&mut self.buf[..self.offset]).put_slice(header.as_ref());
                let len = self.offset + buf.len();
                (&mut self.buf[self.offset..len]).put_slice(buf);
                &self.buf[..len]
            } else {
                buf
            }
        } else {
            buf
        };
        unsafe {
            let amount = libc::write(self.fd.as_raw_fd(), buf.as_ptr() as *const _, buf.len());

            if amount < 0 {
                return Err(io::Error::last_os_error());
            }

            Ok(amount as usize - self.offset)
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }

    // default implementation is sufficient
    // fn write_vectored(&mut self, bufs: &[io::IoSlice<'_>]) -> io::Result<usize> {
    //     unsafe {
    //         let mut msg: libc::msghdr = mem::zeroed();
    //         // msg.msg_name = NULL
    //         // msg.msg_namelen = 0
    //         msg.msg_iov = bufs.as_ptr() as *mut _;
    //         msg.msg_iovlen = bufs.len().min(libc::c_int::MAX as usize) as _;

    //         let n = libc::sendmsg(self.fd.as_raw_fd(), &msg, 0);
    //         if n < 0 {
    //             return Err(io::Error::last_os_error());
    //         }

    //         Ok(n as usize)
    //     }
    // }
}

impl AsRawFd for Reader {
    fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }
}

impl AsRawFd for Writer {
    fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }
}

pub struct Tun {
    pub(crate) reader: Reader,
    pub(crate) writer: Writer,
    pub(crate) info: TunInfo,
}

impl Tun {
    pub(crate) fn new(fd: Fd, mtu: usize, packet_information: bool) -> Self {
        let fd = Arc::new(fd);
        let offset = if packet_information {
            PACKET_INFORMATION_LENGTH
        } else {
            0
        };
        Self {
            reader: Reader {
                fd: fd.clone(),
                offset,
                buf: vec![0; mtu + offset],
            },
            writer: Writer {
                fd,
                offset,
                buf: vec![0; mtu + offset],
            },
            info: TunInfo {
                mtu,
                packet_information,
            },
        }
    }
    pub fn set_nonblock(&self) -> io::Result<()> {
        self.reader.fd.set_nonblock()
    }
    pub fn set_mtu(&mut self, value: usize) {
        self.info.mtu = value;
        self.reader.set_mtu(value);
    }
    pub fn packet_information(&self) -> bool {
        self.info.packet_information
    }
}

impl Read for Tun {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.reader.read(buf)
    }

    fn read_vectored(&mut self, bufs: &mut [io::IoSliceMut<'_>]) -> io::Result<usize> {
        self.reader.read_vectored(bufs)
    }
}

impl Write for Tun {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.writer.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }

    fn write_vectored(&mut self, bufs: &[io::IoSlice<'_>]) -> io::Result<usize> {
        self.writer.write_vectored(bufs)
    }
}
impl AsRawFd for Tun {
    fn as_raw_fd(&self) -> RawFd {
        self.reader.as_raw_fd()
    }
}

impl IntoRawFd for Tun {
    fn into_raw_fd(self) -> RawFd {
        let fd = self.reader.fd.clone();
        drop(self.reader);
        drop(self.writer);
        // guarantee fd is the unique owner such that Arc::into_inner can return some
        let fd = Arc::into_inner(fd).unwrap(); //panic if accident
        fd.into_raw_fd()
    }
}
