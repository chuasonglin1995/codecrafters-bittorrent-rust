use serde::Serialize;
use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;

#[derive(Debug, Clone, Serialize)]
pub struct Handshake {
    pub protocol_str: String,
    pub reserved: [u8; 8],
    pub info_hash: [u8; 20],
    pub peer_id: [u8; 20],
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
        let mut bytes_message = Vec::with_capacity(1 + 19 + 8 + 20 + 20);
        bytes_message.push(self.protocol_str.len() as u8);
        bytes_message.extend_from_slice(self.protocol_str.as_bytes());
        bytes_message.extend_from_slice(&self.reserved);
        bytes_message.extend_from_slice(&self.info_hash);
        bytes_message.extend_from_slice(&self.peer_id);

        bytes_message
    }

    pub fn from_bytes(bytes: &[u8]) -> io::Result<Self> {
        if bytes.len() < 68 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid handshake length",
            ));
        }

        let protocol_len = bytes[0] as usize;
        let protocol_str = String::from_utf8(bytes[1..1 + protocol_len].to_vec())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid protocol string"))?;
        let reserved: [u8; 8] = bytes[1 + protocol_len..9 + protocol_len]
            .try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid reserved bytes"))?;
        let info_hash: [u8; 20] = bytes[9 + protocol_len..29 + protocol_len]
            .try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid info hash"))?;
        let peer_id: [u8; 20] = bytes[29 + protocol_len..49 + protocol_len]
            .try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid peer ID"))?;

        Ok(Handshake {
            protocol_str,
            reserved,
            info_hash,
            peer_id,
        })
    }
}

pub async fn send_handshake(
    addr: &str,
    info_hash: &[u8; 20],
    peer_id: [u8; 20],
) -> io::Result<Handshake> {
    let handshake = Handshake::new(*info_hash, peer_id);
    let handshake_bytes_message = handshake.to_bytes_message();

    let mut stream = TcpStream::connect(addr).await?;
    stream.write_all(&handshake_bytes_message).await?;
    stream.flush().await?;

    // Read the response from the peer
    let mut response = vec![0; 68]; // Handshake response is 68 bytes
    stream.read_exact(&mut response).await?;

    // Parse the response into a Handshake instance using serde
    let response_handshake = Handshake::from_bytes(&response)?;

    Ok(response_handshake)
}

#[derive(Debug)]
pub enum PeerMessage {
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    Have(u32),
    Bitfield(Vec<u8>),
    Request {
        index: u32,
        begin: u32,
        length: u32,
    },
    Piece {
        index: u32,
        begin: u32,
        block: Vec<u8>,
    },
    Cancel {
        index: u32,
        begin: u32,
        length: u32,
    },
}

pub async fn connect_to_peer(
    addr: &str,
    info_hash: &[u8; 20],
    peer_id: [u8; 20],
) -> io::Result<TcpStream> {
    let handshake = Handshake::new(*info_hash, peer_id);
    let handshake_bytes_message = handshake.to_bytes_message();

    let mut stream = TcpStream::connect(addr).await?;
    stream.write_all(&handshake_bytes_message).await?;
    stream.flush().await?;

    // Read the response from the peer
    let mut response = vec![0; 68]; // Handshake response is 68 bytes
    stream.read_exact(&mut response).await?;

    Ok(stream)
}

impl PeerMessage {
    const MSG_ID_CHOKE: u8 = 0;
    const MSG_ID_UNCHOKE: u8 = 1;
    const MSG_ID_INTERESTED: u8 = 2;
    const MSG_ID_NOT_INTERESTED: u8 = 3;
    const MSG_ID_HAVE: u8 = 4;
    const MSG_ID_BIT_FIELD: u8 = 5;
    const MSG_ID_REQUEST: u8 = 6;
    const MSG_ID_PIECE: u8 = 7;
    const MSG_ID_CANCEL: u8 = 8;

