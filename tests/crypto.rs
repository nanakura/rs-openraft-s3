#[cfg(test)]
mod test {
    use rs_s3_local::util::cry::{aes_256_cbc_decrypt, aes_256_cbc_encrypt, do_hmac_sha256};
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
