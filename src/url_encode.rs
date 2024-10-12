pub fn url_encode(t: &[u8; 20]) -> String {
  let mut encoded = String::with_capacity(3 * t.len());
  for &byte in t {
      encoded.push('%');
      encoded.push_str(&hex::encode(&[byte]));
  }
  encoded
}