#[derive(Debug, PartialEq, Eq)]
enum ParseError {
    TooShort,
    BadMagic,
    BadVersion,
    LengthMismatch,
}

fn parse_packet(data: &[u8]) -> Result<&[u8], ParseError> {
    if data.len() < 4 {
        return Err(ParseError::TooShort);
    }
    if data[0] != b'C' || data[1] != b'S' {
        return Err(ParseError::BadMagic);
    }
    if data[2] > 3 {
        return Err(ParseError::BadVersion);
    }
    let len = data[3] as usize;
    if len + 4 != data.len() {
        return Err(ParseError::LengthMismatch);
    }
    Ok(&data[4..])
}

fn main() {
    let good = b"CS\x01\x03abc";
    let bad = b"CS\x09\x01z";

    println!("good={:?}", parse_packet(good));
    println!("bad={:?}", parse_packet(bad));
}
