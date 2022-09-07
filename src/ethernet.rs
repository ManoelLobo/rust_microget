use rand::RngCore;
use std::fmt;
use std::fmt::Display;

use smoltcp::wire;

#[derive(Debug)]
pub struct MacAddress([u8; 6]);

impl Display for MacAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let octets = self.0;
        write!(
            f,
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            octets[0], octets[1], octets[2], octets[3], octets[4], octets[5]
        )
    }
}

impl MacAddress {
    pub fn new() -> Self {
        let mut octets = [0; 6];
        rand::thread_rng().fill_bytes(&mut octets);

        octets[0] |= 0b0000_0010;
        octets[0] &= 0b1111_1110;
        MacAddress(octets)
    }
}

impl Into<wire::EthernetAddress> for MacAddress {
    fn into(self) -> wire::EthernetAddress {
        wire::EthernetAddress(self.0)
    }
}
