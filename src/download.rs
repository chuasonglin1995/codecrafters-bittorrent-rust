use hex::ToHex;
use sha1::{Sha1, Digest};
use tokio::net::TcpStream;
use crate::{peer::PeerMessage, torrent::{Info, Keys}};

// Many blocks form a piece
// Many pieces form a whole file
pub async fn download_piece(mut stream: TcpStream, piece_index: u32, meta_info: &Info) -> Result<Vec<u8>, std::io::Error> {

    // expect first message to be a bitfield message
    assert!(matches!(
        PeerMessage::read(&mut stream).await?,
        PeerMessage::Bitfield(_)
    ));

    // send an interested message
    PeerMessage::Interested.write(&mut stream).await?;

    // expect the next message to be an unchoke message
    assert!(matches!(
        PeerMessage::read(&mut stream).await?,
        PeerMessage::Unchoke
    ));

    eprintln!("{:?}", meta_info);
    const BLOCK_SIZE: u32 = 16 << 10; // 16 KiB

    let piece_size = get_piece_size(piece_index, meta_info);
    let block_sizes = get_block_sizes(piece_size, BLOCK_SIZE);

    let mut piece_data = Vec::new();
    let mut offset = 0;
    for block_length in block_sizes {
        PeerMessage::Request { 
            index: piece_index, 
            begin: offset, 
            length: block_length
        }.write(&mut stream).await?;
        eprintln!("Requesting block - index: {}, begin: {}, length: {}", piece_index, offset, block_length);

        let piece_message = PeerMessage::read(&mut stream).await?;

        if let PeerMessage::Piece { index, begin, block } = piece_message {
            piece_data.extend_from_slice(&block);
            offset += block_length;
        }
    }
        
    // validate hash
    let mut hasher = Sha1::new();
    hasher.update(&piece_data);
    let piece_hash = hasher.finalize();
    let expected_piece_hash = &meta_info.pieces.0[piece_index as usize];

    assert_eq!(piece_hash.encode_hex::<String>(), expected_piece_hash.encode_hex::<String>());
    eprintln!("Validated!");

    Ok(piece_data)

}

pub fn get_block_sizes(piece_length: u32, block_size: u32) -> Vec<u32> {
    let mut block_sizes = Vec::new();
    let mut remaining_length = piece_length;

    while remaining_length > 0 {
        let current_block_size = std::cmp::min(block_size, remaining_length);
        block_sizes.push(current_block_size);
        remaining_length -= current_block_size;
    }

    block_sizes
}

pub fn get_piece_size(piece_index: u32, meta_info: &Info) -> u32 {

    let length = if let Keys::SingleFile { length } = &meta_info.keys {
        length
    } else {
        todo!()
    };

    let piece_length = meta_info.plength;
    let last_piece = length / piece_length; // rounded down even if its 1.9

    if piece_index == last_piece {
        length % piece_length
    } else {
        piece_length
    }

}