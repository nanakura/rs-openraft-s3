use aes::Aes256;
use block_modes::block_padding::Pkcs7;
use block_modes::{BlockMode, Cbc};
use crypto_hash::{hex_digest, Algorithm};
use hmac::{Hmac, Mac};
use ntex::util::BytesMut;
use rand::seq::IndexedRandom;
use sha2::Sha256;
use zstd::zstd_safe::WriteBuf;

// 定义一个默认的密钥常量。
const DEFAULT_KEY: &str = "000102030405060708090A0B0C0D0E0F";

// 使用 MD5 算法对字符串进行哈希加密的函数。
pub fn encrypt_by_md5(s: &str) -> String {
    let digest = hex_digest(Algorithm::MD5, s.as_bytes());
    digest
}

// 定义一个包含所有可打印 ASCII 字符的字符串常量。
const BASE_STR: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
// 定义 AesCbc 类型为 Aes256 加密算法和 Pkcs7 填充方式的组合。
type AesCbc = Cbc<Aes256, Pkcs7>;

// 生成指定长度的随机 ASCII 字符串的函数。
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

// 使用 AES-256-CBC 加密算法加密数据的函数。
pub fn aes_256_cbc_encrypt(data: &[u8]) -> anyhow::Result<Vec<u8>> {
    let iv_str = gen_ascii_chars(16);
    let iv = iv_str.as_bytes();
    let cipher = AesCbc::new_from_slices(DEFAULT_KEY.as_bytes(), iv)?;
    let ciphertext = cipher.encrypt_vec(data);
    let mut buffer = BytesMut::from(iv);
    buffer.extend_from_slice(&ciphertext);
    Ok(Vec::from(buffer.as_slice()))
}

// 使用 AES-256-CBC 解密算法解密数据的函数。
pub fn aes_256_cbc_decrypt(data: &[u8]) -> anyhow::Result<Vec<u8>> {
    //let bytes = general_purpose::STANDARD.decode(data)?;
    let cipher = AesCbc::new_from_slices(DEFAULT_KEY.as_bytes(), &data[0..16])?;
    Ok(cipher.decrypt_vec(&data[16..])?)
}

// 定义 HmacSha256 类型为使用 SHA256 哈希函数的 HMAC。
type HmacSha256 = Hmac<Sha256>;
// 对数据进行 SHA256 哈希的函数，返回十六进制字符串。
pub fn do_hex(data: &str) -> String {
    hex_digest(Algorithm::SHA256, data.as_bytes())
}

// 使用 HMAC-SHA256 算法对数据进行签名的函数。
pub fn do_hmac_sha256(key: &[u8], data: &str) -> anyhow::Result<Vec<u8>> {
    let mut mac = HmacSha256::new_from_slice(key)?;
    mac.update(data.as_bytes());
    let res = mac.finalize();
    let bytes = res.into_bytes();
    let x = &bytes[..];
    //Ok(hex::encode(x))
    Ok(Vec::from(x))
}

// 将字节向量转换为十六进制字符串的函数。
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
