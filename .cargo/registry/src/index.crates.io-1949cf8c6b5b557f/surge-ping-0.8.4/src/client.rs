#[cfg(unix)]
use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};
#[cfg(windows)]
use std::os::windows::io::{AsRawSocket, FromRawSocket, IntoRawSocket, RawSocket};

use std::{
    collections::HashMap,
    io,
    net::{IpAddr, SocketAddr},
    sync::atomic::{AtomicBool, Ordering},
    sync::Arc,
    time::Instant,
};

use parking_lot::Mutex;
use socket2::{Domain, Protocol, Socket, Type as SockType};
use tokio::{
    net::UdpSocket,
    sync::oneshot,
    task::{self, JoinHandle},
};
use tracing::debug;

use crate::{
    config::Config,
    icmp::{icmpv4::Icmpv4Packet, icmpv6::Icmpv6Packet},
    IcmpPacket, PingIdentifier, PingSequence, Pinger, SurgeError, ICMP,
};

// Check, if the platform's socket operates with ICMP packets in a casual way
#[macro_export]
macro_rules! is_linux_icmp_socket {
    ($sock_type:expr) => {
        if ($sock_type == socket2::Type::DGRAM
            && cfg!(not(any(target_os = "linux", target_os = "android"))))
            || $sock_type == socket2::Type::RAW
        {
            false
        } else {
            true
        }
    };
}

#[derive(Clone)]
pub struct AsyncSocket {
    inner: Arc<UdpSocket>,
    sock_type: SockType,
}

impl AsyncSocket {
    pub fn new(config: &Config) -> io::Result<Self> {
        let (sock_type, socket) = Self::create_socket(config)?;

        socket.set_nonblocking(true)?;
        if let Some(sock_addr) = &config.bind {
            socket.bind(sock_addr)?;
        }
        #[cfg(any(target_os = "android", target_os = "fuchsia", target_os = "linux"))]
        if let Some(interface) = &config.interface {
            socket.bind_device(Some(interface.as_bytes()))?;
        }
        #[cfg(any(
            target_os = "ios",
            target_os = "visionos",
            target_os = "macos",
            target_os = "tvos",
            target_os = "watchos",
            target_os = "illumos",
            target_os = "solaris",
            target_os = "linux",
            target_os = "android",
        ))]
        {
            if config.interface_index.is_some() {
                match config.kind {
                    ICMP::V4 => socket.bind_device_by_index_v4(config.interface_index)?,
                    ICMP::V6 => socket.bind_device_by_index_v6(config.interface_index)?,
                }
            }
        }
        if let Some(ttl) = config.ttl {
            match config.kind {
                ICMP::V4 => socket.set_ttl_v4(ttl)?,
                ICMP::V6 => socket.set_unicast_hops_v6(ttl)?,
            }
        }
        #[cfg(target_os = "freebsd")]
        if let Some(fib) = config.fib {
            socket.set_fib(fib)?;
        }
        #[cfg(windows)]
        let socket = UdpSocket::from_std(unsafe {
            std::net::UdpSocket::from_raw_socket(socket.into_raw_socket())
        })?;
        #[cfg(unix)]
        let socket =
            UdpSocket::from_std(unsafe { std::net::UdpSocket::from_raw_fd(socket.into_raw_fd()) })?;
        Ok(Self {
            inner: Arc::new(socket),
            sock_type,
        })
    }

    fn create_socket(config: &Config) -> io::Result<(SockType, Socket)> {
        let (domain, proto) = match config.kind {
            ICMP::V4 => (Domain::IPV4, Some(Protocol::ICMPV4)),
            ICMP::V6 => (Domain::IPV6, Some(Protocol::ICMPV6)),
        };

        match Socket::new(domain, config.sock_type_hint, proto) {
            Ok(sock) => Ok((config.sock_type_hint, sock)),
            Err(err) => {
                let new_type = if config.sock_type_hint == SockType::DGRAM {
                    SockType::RAW
                } else {
                    SockType::DGRAM
                };

                debug!(
                    "error opening {:?} type socket, trying {:?}: {:?}",
                    config.sock_type_hint, new_type, err
                );

                Ok((new_type, Socket::new(domain, new_type, proto)?))
            }
        }
    }

    pub async fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        self.inner.recv_from(buf).await
    }

    pub async fn send_to(&self, buf: &mut [u8], target: &SocketAddr) -> io::Result<usize> {
        self.inner.send_to(buf, target).await
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.inner.local_addr()
    }

    pub fn get_type(&self) -> SockType {
        self.sock_type
    }

    #[cfg(unix)]
    pub fn get_native_sock(&self) -> RawFd {
        self.inner.as_raw_fd()
    }

    #[cfg(windows)]
    pub fn get_native_sock(&self) -> RawSocket {
        self.inner.as_raw_socket()
    }
}

#[derive(PartialEq, Eq, Hash)]
struct ReplyToken(IpAddr, Option<PingIdentifier>, PingSequence);

