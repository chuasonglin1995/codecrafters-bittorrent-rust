use serde::{Deserialize, Serialize};
use sha1::{Sha1, Digest};

pub use hashes::Hashes;

/// A Metainfo files (also known as .torrent files)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Torrent {
    /// URL to a "tracker", which is a central server that keeps track of peers participating in the sharing of a torrent.
    pub announce: String,

    pub info: Info,
}

impl Torrent {
    pub fn info_hash(&self) -> [u8; 20] {
        let info_encoded =
            serde_bencode::to_bytes(&self.info).expect("re-encode info section should be fine");
        let mut hasher = Sha1::new();
        hasher.update(&info_encoded);
        hasher
            .finalize()
            .try_into()
            .expect("GenericArray<_, 20> == [_; 20]")
    }
    // pub fn info_hash(&self) -> [u8; 20] {
    //     let info_dict_bytes = serde_bencode::to_bytes(&self.info).expect("re-encode info section should be fine");
    //     let mut hasher = Sha1::new();
    //     hasher.update(&info_dict_bytes);
    //     let info_hash =  hasher.finalize();
    //     info_hash
    //         .try_into()
    //         .expect("GenericArray should be able to be converted")
    // }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Info {
    /// suggested name to save the file / directory as
    /// in a single file case, it will be the name of the file
    /// in a multi file case, it will be the name of a directory
    pub name: String,

    /// number of bytes in each piece
    /// For the purposes of transfer, files are split into fixed-size pieces which are all the same
    /// length except for possibly the last one which may be truncated, piece length is almost
    /// always a power of two, most commonly 2^18 = 256K (BitTorrent prior to version 3.2 uses 2^20 = 1M
    /// as default). 
    #[serde(rename = "piece length")]
    pub plength: u32,

    /// pieces maps to a string whose length is a multiple of 20. 
    /// concatenated SHA-1 hashes of each piece
    pub pieces: Hashes,

    #[serde(flatten)]
    pub keys: Keys,
}

/// There is a key length or a key files but not both or neither. 
/// If length is present then download represents a single file,
/// otherwise, its a set of files which go in a directory structure
/// *Serde untagged comes with abit of performance loss
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Keys {
    SingleFile {
        length: u32,
    },
    MultiFile { 
        files: Vec<File> 
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct File {
    // the length of the file, in bytes
    pub length: u32,

    // Subdirectory names for this file
    pub path:Vec<String>,
}

pub mod hashes {
    use serde::de::{self, Deserialize, Deserializer, Visitor};
	use serde::ser::{Serialize, Serializer};
    use std::fmt;

	#[derive(Debug, Clone)]
	pub struct Hashes(pub Vec<[u8; 20]>);
	struct HashesVisitor;

	impl<'de> Visitor<'de> for HashesVisitor {
		type Value = Hashes;

		fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
			formatter.write_str("a byte string whose length is a multiple of 20")
		}

		fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
		where
			E: de::Error,
		{
			if v.len() % 20 != 0 {
				return Err(E::custom(format!("length is {}", v.len())));
			}
			Ok(Hashes(
				v.chunks_exact(20)
					.map(|slice_20| slice_20.try_into().expect("guaranteed to be length 20"))
					.collect()
			))
		}
	}

	impl<'de> Deserialize<'de> for Hashes {
		fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
		where
			D: Deserializer<'de>,
		{
			deserializer.deserialize_bytes(HashesVisitor)
		}
	}

    // note: this hash is actually not complete, because we also need to sort the keys in alphabetical order
	impl Serialize for Hashes {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let single_slice = self.0.concat(); // concat is like flatten
            serializer.serialize_bytes(&single_slice)
        }
    }
}