use std::{
    io,
    net::{IpAddr, SocketAddr},
    ops::Deref,
    os::fd::{AsRawFd, FromRawFd, IntoRawFd, OwnedFd},
};

use smol::net::UdpSocket;
use socket2::{Domain, Protocol, SockAddr, Socket, Type};

use crate::netns::run_in_netns;

#[derive(Debug, Clone)]
pub struct IcmpSocket(UdpSocket);

impl IcmpSocket {
    pub fn new(
        is_ipv4: bool,
        source: Option<IpAddr>,
        device: Option<impl AsRef<str>>,
        netns: impl AsRef<str>,
    ) -> io::Result<Self> {
        run_in_netns(&netns, || {
            let (domain, protocol) = if is_ipv4 {
                (Domain::IPV4, Some(Protocol::ICMPV4))
            } else {
                (Domain::IPV6, Some(Protocol::ICMPV6))
            };
            let socket = Socket::new(domain, Type::DGRAM, protocol)?;

            socket.set_nonblocking(true)?;

            if let Some(source) = source {
                socket.bind(&SockAddr::from(SocketAddr::new(source, 0)))?;
            }

            socket.bind_device(device.as_ref().map(|device| device.as_ref().as_bytes()))?;

            UdpSocket::try_from(unsafe { OwnedFd::from_raw_fd(socket.into_raw_fd()) })
        })?
        .map(Self)
    }

    pub fn set_ttl(&self, ttl: u32) -> io::Result<()> {
        let socket = unsafe { Socket::from_raw_fd(self.0.as_raw_fd()) };
        let result = socket.set_ttl_v4(ttl).and(socket.set_unicast_hops_v6(ttl));

        socket.into_raw_fd();
        result
    }
}

impl Deref for IcmpSocket {
    type Target = UdpSocket;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