pub(crate) struct Reply {
    pub timestamp: Instant,
    pub packet: IcmpPacket,
}

#[derive(Clone)]
pub(crate) struct ReplyMap {
    inner: Arc<Mutex<HashMap<ReplyToken, oneshot::Sender<Reply>>>>,
    alive: Arc<AtomicBool>,
}

impl Default for ReplyMap {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            alive: Arc::new(AtomicBool::new(true)),
        }
    }
}

impl ReplyMap {
    /// Register to wait for a reply from host with ident and sequence number.
    /// If there is already someone waiting for this specific reply then an
    /// error is returned.
    pub fn new_waiter(
        &self,
        host: IpAddr,
        ident: Option<PingIdentifier>,
        seq: PingSequence,
    ) -> Result<oneshot::Receiver<Reply>, SurgeError> {
        if !self.alive.load(Ordering::Relaxed) {
            return Err(SurgeError::ClientDestroyed);
        }
        let (tx, rx) = oneshot::channel();
        if self
            .inner
            .lock()
            .insert(ReplyToken(host, ident, seq), tx)
            .is_some()
        {
            return Err(SurgeError::IdenticalRequests { host, ident, seq });
        }
        Ok(rx)
    }

    /// Remove a waiter.
    pub(crate) fn remove(
        &self,
        host: IpAddr,
        ident: Option<PingIdentifier>,
        seq: PingSequence,
    ) -> Option<oneshot::Sender<Reply>> {
        self.inner.lock().remove(&ReplyToken(host, ident, seq))
    }

    /// Mark the client as destroyed. This is called when the Client is dropped.
    pub(crate) fn mark_destroyed(&self) {
        self.alive.store(false, Ordering::Relaxed);
    }
}

///
/// If you want to pass the `Client` in the task, please wrap it with `Arc`: `Arc<Client>`.
/// and can realize the simultaneous ping of multiple addresses when only one `socket` is created.
///
#[derive(Clone)]
pub struct Client {
    socket: AsyncSocket,
    reply_map: ReplyMap,
    recv: Arc<JoinHandle<()>>,
}

impl Drop for Client {
    fn drop(&mut self) {
        // Mark the reply_map as destroyed so any pending or new ping operations
        // will fail with ClientDestroyed error instead of timing out.
        self.reply_map.mark_destroyed();
        // The client may pass through multiple tasks, so need to judge whether the number of references is 1.
        if Arc::strong_count(&self.recv) <= 1 {
            self.recv.abort();
        }
    }
}

impl Client {
    /// A client is generated according to the configuration. In fact, a `AsyncSocket` is wrapped inside,
    /// and you can clone to any `task` at will.
    pub fn new(config: &Config) -> io::Result<Self> {
        let socket = AsyncSocket::new(config)?;
        let reply_map = ReplyMap::default();
        let recv = task::spawn(recv_task(socket.clone(), reply_map.clone()));
        Ok(Self {
            socket,
            reply_map,
            recv: Arc::new(recv),
        })
    }

    /// Create a `Pinger` instance, you can make special configuration for this instance.
    pub async fn pinger(&self, host: IpAddr, ident: PingIdentifier) -> Pinger {
        Pinger::new(host, ident, self.socket.clone(), self.reply_map.clone())
    }

    /// Expose the underlying socket, if user wants to modify any options on it
    pub fn get_socket(&self) -> AsyncSocket {
        self.socket.clone()
    }
}

async fn recv_task(socket: AsyncSocket, reply_map: ReplyMap) {
    let mut buf = [0; 2048];
    loop {
        if let Ok((sz, addr)) = socket.recv_from(&mut buf).await {
            let timestamp = Instant::now();
            let message = &buf[..sz];
            let local_addr = socket.local_addr().unwrap().ip();
            let packet = {
                let result = match addr.ip() {
                    IpAddr::V4(src_addr) => {
                        let local_addr_ip4 = match local_addr {
                            IpAddr::V4(local_addr_ip4) => local_addr_ip4,
                            _ => continue,
                        };

                        Icmpv4Packet::decode(message, socket.sock_type, src_addr, local_addr_ip4)
                            .map(IcmpPacket::V4)
                    }
                    IpAddr::V6(src_addr) => {
                        Icmpv6Packet::decode(message, src_addr).map(IcmpPacket::V6)
                    }
                };
                match result {
                    Ok(packet) => packet,
                    Err(err) => {
                        debug!("error decoding ICMP packet: {:?}", err);
                        continue;
                    }
                }
            };

            let ident = if is_linux_icmp_socket!(socket.get_type()) {
                None
            } else {
                Some(packet.get_identifier())
            };

            if let Some(waiter) = reply_map.remove(addr.ip(), ident, packet.get_sequence()) {
                // If send fails the receiving end has closed. Nothing to do.
                let _ = waiter.send(Reply { timestamp, packet });
            } else {
                debug!("no one is waiting for ICMP packet ({:?})", packet);
            }
        }
    }
}
