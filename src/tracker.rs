
use serde::{Deserialize, Serialize};
use peers::Peers;


#[derive(Debug, Clone, Serialize)]
pub struct TrackerRequest {
	/// the info hash of the torrent
	/// 20 bytes long, will need to be URL encoded
	/// Note: this is NOT the hexadecimal representation, which is 40 bytes long
	/// Note: We will manually pass this into the request so that it does not double encode.
	// pub info_hash: [u8; 20],

	/// a unique identifier for your client
	pub peer_id: String,
	/// the port your client is listening on
	pub port: u16,
	/// the total amount uploaded so far
	pub uploaded: usize,
	/// the total amount downloaded so far
	pub downloaded: usize,
	/// the number of bytes left to download
	pub left: usize,
	/// whether the peer list should use the compact representation
	/// The compact representation is more commonly used in the wild, the non-compact representation is mostly supported for backward-compatibility.
	pub compact: u8
}

#[derive(Debug, Clone, Deserialize)]
pub struct TrackerResponse {
	/// An integer, indicating how often your client should make a request to the tracker in seconds
	pub interval: u8,
	/// A string, which contains list of peers that your client can connect to.
	/// Each peer is represented using 6 bytes. The first 4 bytes are the peer's IP address and the last 2 bytes are the peer's port number.
	pub peers: Peers

}

pub mod peers {
    use serde::de::{self, Deserialize, Deserializer, Visitor};
	use serde::ser::{Serialize, Serializer};
    use std::fmt;
    use std::net::{Ipv4Addr, SocketAddrV4};

	#[derive(Debug, Clone)]
	pub struct Peers(pub Vec<SocketAddrV4>);
	struct PeersVisitor;

	impl<'de> Visitor<'de> for PeersVisitor {
		type Value = Peers;

		fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
			formatter.write_str("6 bytes. The first 4 are a peer's IP address and last 2 are a peer's port number")
		}

		fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
		where
			E: de::Error,
		{
			if v.len() % 6 != 0 {
				return Err(E::custom(format!("length is {}", v.len())));
			}
			Ok(Peers(
				v.chunks_exact(6)
					.map(|slice_6| SocketAddrV4::new(
						Ipv4Addr::new(slice_6[0], slice_6[1], slice_6[2], slice_6[3]),
						u16::from_be_bytes([slice_6[4], slice_6[5]]),
						)
					)	
					.collect()
			))
		}
	}

	impl<'de> Deserialize<'de> for Peers {
		fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
		where
			D: Deserializer<'de>,
		{
			deserializer.deserialize_bytes(PeersVisitor)
		}
	}

	impl Serialize for Peers {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let mut single_slice = Vec::with_capacity(6 * self.0.len());
			for peer in &self.0 {
				let ip = peer.ip().octets();
				let port = peer.port().to_be_bytes();
				single_slice.extend(&ip);
				single_slice.extend(&port);
			}
            serializer.serialize_bytes(&single_slice)
        }
    }
}