use serde_json;
use core::panic;
use std::env;

// Available if you need it!
// use serde_bencode

#[allow(dead_code)]
fn decode_bencoded_value(encoded_value: &str) -> (serde_json::Value, &str) {
    let first_char = encoded_value.chars().next().expect("Empty string received");

    match first_char {
        // decode integers eg. i52e
        'i' => {
            if let Some((n, rest)) = encoded_value.strip_prefix('i').unwrap().split_once('e') {
                 if let Ok(n) = n.parse::<i64>() {
                    return (n.into(), rest)
                 } else {
                    panic!("integer cannot be decoded")
                 }
            } else {
                panic!("integer cannot be decoded")
            }
        }

        // decode lists eg. l5:helloe = ["hello"], li25el3:fooi-43e5:helloe = [25,[ "foo", -43], "hello"]
        'l' => {
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

        // decode strings eg. 5:hello
        '0'..='9' => {
            if let Some((len, str)) = encoded_value.split_once(':') {
                if let Ok(len) = len.parse::<usize>() {
                    return (str[..len].into(), &str[len..]);
                } else {
                    panic!("string length cannot be parsed")
                }
            } else {
                panic!("string cannot be decoded")
            }
        }

        _ => {
            panic!("Unhandled encoded value: {}", encoded_value);
        }
    }

}

// Usage: your_bittorrent.sh decode "<encoded_value>"
fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];

    if command == "decode" {
        let encoded_value = &args[2];
        let decoded_value = decode_bencoded_value(encoded_value);
        println!("{}", decoded_value.0.to_string());
    } else {
        println!("unknown command: {}", args[1])
    }
}
