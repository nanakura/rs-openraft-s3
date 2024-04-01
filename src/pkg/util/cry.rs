use aes::Aes256;
use block_modes::block_padding::Pkcs7;
use block_modes::{BlockMode, Cbc};
use crypto_hash::{hex_digest, Algorithm};
use hmac::{Hmac, Mac};
use ntex::util::BytesMut;
use rand::seq::IndexedRandom;
use sha2::Sha256;
use zstd::zstd_safe::WriteBuf;

const DEFAULT_KEY: &str = "000102030405060708090A0B0C0D0E0F";

pub fn encrypt_by_md5(s: &str) -> String {
    let digest = hex_digest(Algorithm::MD5, s.as_bytes());
    digest
}

const BASE_STR: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
type AesCbc = Cbc<Aes256, Pkcs7>;
fn gen_ascii_chars(size: usize) -> String {
    let mut rng = &mut rand::thread_rng();
    String::from_utf8(
        BASE_STR
            .as_bytes()
            .choose_multiple(&mut rng, size)
            .cloned()
            .collect(),
    )
    .unwrap()
}
pub fn aes_256_cbc_encrypt(data: &[u8]) -> anyhow::Result<Vec<u8>> {
    let iv_str = gen_ascii_chars(16);
    let iv = iv_str.as_bytes();
    let cipher = AesCbc::new_from_slices(DEFAULT_KEY.as_bytes(), iv)?;
    let ciphertext = cipher.encrypt_vec(data);
    let mut buffer = BytesMut::from(iv);
    buffer.extend_from_slice(&ciphertext);
    Ok(Vec::from(buffer.as_slice()))
}

pub fn aes_256_cbc_decrypt(data: &[u8]) -> anyhow::Result<Vec<u8>> {
    //let bytes = general_purpose::STANDARD.decode(data)?;
    let cipher = AesCbc::new_from_slices(DEFAULT_KEY.as_bytes(), &data[0..16])?;
    Ok(cipher.decrypt_vec(&data[16..])?)
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

    #[test]
    fn test2() {
        let s = "xxxxxx";
        let en = aes_256_cbc_encrypt(s.as_bytes()).unwrap();
        //let en = general_purpose::STANDARD.encode(&en);
        let de = String::from_utf8(aes_256_cbc_decrypt(&en).unwrap()).unwrap();
        assert_eq!(s, &de);
    }
}
