use why2025_badge_sys_bindings as abi;

use crate::ffi::{CStr, c_char, c_int, c_void};
use crate::io::{self, BorrowedCursor, IoSlice, IoSliceMut};
use crate::net::{Ipv4Addr, Ipv6Addr, Shutdown, SocketAddr, SocketAddrV4, ToSocketAddrs};
use crate::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use crate::sys::helpers::run_with_cstr;
use crate::sys::net::connection::each_addr;
use crate::time::Duration;
use crate::{fmt, mem, ptr};

const AF_INET: c_int = 2;
const SOCK_STREAM: c_int = 1;
const BACKLOG: c_int = 128;

#[repr(C)]
#[derive(Copy, Clone)]
struct SockaddrIn {
    sin_len: u8,
    sin_family: abi::sa_family_t,
    sin_port: u16,
    sin_addr: abi::in_addr,
    sin_zero: [c_char; 8],
}

#[derive(Debug)]
struct Socket(OwnedFd);

impl Socket {
    fn new_tcp() -> io::Result<Socket> {
        let fd = cvt(unsafe { abi::socket(AF_INET, SOCK_STREAM, 0) })?;
        Ok(Socket(unsafe { OwnedFd::from_raw_fd(fd) }))
    }

    fn raw(&self) -> c_int {
        self.0.as_raw_fd()
    }

    fn connect(&self, addr: &SocketAddr) -> io::Result<()> {
        let (addr, len) = socket_addr_to_c(addr)?;
        loop {
            let ret = unsafe { abi::connect(self.raw(), (&addr as *const SockaddrIn).cast(), len) };
            if ret >= 0 {
                return Ok(());
            }

            let err = io::Error::last_os_error();
            if err.is_interrupted() {
                continue;
            }
            return Err(err);
        }
    }

    fn bind(&self, addr: &SocketAddr) -> io::Result<()> {
        let (addr, len) = socket_addr_to_c(addr)?;
        cvt(unsafe { abi::bind(self.raw(), (&addr as *const SockaddrIn).cast(), len) }).map(drop)
    }

    fn listen(&self) -> io::Result<()> {
        cvt(unsafe { abi::listen(self.raw(), BACKLOG) }).map(drop)
    }

    fn accept(&self) -> io::Result<(Socket, SocketAddr)> {
        let mut addr = mem::MaybeUninit::<SockaddrIn>::zeroed();
        let mut len = size_of::<SockaddrIn>() as abi::socklen_t;
        let fd = cvt(unsafe { abi::accept(self.raw(), addr.as_mut_ptr().cast(), &mut len) })?;
        let socket = Socket(unsafe { OwnedFd::from_raw_fd(fd) });
        let addr = unsafe { socket_addr_from_c(addr.as_ptr(), len as usize)? };
        Ok((socket, addr))
    }

    fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        cvt_isize(unsafe { abi::read(self.raw(), buf.as_mut_ptr().cast::<c_void>(), buf.len()) })
            .map(|ret| ret as usize)
    }

    fn write(&self, buf: &[u8]) -> io::Result<usize> {
        cvt_isize(unsafe { abi::write(self.raw(), buf.as_ptr().cast::<c_void>(), buf.len()) })
            .map(|ret| ret as usize)
    }
}

fn socket_addr_to_c(addr: &SocketAddr) -> io::Result<(SockaddrIn, abi::socklen_t)> {
    match addr {
        SocketAddr::V4(addr) => Ok((
            SockaddrIn {
                sin_len: size_of::<SockaddrIn>() as u8,
                sin_family: AF_INET as abi::sa_family_t,
                sin_port: addr.port().to_be(),
                sin_addr: abi::in_addr { s_addr: u32::from_ne_bytes(addr.ip().octets()) },
                sin_zero: [0; 8],
            },
            size_of::<SockaddrIn>() as abi::socklen_t,
        )),
        SocketAddr::V6(_) => Err(io::Error::UNSUPPORTED_PLATFORM),
    }
}

unsafe fn socket_addr_from_c(addr: *const SockaddrIn, len: usize) -> io::Result<SocketAddr> {
    if len < size_of::<SockaddrIn>() {
        return Err(io::const_error!(io::ErrorKind::InvalidData, "short BadgeVMS socket address"));
    }

    let addr = unsafe { *addr };
    if addr.sin_family as c_int != AF_INET {
        return Err(io::Error::UNSUPPORTED_PLATFORM);
    }

    Ok(SocketAddr::V4(SocketAddrV4::new(
        Ipv4Addr::from(addr.sin_addr.s_addr.to_ne_bytes()),
        u16::from_be(addr.sin_port),
    )))
}

unsafe fn addrinfo_socket_addr(addr: *const abi::sockaddr, len: usize) -> io::Result<SocketAddr> {
    if addr.is_null() {
        return Err(io::const_error!(io::ErrorKind::InvalidData, "null BadgeVMS socket address"));
    }
    unsafe { socket_addr_from_c(addr.cast(), len) }
}

fn cvt(ret: c_int) -> io::Result<c_int> {
    if ret < 0 { Err(io::Error::last_os_error()) } else { Ok(ret) }
}

