use serde_json;
use core::panic;
use std::env;

// Available if you need it!
// use serde_bencode

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
