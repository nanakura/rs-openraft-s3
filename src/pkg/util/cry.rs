use crypto_hash::{hex_digest, Algorithm};
use hmac::{Hmac, Mac};
use sha2::Sha256;

// const DEFAULT_KEY: [u8; 8] = [76, 111, 99, 97, 108, 83, 51, 88];
#[allow(dead_code)]
static DEFAULT_KEY: &str = "LocalS3X";

pub fn encrypt_by_md5(s: &str) -> String {
    let digest = hex_digest(Algorithm::MD5, s.as_bytes());
    digest
}

#[allow(dead_code)]
pub fn encrypt_by_des(data: &str) -> anyhow::Result<String> {
    Ok(data.to_string())
}

#[allow(dead_code)]
pub fn decrypt_by_des(data: &str) -> anyhow::Result<String> {
    Ok(data.to_string())
}

type HmacSha256 = Hmac<Sha256>;
pub fn do_hex(data: &str) -> String {
    hex_digest(Algorithm::SHA256, data.as_bytes())
}

pub fn do_hmac_sha256(key: &[u8], data: &str) -> anyhow::Result<Vec<u8>> {
    let mut mac = HmacSha256::new_from_slice(key)?;
    mac.update(data.as_bytes());
    let res = mac.finalize();
    let bytes = res.into_bytes();
    let x = &bytes[..];
    //Ok(hex::encode(x))
    Ok(Vec::from(x))
}

pub fn do_bytes_to_hex(bytes: &[u8]) -> String {
    let hex_array: [char; 16] = [
        '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F',
    ];
    let mut hex_chars = Vec::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        let v = byte as usize;
        hex_chars.push(hex_array[v >> 4]);
        hex_chars.push(hex_array[v & 0x0F]);
    }
    hex_chars.into_iter().collect::<String>().to_lowercase()
}

// fn hex_to_string(hex: &str) -> anyhow::Result<String> {
//     let bytes = hex::decode(hex)?;
//     let string = String::from_utf8(bytes)?;
//     Ok(string)
// }

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test1() {
        let code = do_hmac_sha256(b"my secret and secure key", "input message").unwrap();
        assert_eq!(
            hex::encode(code),
            "97d2a569059bbcd8ead4444ff99071f4c01d005bcefe0d3567e1be628e5fdcd9"
        );
    }
}
