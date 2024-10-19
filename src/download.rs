use hex::ToHex;
use sha1::{Sha1, Digest};
use tokio::net::TcpStream;
use crate::{peer::PeerMessage, torrent::{Info, Keys}};

// Many blocks form a piece
// Many pieces form a whole file
pub async fn download_whole_file(stream: &mut TcpStream, meta_info: &Info) -> Result<Vec<u8>, std::io::Error> {
    let mut file_data = Vec::new();
    let num_pieces = meta_info.pieces.0.len();
    eprintln!("Downloading file with {} pieces", num_pieces);
    for i in 0..num_pieces as u32 {
        eprintln!("Starting download for piece: {}", i);
        let piece_data = download_piece(stream, i, meta_info).await?;
        file_data.extend_from_slice(&piece_data);
        eprintln!("Finished download for piece: {}", i);
    }
    Ok(file_data)
} 


pub async fn download_piece(stream: &mut TcpStream, piece_index: u32, meta_info: &Info) -> Result<Vec<u8>, std::io::Error> {

    const BLOCK_SIZE: u32 = 16 << 10; // 16 KiB

    let piece_size = get_piece_size(piece_index, meta_info);
    let block_sizes = get_block_sizes(piece_size, BLOCK_SIZE);

    // Initialize a vector of vectors for each block
    //  [ 
    //    [], <--block_sizes[0] 
    //    [], <--block_sizes[1] 
    //    [], <--block_sizes[2] 
    // ]
    let mut piece_data: Vec<Vec<u8>> = block_sizes.iter().map(|&size| Vec::with_capacity(size as usize)).collect();
    let mut offset = 0;
    for &block_length in &block_sizes {
        PeerMessage::Request { 
            index: piece_index, 
            begin: offset, 
            length: block_length
        }.write( stream).await?;
        offset += block_length;
    }

    for i in 0..block_sizes.len() {
        let piece_message = PeerMessage::read(stream).await?;
        if let PeerMessage::Piece { index:_, begin: _, block } = piece_message {
            piece_data[i] = block;
        }
    }

    let piece = piece_data.into_iter().flatten().collect::<Vec<u8>>();
        
    // validate hash
    let mut hasher = Sha1::new();
    hasher.update(&piece);
    let piece_hash = hasher.finalize();
    let expected_piece_hash = &meta_info.pieces.0[piece_index as usize];

    assert_eq!(piece_hash.encode_hex::<String>(), expected_piece_hash.encode_hex::<String>());

    Ok(piece)

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