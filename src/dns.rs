use std::error::Error;
use std::fmt;
use std::fmt::Display;
use std::net::UdpSocket;

use trust_dns::op::{Message, MessageType, OpCode, Query};
use trust_dns::proto::error::ProtoError;
use trust_dns::rr::{Name, RecordType};
use trust_dns::serialize::binary::{BinEncodable, BinEncoder};

fn message_id() -> u16 {
    let candidate = rand::random();

    if candidate == 0 {
        return message_id();
    }

    candidate
}

#[derive(Debug)]
pub enum DnsError {
    ParseDomain(ProtoError),
    ParseDnsServerAddr(std::net::AddrParseError),
    Encoding(ProtoError),
    Decoding(ProtoError),
    Network(std::io::Error),
    Sending(std::io::Error),
    Receiving(std::io::Error),
}

impl Display for DnsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#?}", self)
    }
}

impl std::error::Error for DnsError {}

pub fn resolve(dns_server: &str, domain: &str) -> Result<Option<std::net::IpAddr>, Box<dyn Error>> {
    let domain = Name::from_ascii(domain).map_err(DnsError::ParseDomain)?;

    let dns_server: std::net::SocketAddr = format!("{}:53", dns_server)
        .parse()
        .map_err(DnsError::ParseDnsServerAddr)?;

    let mut request_buffer: Vec<u8> = Vec::with_capacity(64);
    let mut response_buffer: Vec<u8> = vec![0; 512];

    let mut request = Message::new();
    request.add_query(Query::query(domain, RecordType::A));

    request
        .set_id(message_id())
        .set_message_type(MessageType::Query)
        .set_recursion_desired(true)
        .set_op_code(OpCode::Query);

    let localhost = UdpSocket::bind("0.0.0.0:0").map_err(DnsError::Network)?;

    let timeout = std::time::Duration::from_secs(5);

    localhost
        .set_read_timeout(Some(timeout))
        .map_err(DnsError::Network)?;

    localhost
        .set_nonblocking(false)
        .map_err(DnsError::Network)?;

    let mut encoder = BinEncoder::new(&mut request_buffer);
    request.emit(&mut encoder).map_err(DnsError::Encoding)?;

    let _n_bytes_sent = localhost
        .send_to(&request_buffer, dns_server)
        .map_err(DnsError::Sending)?;

    loop {
        let (_b_bytes_recv, remote_port) = localhost
            .recv_from(&mut response_buffer)
            .map_err(DnsError::Receiving)?;

        if remote_port == dns_server {
            break;
        }
    }

    let response = Message::from_vec(&response_buffer).map_err(DnsError::Decoding)?;

    for answer in response.answers() {
        if answer.record_type() == RecordType::A {
            let resource = answer.rdata();
            let server_ip = resource.to_ip_addr().expect("Invalid Ip address received");
            return Ok(Some(server_ip));
        }
    }

    Ok(None)
}
