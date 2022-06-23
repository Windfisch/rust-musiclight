// vim: noet

use std::net::UdpSocket;
use std::net::SocketAddrV4;
use std::net::Ipv4Addr;

const MAX_PACKET_LEN: usize = 1470;
const TIMEOUT_SEC: u8 = 3;
const WLED_MODE_DRGBW: u8 = 3;

struct Command
{
	cmd:   u8,
	strip: u8,
	led:   u8,
	data: [u8; 4],
}

pub struct UdpProto
{
	socket:        UdpSocket,
	packet:        Vec<u8>,
}

impl UdpProto
{
	pub fn new(target_address: &str, num_leds_total: usize) -> std::io::Result<UdpProto>
	{
		let mut u = UdpProto {
			socket: UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0))?,
			packet: vec![0; 2 + 4*num_leds_total],
		};

		u.packet[0] = WLED_MODE_DRGBW;
		u.packet[1] = TIMEOUT_SEC;

		u.socket.connect(target_address)?;

		Ok(u)
	}

	pub fn set_color(&mut self, _strip: u8, led: usize,
		r: u8, g: u8, b: u8, w: u8) -> std::io::Result<()>
	{
		let offset = 2 + 4*led;
		if offset > self.packet.len() {
			Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "LED index out of range"))
		}
		else {
			self.packet[offset + 0] = r;
			self.packet[offset + 1] = g;
			self.packet[offset + 2] = b;
			self.packet[offset + 3] = w;
			Ok( () )
		}
	}

	pub fn commit(&mut self) -> std::io::Result<()>
	{
		self.socket.send(&self.packet)?;
		Ok( () )
	}
}
