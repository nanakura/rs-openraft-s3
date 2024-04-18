#[cfg(test)]
mod test {
    use rkyv::{Deserialize, Infallible};
    use rs_s3_local::fs::Metadata;

    #[test]
    fn test1() {
        let m = Metadata {
            name: "xxx".to_string(),
            size: 10,
            file_type: "xxxxx".to_string(),
            time: Default::default(),
            chunks: vec![],
        };

        let bytes = rkyv::to_bytes::<_, 256>(&m).unwrap();
        let bytes = bytes.as_slice();
        let archived = rkyv::check_archived_root::<Metadata>(&bytes[..]).unwrap();
        let res: Metadata = archived.deserialize(&mut Infallible).unwrap();
        assert_eq!(m, res)
    }

    #[test]
    fn test2() {}
}
