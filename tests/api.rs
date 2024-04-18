
#[cfg(test)]
mod test {
    use quick_xml::se::to_string;
    use serde::{Deserialize, Serialize};
    use rs_s3_local::model::{Bucket, BucketWrapper, ListBucketResp, Owner};

    #[test]
    fn test1() {
        let mut buckets = Vec::new();
        buckets.push(Bucket {
            name: "xx".to_string(),
            creation_date: "111".to_string(),
        });
        let list_res = ListBucketResp {
            id: "20230529".to_string(),
            owner: Owner {
                display_name: "minioadmin".to_string(),
            },
            buckets: BucketWrapper { bucket: buckets },
        };
        let xml = to_string(&list_res);
        assert!(xml.is_ok(), "序列化错误");
    }

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Person {
        #[serde(rename = "FullName")]
        full_name: String,
        age: u32,
        #[serde(rename = "Address")]
        addresses: Vec<Address>,
    }

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Address {
        street: String,
        city: String,
        state: String,
    }

    #[test]
    fn test2() {
        let person = Person {
            full_name: "John Doe".to_string(),
            age: 30,
            addresses: vec![
                Address {
                    street: "123 Main St".to_string(),
                    city: "New York".to_string(),
                    state: "NY".to_string(),
                },
                Address {
                    street: "456 Elm St".to_string(),
                    city: "Los Angeles".to_string(),
                    state: "CA".to_string(),
                },
            ],
        };
        // Serialize to XML
        let xml = to_string(&person);
        assert!(xml.is_ok(), "序列化失败");
    }
}