    pub async fn read<R: AsyncRead + Unpin>(reader: &mut R) -> Result<PeerMessage, std::io::Error> {
        let message_size = reader.read_u32().await?; // Read the length (4 bytes)
        let message_id = reader.read_u8().await?; // Read the message ID (1 byte)

        let message = match message_id {
            Self::MSG_ID_CHOKE => {
                eprintln!("Received Choke message");
                PeerMessage::Choke
            },
            Self::MSG_ID_UNCHOKE => {
                eprintln!("Received Unchoke message");
                PeerMessage::Unchoke
            },
            Self::MSG_ID_INTERESTED => {
                eprintln!("Received Interested message");
                PeerMessage::Interested
            },
            Self::MSG_ID_NOT_INTERESTED => {
                eprintln!("Received Not Interested message");
                PeerMessage::NotInterested
            },
            Self::MSG_ID_HAVE => {
                assert_eq!(message_size, 5, "Invalid have message size");
                let piece_id = reader.read_u32().await?;
                PeerMessage::Have(piece_id)
            }
            Self::MSG_ID_BIT_FIELD => {
                eprintln!("Received bitfield message");
                // Its payload is a bitfield with each index that downloader has sent set to one and the rest set to zero.
                // Downloaders which don't have anything yet may skip the 'bitfield' message.
                // The first byte of the bitfield corresponds to indices 0 - 7 from high bit to low bit, respectively.
                // The next one 8-15, etc. Spare bits at the end are set to zero.
                let block_length = message_size as usize - 1;
                let mut block = vec![0u8; block_length];
                reader.read_exact(&mut block).await?;
                PeerMessage::Bitfield(block)
            }
            Self::MSG_ID_REQUEST => {
                eprintln!("Received request message");
                let index = reader.read_u32().await?;
                let begin = reader.read_u32().await?;
                let length = reader.read_u32().await?;
                PeerMessage::Request {
                    index,
                    begin,
                    length,
                }
            }
            Self::MSG_ID_PIECE => {
                let index = reader.read_u32().await?;
                let begin = reader.read_u32().await?;
                let mut block = vec![0; (message_size - 9) as usize];
                reader.read_exact(&mut block).await?;
                eprintln!("index: {}, begin: {}", index, begin);
                PeerMessage::Piece {
                    index,
                    begin,
                    block,
                }
                // eprintln!("Received piece message");
                // let index = reader.read_u32().await?;
                // let begin = reader.read_u32().await?;
                // let block_length = message_size as usize - 9;
                // let mut block = vec![0u8; block_length];
                // eprintln!("Expected block length: {}", block_length);

                // reader.read_exact(&mut block).await?;
                // PeerMessage::Piece {
                //     index,
                //     begin,
                //     block,
                // }
            }
            Self::MSG_ID_CANCEL => {
                eprintln!("Received cancel message");
                let index = reader.read_u32().await?;
                let begin = reader.read_u32().await?;
                let length = reader.read_u32().await?;
                PeerMessage::Cancel {
                    index,
                    begin,
                    length,
                }
            }
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Invalid message ID",
                ))
            }
        };

        Ok(message)
    }

    pub async fn write<W: AsyncWrite + Unpin>(&self, writer: &mut W) -> Result<(), std::io::Error> {
        match &self {
            PeerMessage::Choke => {
                writer.write_u32(1).await?;
                writer.write_u8(Self::MSG_ID_CHOKE).await?;
                eprintln!("Sent Choke message");
            }
            PeerMessage::Unchoke => {
                writer.write_u32(1).await?;
                writer.write_u8(Self::MSG_ID_UNCHOKE).await?;
                eprintln!("Sent Unchoke message");
            }
            PeerMessage::Interested => {
                writer.write_u32(1).await?;
                writer.write_u8(Self::MSG_ID_INTERESTED).await?;
                eprintln!("Sent Interested message");

            }
            PeerMessage::NotInterested => {
                writer.write_u32(1).await?;
                writer.write_u8(Self::MSG_ID_NOT_INTERESTED).await?;
                eprintln!("Sent NotInterested message");
            }
            PeerMessage::Have(piece_id) => {
                writer.write_u32(5).await?;
                writer.write_u8(Self::MSG_ID_HAVE).await?;
                writer.write_u32(*piece_id).await?;
                eprintln!("Sent Have message");
            }
            PeerMessage::Bitfield(block) => {
                writer.write_u32((block.len() + 1) as u32).await?;
                writer.write_u8(Self::MSG_ID_BIT_FIELD).await?;
                writer.write_all(&block).await?;
                eprintln!("Sent Bitfield message");
            }
            PeerMessage::Request { index, begin, length } => {
                writer.write_u32(13).await?;
                writer.write_u8(Self::MSG_ID_REQUEST).await?;
                writer.write_u32(*index).await?;
                writer.write_u32(*begin).await?;
                writer.write_u32(*length).await?;
                eprintln!("Sent Request message");
            }
            PeerMessage::Piece { index, begin, block } => {
                writer.write_u32((block.len() + 9) as u32).await?;
                writer.write_u8(Self::MSG_ID_PIECE).await?;
                writer.write_u32(*index).await?;
                writer.write_u32(*begin).await?;
                writer.write_all(&block).await?;
                eprintln!("Sent Piece message");
            }
            PeerMessage::Cancel { index, begin, length } => {
                writer.write_u32(13).await?;
                writer.write_u8(Self::MSG_ID_CANCEL).await?;
                writer.write_u32(*index).await?;
                writer.write_u32(*begin).await?;
                writer.write_u32(*length).await?;
                eprintln!("Sent Cancel message");
            }
        };

        writer.flush().await?;
        Ok(())

    }
}