fn cvt_isize(ret: isize) -> io::Result<isize> {
    if ret < 0 { Err(io::Error::last_os_error()) } else { Ok(ret) }
}

fn unsupported<T>() -> io::Result<T> {
    Err(io::Error::UNSUPPORTED_PLATFORM)
}

#[derive(Debug)]
pub struct TcpStream {
    inner: Socket,
    peer: Option<SocketAddr>,
}

impl TcpStream {
    pub fn connect<A: ToSocketAddrs>(addr: A) -> io::Result<TcpStream> {
        each_addr(addr, |addr| {
            let socket = Socket::new_tcp()?;
            socket.connect(addr)?;
            Ok(TcpStream { inner: socket, peer: Some(*addr) })
        })
    }

    pub fn connect_timeout(_: &SocketAddr, _: Duration) -> io::Result<TcpStream> {
        unsupported()
    }

    pub fn set_read_timeout(&self, _: Option<Duration>) -> io::Result<()> {
        unsupported()
    }

    pub fn set_write_timeout(&self, _: Option<Duration>) -> io::Result<()> {
        unsupported()
    }

    pub fn read_timeout(&self) -> io::Result<Option<Duration>> {
        unsupported()
    }

    pub fn write_timeout(&self) -> io::Result<Option<Duration>> {
        unsupported()
    }

    pub fn peek(&self, _: &mut [u8]) -> io::Result<usize> {
        unsupported()
    }

    pub fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }

    pub fn read_buf(&self, cursor: BorrowedCursor<'_>) -> io::Result<()> {
        io::default_read_buf(|buf| self.read(buf), cursor)
    }

    pub fn read_vectored(&self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        io::default_read_vectored(|buf| self.read(buf), bufs)
    }

    #[inline]
    pub fn is_read_vectored(&self) -> bool {
        false
    }

    pub fn write(&self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    pub fn write_vectored(&self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        io::default_write_vectored(|buf| self.write(buf), bufs)
    }

    #[inline]
    pub fn is_write_vectored(&self) -> bool {
        false
    }

    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.peer.ok_or(io::Error::UNSUPPORTED_PLATFORM)
    }

    pub fn socket_addr(&self) -> io::Result<SocketAddr> {
        unsupported()
    }

    pub fn shutdown(&self, _: Shutdown) -> io::Result<()> {
        unsupported()
    }

    pub fn duplicate(&self) -> io::Result<TcpStream> {
        unsupported()
    }

    pub fn set_linger(&self, _: Option<Duration>) -> io::Result<()> {
        unsupported()
    }

    pub fn linger(&self) -> io::Result<Option<Duration>> {
        unsupported()
    }

    pub fn set_keepalive(&self, _: bool) -> io::Result<()> {
        unsupported()
    }

    pub fn keepalive(&self) -> io::Result<bool> {
        unsupported()
    }

    pub fn set_nodelay(&self, _: bool) -> io::Result<()> {
        unsupported()
    }

    pub fn nodelay(&self) -> io::Result<bool> {
        unsupported()
    }

    pub fn set_ttl(&self, _: u32) -> io::Result<()> {
        unsupported()
    }

    pub fn ttl(&self) -> io::Result<u32> {
        unsupported()
    }

    pub fn take_error(&self) -> io::Result<Option<io::Error>> {
        unsupported()
    }

    pub fn set_nonblocking(&self, _: bool) -> io::Result<()> {
        unsupported()
    }
}

#[derive(Debug)]
pub struct TcpListener {
    inner: Socket,
    local: Option<SocketAddr>,
}

impl TcpListener {
    pub fn bind<A: ToSocketAddrs>(addr: A) -> io::Result<TcpListener> {
        each_addr(addr, |addr| {
            let socket = Socket::new_tcp()?;
            socket.bind(addr)?;
            socket.listen()?;
            let local = match addr {
                SocketAddr::V4(addr) if addr.port() != 0 => Some(SocketAddr::V4(*addr)),
                _ => None,
            };
            Ok(TcpListener { inner: socket, local })
        })
    }

    pub fn socket_addr(&self) -> io::Result<SocketAddr> {
        self.local.ok_or(io::Error::UNSUPPORTED_PLATFORM)
    }

    pub fn accept(&self) -> io::Result<(TcpStream, SocketAddr)> {
        let (socket, addr) = self.inner.accept()?;
        Ok((TcpStream { inner: socket, peer: Some(addr) }, addr))
    }

    pub fn duplicate(&self) -> io::Result<TcpListener> {
        unsupported()
    }

    pub fn set_ttl(&self, _: u32) -> io::Result<()> {
        unsupported()
    }

    pub fn ttl(&self) -> io::Result<u32> {
        unsupported()
    }

    pub fn set_only_v6(&self, _: bool) -> io::Result<()> {
        unsupported()
    }

    pub fn only_v6(&self) -> io::Result<bool> {
        unsupported()
    }

    pub fn take_error(&self) -> io::Result<Option<io::Error>> {
        unsupported()
    }

    pub fn set_nonblocking(&self, _: bool) -> io::Result<()> {
        unsupported()
    }
}

