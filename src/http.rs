use std::collections::BTreeMap;
use std::fmt::Display;
use std::net::IpAddr;
use std::os::unix::io::AsRawFd;

use smoltcp::iface::{EthernetInterfaceBuilder, NeighborCache, Routes};
use smoltcp::phy::{self, TapInterface};
use smoltcp::socket::{SocketSet, TcpSocket, TcpSocketBuffer};
use smoltcp::time::Instant;
use smoltcp::wire::{EthernetAddress, IpAddress, IpCidr, Ipv4Address};
use url::Url;

#[derive(Debug)]
enum HttpState {
    Connect,
    Request,
    Response,
}

#[derive(Debug)]
pub enum UpstreamError {
    Network(smoltcp::Error),
    InvalidUrl,
    Content(std::str::Utf8Error),
}

impl Display for UpstreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<smoltcp::Error> for UpstreamError {
    fn from(e: smoltcp::Error) -> Self {
        UpstreamError::Network(e)
    }
}

impl From<std::str::Utf8Error> for UpstreamError {
    fn from(e: std::str::Utf8Error) -> Self {
        UpstreamError::Content(e)
    }
}

fn random_port() -> u16 {
    49152 + rand::random::<u16>() % 16384
}

pub fn get(
    tap: TapInterface,
    mac: EthernetAddress,
    addr: IpAddr,
    url: Url,
) -> Result<(), UpstreamError> {
    let domain = url.host_str().ok_or(UpstreamError::InvalidUrl)?;

    let neighbor_cache = NeighborCache::new(BTreeMap::new());

    let tcp_rx_buffer = TcpSocketBuffer::new(vec![0; 1024]);
    let tcp_tx_buffer = TcpSocketBuffer::new(vec![0; 1024]);
    let tcp_socket = TcpSocket::new(tcp_rx_buffer, tcp_tx_buffer);

    let ip_addrs = [IpCidr::new(IpAddress::v4(192, 168, 42, 1), 24)];

    let fd = tap.as_raw_fd();
    let mut routes = Routes::new(BTreeMap::new());
    let default_gateway = Ipv4Address::new(192, 168, 42, 100);
    routes.add_default_ipv4_route(default_gateway).unwrap();

    let mut iface = EthernetInterfaceBuilder::new(tap)
        .ethernet_addr(mac)
        .neighbor_cache(neighbor_cache)
        .ip_addrs(ip_addrs)
        .routes(routes)
        .finalize();

    let mut sockets = SocketSet::new(vec![]);

    let tcp_handle = sockets.add(tcp_socket);

    let http_header = format!(
        "GET {} HTTP/1.0\r\nHost: {}\r\nConnection: close\r\n\r\n",
        url.path(),
        domain
    );

    let mut state = HttpState::Connect;

    'http: loop {
        let timestamp = Instant::now();

        match iface.poll(&mut sockets, timestamp) {
            Ok(_) => {}
            Err(smoltcp::Error::Unrecognized) => {}
            Err(e) => {
                eprintln!("Poll error: {:?}", e);
            }
        }

        {
            let mut socket = sockets.get::<TcpSocket>(tcp_handle);

            state = match state {
                HttpState::Connect if !socket.is_active() => {
                    eprintln!("Connecting to {}", addr);
                    socket.connect((addr, 80), random_port())?;

                    HttpState::Request
                }

                HttpState::Request if socket.may_send() => {
                    eprintln!("Sending HTTP request");
                    socket.send_slice(http_header.as_ref())?;

                    HttpState::Response
                }

                HttpState::Response if socket.can_recv() => {
                    socket.recv(|data| {
                        let output = String::from_utf8_lossy(data);
                        println!("{}", output);
                        (data.len(), ())
                    })?;

                    HttpState::Response
                }

                HttpState::Response if socket.may_recv() => {
                    eprintln!("Transmission complete");
                    break 'http;
                }

                _ => state,
            }
        }
        phy::wait(fd, iface.poll_delay(&sockets, timestamp)).expect("wait error");
    }

    Ok(())
}
