pub fn url_encode(input: &[u8; 20]) -> String {
  let mut output = String::with_capacity(input.len() * 3);

  for b in input {
      output.push('%');
      output.push_str(&format!("{b:02x}"));
  }

  output
}