pub struct UdpSocket(!);

impl UdpSocket {
    pub fn bind<A: ToSocketAddrs>(_: A) -> io::Result<UdpSocket> {
        unsupported()
    }

    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.0
    }

    pub fn socket_addr(&self) -> io::Result<SocketAddr> {
        self.0
    }

    pub fn recv_from(&self, _: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        self.0
    }

    pub fn peek_from(&self, _: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        self.0
    }

    pub fn send_to(&self, _: &[u8], _: &SocketAddr) -> io::Result<usize> {
        self.0
    }

    pub fn duplicate(&self) -> io::Result<UdpSocket> {
        self.0
    }

    pub fn set_read_timeout(&self, _: Option<Duration>) -> io::Result<()> {
        self.0
    }

    pub fn set_write_timeout(&self, _: Option<Duration>) -> io::Result<()> {
        self.0
    }

    pub fn read_timeout(&self) -> io::Result<Option<Duration>> {
        self.0
    }

    pub fn write_timeout(&self) -> io::Result<Option<Duration>> {
        self.0
    }

    pub fn set_broadcast(&self, _: bool) -> io::Result<()> {
        self.0
    }

    pub fn broadcast(&self) -> io::Result<bool> {
        self.0
    }

    pub fn set_multicast_loop_v4(&self, _: bool) -> io::Result<()> {
        self.0
    }

    pub fn multicast_loop_v4(&self) -> io::Result<bool> {
        self.0
    }

    pub fn set_multicast_ttl_v4(&self, _: u32) -> io::Result<()> {
        self.0
    }

    pub fn multicast_ttl_v4(&self) -> io::Result<u32> {
        self.0
    }

    pub fn set_multicast_loop_v6(&self, _: bool) -> io::Result<()> {
        self.0
    }

    pub fn multicast_loop_v6(&self) -> io::Result<bool> {
        self.0
    }

    pub fn join_multicast_v4(&self, _: &Ipv4Addr, _: &Ipv4Addr) -> io::Result<()> {
        self.0
    }

    pub fn join_multicast_v6(&self, _: &Ipv6Addr, _: u32) -> io::Result<()> {
        self.0
    }

    pub fn leave_multicast_v4(&self, _: &Ipv4Addr, _: &Ipv4Addr) -> io::Result<()> {
        self.0
    }

    pub fn leave_multicast_v6(&self, _: &Ipv6Addr, _: u32) -> io::Result<()> {
        self.0
    }

    pub fn set_ttl(&self, _: u32) -> io::Result<()> {
        self.0
    }

    pub fn ttl(&self) -> io::Result<u32> {
        self.0
    }

    pub fn take_error(&self) -> io::Result<Option<io::Error>> {
        self.0
    }

    pub fn set_nonblocking(&self, _: bool) -> io::Result<()> {
        self.0
    }

    pub fn recv(&self, _: &mut [u8]) -> io::Result<usize> {
        self.0
    }

    pub fn peek(&self, _: &mut [u8]) -> io::Result<usize> {
        self.0
    }

    pub fn send(&self, _: &[u8]) -> io::Result<usize> {
        self.0
    }

    pub fn connect<A: ToSocketAddrs>(&self, _: A) -> io::Result<()> {
        self.0
    }
}

impl fmt::Debug for UdpSocket {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0
    }
}

pub struct LookupHost {
    original: *mut abi::addrinfo,
    cur: *mut abi::addrinfo,
    port: u16,
}

impl Iterator for LookupHost {
    type Item = SocketAddr;

    fn next(&mut self) -> Option<SocketAddr> {
        loop {
            let cur = unsafe { self.cur.as_ref()? };
            self.cur = cur.ai_next;
            if cur.ai_family != AF_INET || cur.ai_socktype != SOCK_STREAM {
                continue;
            }
            let Ok(mut addr) =
                (unsafe { addrinfo_socket_addr(cur.ai_addr, cur.ai_addrlen as usize) })
            else {
                continue;
            };
            addr.set_port(self.port);
            return Some(addr);
        }
    }
}

unsafe impl Sync for LookupHost {}
unsafe impl Send for LookupHost {}

impl Drop for LookupHost {
    fn drop(&mut self) {
        if !self.original.is_null() {
            unsafe { abi::freeaddrinfo(self.original) }
        }
    }
}

pub fn lookup_host(host: &str, port: u16) -> io::Result<LookupHost> {
    run_with_cstr(host.as_bytes(), &|c_host: &CStr| {
        let mut hints: abi::addrinfo = unsafe { mem::zeroed() };
        hints.ai_family = AF_INET;
        hints.ai_socktype = SOCK_STREAM;
        let mut res = ptr::null_mut();
        let ret = unsafe { abi::getaddrinfo(c_host.as_ptr(), ptr::null(), &hints, &mut res) };
        if ret != 0 {
            return Err(io::Error::new(
                io::ErrorKind::Uncategorized,
                format!("BadgeVMS getaddrinfo failed with code {ret}"),
            ));
        }
        Ok(LookupHost { original: res, cur: res, port })
    })
}
