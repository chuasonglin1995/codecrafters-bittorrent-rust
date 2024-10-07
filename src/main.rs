use anyhow::Context;
use core::panic;
use clap::{Parser, Subcommand};
use bittorrent_starter_rust::torrent::{Keys, Torrent};
use serde_bencode;
use sha1::{Sha1, Digest};

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
        torrent: String
    },
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

// Usage: your_bittorrent.sh decode "<encoded_value>"
// Usage: sh ./your_bittorrent.sh info sample.torrent 
fn main() -> anyhow::Result<()> {
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
            let info_dict = &t.info;
            let info_dict_bytes = serde_bencode::to_bytes(info_dict).context("serialize info dict")?;
            let mut  hasher = Sha1::new();
            hasher.update(&info_dict_bytes);
            let hash_result = hasher.finalize();
            let hash_hex = format!("{:x}", hash_result);

            println!("Info Hash: {hash_hex}");
        }
    }

    Ok(())
}
