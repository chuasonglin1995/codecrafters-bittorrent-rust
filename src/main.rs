use anyhow::Context;
use bittorrent_starter_rust::peer::send_handshake;
use bittorrent_starter_rust::torrent::{Keys, Torrent};
use bittorrent_starter_rust::tracker::{TrackerRequest, TrackerResponse};
use bittorrent_starter_rust::url_encode::url_encode;
use clap::{Parser, Subcommand};
use core::panic;
use serde_bencode;
use std::path::PathBuf;
use tokio;

#[derive(Parser, Debug)]
struct Args {
    #[command(subcommand)]
    command: Command
}

#[derive(Subcommand, Debug)]
#[clap(rename_all = "snake_case")]
enum Command {
    Decode {
        value: String,
    },
    Info {
        torrent: PathBuf,
    },
    Peers {
        torrent: PathBuf,
    },
    Handshake {
        torrent: PathBuf,
        peer_addr: String,
    },
    DownloadPiece {
        #[clap(short, long)]
        output: PathBuf,
        torrent: PathBuf,
        piece_index: u32
    }
}


#[allow(dead_code)]
fn decode_bencoded_value(encoded_value: &str) -> (serde_json::Value, &str) {

    match encoded_value.chars().next() {
        // decode integers eg. i52e
        Some('i') => {
            if let Some((n, rest)) = encoded_value.strip_prefix('i').unwrap().split_once('e') {
                 if let Ok(n) = n.parse::<i64>() {
                    return (n.into(), rest)
                 }
            } 
        }

        // decode lists eg. l5:helloe = ["hello"], li25el3:fooi-43ee5:helloe = [25,[ "foo", -43], "hello"]
        Some('l') => {
            let mut values = Vec::new();
            let mut rest = encoded_value.split_at(1).1;
            while !rest.is_empty() && rest.chars().next().unwrap() != 'e' {
                let (v, remainder) = decode_bencoded_value(rest);
                eprint!("v: {:?}, remainder: {:?}\n", v, remainder);
                values.push(v);
                rest = remainder;
            }
            return (values.into(), &rest[1..])
        }

        // decode dictionaries eg. d3:foo3:bar5:helloi52ee = {"foo": "bar", "hello": 52}
        Some('d') => {
            let mut dict = serde_json::Map::new();
            let mut rest = encoded_value.split_at(1).1;
            while !rest.is_empty() && rest.chars().next().unwrap() != 'e' {
                let (k, remainder) = decode_bencoded_value(rest);
                let k = match k {
                    serde_json::Value::String(k) => k,
                    k => {
                        panic!("dict keys must be strings, not {k:?}")
                    }
                };
                let (v, remainder) = decode_bencoded_value(remainder);
                eprint!("k: {k}, v {v}\n");
                dict.insert(k, v);
                rest = remainder;
            }
            return (dict.into(), &rest[1..])
        }

        // decode strings eg. 5:hello
        Some('0'..='9') => {
            if let Some((len, str)) = encoded_value.split_once(':') {
                if let Ok(len) = len.parse::<usize>() {
                    return (str[..len].to_string().into(), &str[len..]);
                }
            }
        }

        _ => {}
        
    }
    panic!("Unhandled encoded value: {}", encoded_value);

}

// Usage: sh ./your_bittorrent.sh decode "<encoded_value>"
// Usage: sh ./your_bittorrent.sh info sample.torrent
// Usage: sh ./your_bittorrent.sh peers sample.torrent
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.command {
        Command::Decode { value } => {
            let decoded_value = decode_bencoded_value(&value);
            println!("{}", decoded_value.0.to_string());
        }
        Command::Info { torrent } => {
            let dot_torrent = std::fs::read(torrent).context("read torrent file")?;
            let t: Torrent = serde_bencode::from_bytes(&dot_torrent).context("parse torrent file")?;
            eprintln!("{t:?}");
            println!("Tracker URL: {}", t.announce);
            if let Keys::SingleFile { length } = t.info.keys {
                println!("Length: {length}");
            }
            let info_hash = &t.info_hash();
            let hash_hex = hex::encode(info_hash);

            println!("Info Hash: {hash_hex}");
            println!("Piece Length: {}", t.info.plength);
            println!("Piece Hashes:");
            for hash in t.info.pieces.0 {
                println!("{}", hex::encode(&hash))
            };
        }

        Command::Peers { torrent } => {
            let dot_torrent = std::fs::read(torrent).context("read torrent file")?;
            let t: Torrent = serde_bencode::from_bytes(&dot_torrent).context("parse torrent file")?;
            let length = if let Keys::SingleFile { length } = t.info.keys {
                length
            } else {
                todo!()
            };

            let request = TrackerRequest {
                peer_id: String::from("00112233445566778899"),
                port: 6881,
                uploaded: 0,
                downloaded: 0,
                left: length,
                compact: 1
            };
            let url_params =
                serde_urlencoded::to_string(&request).context("url-encode tracker parameters")?;

            // put infohash here so that it wont get double url encoded
            let tracker_url = format!(
                "{}?{}&info_hash={}",
                t.announce,
                url_params,
                &url_encode(&t.info_hash())
            );

            eprintln!("{tracker_url}");
            let client = reqwest::blocking::Client::new();
            let tracker_response = client
                .get(tracker_url)
                .send()
                .context("send request to tracker")?
                .bytes()
                .context("convert response to bytes")?;

            eprintln!("{:?}", tracker_response);
            let response: TrackerResponse =
                serde_bencode::from_bytes(&tracker_response).context("parse tracker response")?;
            for peer in &response.peers.0 {
                println!("{}:{}", peer.ip(), peer.port());
            }
        }

        Command::Handshake { torrent, peer_addr } => {
            let dot_torrent = std::fs::read(torrent).context("read torrent file")?;
            let t: Torrent = serde_bencode::from_bytes(&dot_torrent).context("parse torrent file")?;
            let handshake_response = send_handshake(&peer_addr, t.info_hash(), *b"00112233445566778899").await?;

            eprintln!("{:?}", handshake_response);
            let peer_id_hex = hex::encode(handshake_response.peer_id);
            println!("Peer ID: {}", peer_id_hex);
        }

        // Usage: sh ./your_bittorrent.sh download_piece -o /tmp/test-piece-0 sample.torrent 0
        Command::DownloadPiece { output, torrent, piece_index } => {
            let dot_torrent = std::fs::read(torrent).context("read torrent file")?;
            let t: Torrent = serde_bencode::from_bytes(&dot_torrent).context("parse torrent file")?;


            // send handshake
            // wait for bitfield messsage (5)
            // send an interested message (2)
            // wait until we receive an unchoke message (1)
            // break the piece into blocks of 16kiB and send a request message for each block (6)
            // wait for a piece message for each block you requested (7)
        }
    }

    Ok(())
}
