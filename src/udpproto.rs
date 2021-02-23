// vim: noet

use std::net::UdpSocket;
use std::net::SocketAddrV4;
use std::net::Ipv4Addr;

const MAX_PACKET_LEN: usize = 1470;

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
	packet:        [u8; MAX_PACKET_LEN],
	packet_offset: usize,
}

impl UdpProto
{
	pub fn new(target_address: &str) -> std::io::Result<UdpProto>
	{
		let u = UdpProto {
			socket: UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0))?,
			packet: [0u8; MAX_PACKET_LEN],
			packet_offset: 0,
		};

		u.socket.connect(target_address)?;

		Ok(u)
	}

	fn send_packet(&mut self) -> std::io::Result<()>
	{
		if self.packet_offset == 0 {
			// nothing to do
			return Ok( () );
		}

		self.socket.send(&self.packet[0..self.packet_offset])?;

		self.packet_offset = 0;

		Ok( () )
	}

	fn add_command(&mut self, cmd: u8, strip: u8, led: u8, data: &[u8; 4]) -> std::io::Result<()>
	{
		// put the command into the packet buffer
		self.packet[self.packet_offset + 0] = cmd;
		self.packet[self.packet_offset + 1] = strip;
		self.packet[self.packet_offset + 2] = led;

		for i in 0 .. data.len() {
			self.packet[self.packet_offset + i + 3] = data[i];
		}

		self.packet_offset += 7;

		if self.packet_offset >= MAX_PACKET_LEN {
			self.send_packet()?;
		}

		Ok( () )
	}

	pub fn set_color(&mut self, strip: u8, led: u8,
		r: u8, g: u8, b: u8, w: u8) -> std::io::Result<()>
	{
		let data = [r,g,b,w];

		self.add_command(0x00, strip, led, &data)?;

		Ok( () )
	}

	pub fn fade_color(&mut self, strip: u8, led: u8,
		r: u8, g: u8, b: u8, w: u8) -> std::io::Result<()>
	{
		let data = [r,g,b,w];

		self.add_command(0x01, strip, led, &data)?;

		Ok( () )
	}

	pub fn add_color(&mut self, strip: u8, led: u8,
		r: u8, g: u8, b: u8, w: u8) -> std::io::Result<()>
	{
		let data = [r,g,b,w];

		self.add_command(0x02, strip, led, &data)?;

		Ok( () )
	}

	pub fn set_fadestep(&mut self, fadestep: u8) -> std::io::Result<()>
	{
		let data = [fadestep, 0, 0, 0];

		self.add_command(0x03, 0, 0, &data)?;

		Ok( () )
	}

	pub fn commit(&mut self) -> std::io::Result<()>
	{
		// add the END_OF_UPDATE command
		self.add_command(0xFE, 0, 0, &[0u8; 4])?;

		self.send_packet()
	}
}
