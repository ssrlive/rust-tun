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
use std::mem;
use std::os::unix::io::{AsRawFd, IntoRawFd, RawFd};
use std::sync::Arc;

use crate::platform::posix::Fd;
use bytes::BufMut;
use libc;

use crate::PACKET_INFORMATION_LENGTH;

/// Read-only end for a file descriptor.
pub struct Reader(pub(crate) Arc<Fd>);

/// Write-only end for a file descriptor.
pub struct Writer(pub(crate) Arc<Fd>);

impl Read for Reader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        unsafe {
            let amount = libc::read(self.0.as_raw_fd(), buf.as_mut_ptr() as *mut _, buf.len());

            if amount < 0 {
                return Err(io::Error::last_os_error());
            }

            Ok(amount as usize)
        }
    }

    fn read_vectored(&mut self, bufs: &mut [io::IoSliceMut<'_>]) -> io::Result<usize> {
        unsafe {
            let mut msg: libc::msghdr = mem::zeroed();
            // msg.msg_name: NULL
            // msg.msg_namelen: 0
            msg.msg_iov = bufs.as_mut_ptr().cast();
            msg.msg_iovlen = bufs.len().min(libc::c_int::MAX as usize) as _;

            let n = libc::recvmsg(self.0.as_raw_fd(), &mut msg, 0);
            if n < 0 {
                return Err(io::Error::last_os_error());
            }

            Ok(n as usize)
        }
    }
}

impl Write for Writer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        unsafe {
            let amount = libc::write(self.0.as_raw_fd(), buf.as_ptr() as *const _, buf.len());

            if amount < 0 {
                return Err(io::Error::last_os_error());
            }

            Ok(amount as usize)
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn write_vectored(&mut self, bufs: &[io::IoSlice<'_>]) -> io::Result<usize> {
        unsafe {
            let mut msg: libc::msghdr = mem::zeroed();
            // msg.msg_name = NULL
            // msg.msg_namelen = 0
            msg.msg_iov = bufs.as_ptr() as *mut _;
            msg.msg_iovlen = bufs.len().min(libc::c_int::MAX as usize) as _;

            let n = libc::sendmsg(self.0.as_raw_fd(), &msg, 0);
            if n < 0 {
                return Err(io::Error::last_os_error());
            }

            Ok(n as usize)
        }
    }
}

impl AsRawFd for Reader {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}

impl AsRawFd for Writer {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}

pub struct Tun {
    pub(crate) reader: Reader,
    pub(crate) writer: Writer,
    pub(crate) mtu: usize,
    pub(crate) packet_information: bool,
    pub(crate) buf: Vec<u8>,
}

impl Tun {
    pub fn new(fd: Fd, mtu: usize, packet_information: bool) -> Self {
        let fd = Arc::new(fd);
        Self {
            reader: Reader(fd.clone()),
            writer: Writer(fd),
            mtu,
            packet_information,
            buf: vec![
                0;
                mtu + if packet_information {
                    PACKET_INFORMATION_LENGTH
                } else {
                    0
                }
            ],
        }
    }
    pub fn set_nonblock(&self) -> io::Result<()> {
        self.reader.0.set_nonblock()
    }
    pub fn set_mtu(&mut self, value: usize) {
        self.mtu = value;
        self.buf.reserve(
            value
                + if self.packet_information {
                    PACKET_INFORMATION_LENGTH
                } else {
                    0
                },
        );
    }
}

impl Read for Tun {
    fn read(&mut self, mut buf: &mut [u8]) -> io::Result<usize> {
        let slice = &mut self.buf[..];
        let size = self.reader.read(slice)?;
        let packet_pos = if self.packet_information {
            PACKET_INFORMATION_LENGTH
        } else {
            0
        };
        buf.put_slice(&self.buf[packet_pos..size]);
        Ok(size - packet_pos)
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
        let fd = self.reader.0.clone();
        drop(self.reader);
        drop(self.writer);
        // guarantee fd is the unique owner such that Arc::into_inner can return some
        let fd = Arc::into_inner(fd).unwrap(); //panic if accident
        fd.into_raw_fd()
    }
}
