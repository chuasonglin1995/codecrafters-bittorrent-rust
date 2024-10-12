use serde::Serialize;
use tokio::net::TcpStream;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};

#[derive(Debug, Clone, Serialize)]
pub struct Handshake {
	pub protocol_str: String,
	pub reserved: [u8; 8],
	pub info_hash: [u8; 20],
	pub peer_id:  [u8; 20],
}

// Generate the handshake message
impl Handshake {
	pub fn new(info_hash: [u8; 20], peer_id: [u8; 20]) -> Self {
		let protocol_str = String::from("BitTorrent protocol");
		let reserved = [0u8; 8];

		Handshake {
			protocol_str,
			reserved,
			info_hash,
			peer_id,
		}
	}

	pub fn to_bytes_message(&self) -> Vec<u8> {
		let mut bytes_message = Vec::with_capacity(1+19+8+20+20);
		bytes_message.push(self.protocol_str.len() as u8);
		bytes_message.extend_from_slice(self.protocol_str.as_bytes());
		bytes_message.extend_from_slice(&self.reserved);
		bytes_message.extend_from_slice(&self.info_hash);
		bytes_message.extend_from_slice(&self.peer_id);

		bytes_message
	}

	pub fn from_bytes(bytes: &[u8]) -> io::Result<Self> {
        if bytes.len() < 68 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid handshake length"));
        }

        let protocol_len = bytes[0] as usize;
        let protocol_str = String::from_utf8(bytes[1..1 + protocol_len].to_vec())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid protocol string"))?;
        let reserved: [u8; 8] = bytes[1 + protocol_len..9 + protocol_len].try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid reserved bytes"))?;
        let info_hash: [u8; 20] = bytes[9 + protocol_len..29 + protocol_len].try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid info hash"))?;
        let peer_id: [u8; 20] = bytes[29 + protocol_len..49 + protocol_len].try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid peer ID"))?;

        Ok(Handshake {
            protocol_str,
            reserved,
            info_hash,
            peer_id,
        })
    }
}

pub async fn send_handshake(addr: &str, info_hash: [u8; 20], peer_id: [u8; 20]) -> io::Result<Handshake> {
	let handshake = Handshake::new(info_hash, peer_id);
	let handshake_bytes_message = handshake.to_bytes_message();

	let mut stream = TcpStream::connect(addr).await?;
	stream.write_all(&handshake_bytes_message).await?;

    // Read the response from the peer
    let mut response = vec![0; 68]; // Handshake response is 68 bytes
    stream.read_exact(&mut response).await?;

    // Parse the response into a Handshake instance using serde
    let response_handshake = Handshake::from_bytes(&response)?;

	Ok(response_handshake)
}
