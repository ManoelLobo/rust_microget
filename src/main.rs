use std::net::Ipv4Addr;

use clap::{App, Arg};
use smoltcp::phy::TapInterface;
use url::Url;

mod dns;
mod ethernet;
mod http;

fn main() {
    let app = App::new("microget")
        .about("GET a webpage")
        .arg(Arg::with_name("url").required(true))
        .arg(Arg::with_name("tap-dev").required(true))
        .arg(Arg::with_name("dns-server").default_value("1.1.1.1"))
        .get_matches();

    let url_text = app.value_of("url").unwrap();
    let tap_dev_text = app.value_of("tap-dev").unwrap();
    let dns_server_text = app.value_of("dns-server").unwrap();

    let url = Url::parse(url_text).expect("Unable to parse URL");

    if url.scheme() != "http" {
        eprintln!("Only HTTP is supported");
        return;
    }

    let tap =
        TapInterface::new(tap_dev_text).expect("Unable to use TAP device as network interface");

    let domain = url.host_str().expect("Domain name is required");

    let dns_server: Ipv4Addr = dns_server_text
        .parse()
        .expect("Unable to parse DNS server address as an IPv4 address");

    let addr = dns::resolve(dns_server_text, domain).unwrap();

    let mac = ethernet::MacAddress::new().into();

    http::get(tap, mac, addr, url).unwrap();
}
