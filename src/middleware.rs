use crate::util::cry::{do_bytes_to_hex, do_hex, do_hmac_sha256};
use anyhow::Context;
use chrono::{NaiveDateTime, Utc};
use ntex::service::{Middleware, Service, ServiceCtx};
use ntex::web;
use ntex::web::HttpResponse;
use std::collections::HashMap;

pub struct CredentialsV4;

impl<S> Middleware<S> for CredentialsV4 {
    type Service = CredentialsV4Middleware<S>;

    fn create(&self, service: S) -> Self::Service {
        CredentialsV4Middleware { service }
    }
}

pub struct CredentialsV4Middleware<S> {
    service: S,
}

impl<S, Err> Service<web::WebRequest<Err>> for CredentialsV4Middleware<S>
where
    S: Service<web::WebRequest<Err>, Response = web::WebResponse, Error = web::Error>,
    Err: web::ErrorRenderer,
{
    type Response = web::WebResponse;
    type Error = web::Error;

    ntex::forward_poll_ready!(service);

    async fn call(
        &self,
        req: web::WebRequest<Err>,
        ctx: ServiceCtx<'_, Self>,
    ) -> Result<Self::Response, Self::Error> {
        // do filter here
        let access_key_id = "minioadmin";
        let secret_access_key = "minioadmin";
        let authorization = req.headers().get("Authorization");
        let mut flag = false;
        if authorization.is_some() {
            match valid_authorization_header(&req, access_key_id, secret_access_key) {
                Ok(b) => flag = b,
                Err(err) => {
                    println!("middleware error: {}", err);
                    flag = false
                }
            }
        } else {
            let qs = req.query_string();
            let credential = url::form_urlencoded::parse(qs.as_bytes())
                .find(|(key, _)| key == "X-Amz-Credential");
            if credential.is_some() {
                match valid_authorization_url(&req, access_key_id, secret_access_key) {
                    Ok(b) => flag = b,
                    Err(err) => {
                        println!("middleware error: {}", err);
                        flag = false
                    }
                }
            }
        }
        if !flag {
            return Ok(req.into_response(HttpResponse::Unauthorized().finish()));
        }

        // end do
        let res = ctx.call(&self.service, req).await?;
        Ok(res)
    }
}

fn valid_authorization_header(
    request: &web::WebRequest<impl web::ErrorRenderer>,
    access_key_id: &str,
    secret_access_key: &str,
) -> anyhow::Result<bool> {
    let authorization = request
        .headers()
        .get("Authorization")
        .context("Authorization不存在")?
        .to_str()?;
    let request_date = request
        .headers()
        .get("x-amz-date")
        .context("x-amz-date不存在")?
        .to_str()?;
    let content_hash = request
        .headers()
        .get("x-amz-content-sha256")
        .context("x-amz-content-sha256不存在")?
        .to_str()?;
    let http_method = request.method().to_string();
    let uri = request
        .uri()
        .path()
        .split('?')
        .next()
        .context("不存在")?
        .to_owned();
    let query_string = request.query_string().to_owned();

    // Splitting authorization into parts
    let parts: Vec<&str> = authorization.trim().split(',').collect();
    let credential = parts[0].split('=').nth(1).context("不存在")?;
    let credentials: Vec<&str> = credential.split('/').collect();
    let access_key = credentials[0];
    if access_key_id != access_key {
        return Ok(false);
    }
    let date = credentials[1];
    let region = credentials[2];
    let service = credentials[3];
    let aws4_request = credentials[4];

    let signed_header = parts[1].split('=').nth(1).context("不存在")?;
    let signed_headers: Vec<&str> = signed_header.split(';').collect();
    let signature = parts[2].split('=').nth(1).context("不存在")?;

    let string_to_sign = {
        let mut string_to_sign = String::new();
        string_to_sign.push_str("AWS4-HMAC-SHA256\n");
        string_to_sign.push_str(request_date);
        string_to_sign.push('\n');
        string_to_sign.push_str(&format!(
            "{}/{}/{}/{}\n",
            date, region, service, aws4_request
        ));

        let mut hashed_canonical_request = String::new();
        hashed_canonical_request.push_str(&format!("{}\n", http_method));
        hashed_canonical_request.push_str(&format!("{}\n", uri));

        if !query_string.is_empty() {
            let query_params = parse_query_params(&query_string);
            let mut query_string_builder = String::new();
            let mut query_params_vec: Vec<_> = query_params.iter().collect();
            query_params_vec.sort_by(|a, b| a.0.cmp(b.0));
            for (key, value) in query_params_vec {
                query_string_builder.push_str(&format!("{}={}&", key, value));
            }
            query_string_builder.pop(); // Remove the trailing '&'
            hashed_canonical_request.push_str(&format!("{}\n", query_string_builder));
        } else {
            hashed_canonical_request.push('\n');
        }

        let headers = request.headers();
        for name in signed_headers {
            if let Some(header_value) = headers.get(name) {
                hashed_canonical_request.push_str(&format!(
                    "{}:{}\n",
                    name,
                    header_value.to_str()?
                ));
            }
        }
        hashed_canonical_request.push('\n');
        hashed_canonical_request.push_str(&format!("{}\n", signed_header));
        hashed_canonical_request.push_str(content_hash);

        string_to_sign.push_str(&do_hex(&hashed_canonical_request));

        string_to_sign
    };

    let k_secret = format!("AWS4{}", secret_access_key);
    let k_secret = k_secret.as_bytes();
    let k_date = do_hmac_sha256(k_secret, date)?;
    let k_region = do_hmac_sha256(&k_date, region)?;
    let k_service = do_hmac_sha256(&k_region, service)?;
    let signature_key = do_hmac_sha256(&k_service, aws4_request)?;
    let auth_signature = do_hmac_sha256(&signature_key, &string_to_sign)?;
    let str_hex_signature = do_bytes_to_hex(&auth_signature);
    Ok(signature == str_hex_signature)
}

// TODO FIX: input has valid character
fn valid_authorization_url(
    request: &web::WebRequest<impl web::ErrorRenderer>,
    access_key_id: &str,
    secret_access_key: &str,
) -> anyhow::Result<bool> {
    let qs = request.query_string();
    let request_date = url::form_urlencoded::parse(qs.as_bytes())
        .find(|(key, _)| key == "X-Amz-Date")
        .map(|(_, value)| value.into_owned())
        .context("X-Amz-Date不存在")?;
    let content_hash = "UNSIGNED-PAYLOAD";
    let http_method = request.method().to_string();
    let uri = request
        .uri()
        .path()
        .split('?')
        .next()
        .context("不存在")?
        .to_owned();
    let query_string = request.query_string().to_owned();

    let credential = url::form_urlencoded::parse(qs.as_bytes())
        .find(|(key, _)| key == "X-Amz-Credential")
        .map(|(_, value)| value.into_owned())
        .context("X-Amz-Credential不存在")?;
    let credentials: Vec<&str> = credential.split('/').collect();
    let access_key = credentials[0];
    if access_key_id != access_key {
        return Ok(false);
    }
    let date = credentials[1];
    let region = credentials[2];
    let service = credentials[3];
    let aws4_request = credentials[4];

    // 第二部分-签名头中包含哪些字段
    let signed_header = url::form_urlencoded::parse(qs.as_bytes())
        .find(|(key, _)| key == "X-Amz-SignedHeaders")
        .map(|(_, value)| value.into_owned())
        .context("X-Amz-SignedHeaders不存在")?;
    let signed_headers: Vec<&str> = signed_header.split(';').collect();

    // 第三部分-生成的签名
    let signature = url::form_urlencoded::parse(qs.as_bytes())
        .find(|(key, _)| key == "X-Amz-Signature")
        .map(|(_, value)| value.into_owned())
        .context("X-Amz-Signature不存在")?;

    // 验证过期
    let expires = url::form_urlencoded::parse(qs.as_bytes())
        .find(|(key, _)| key == "X-Amz-Expires")
        .and_then(|(_, value)| value.parse::<i64>().ok())
        .context("X-Amz-Expires不存在")?;
    let fmt = "%Y%m%dT%H%M%SZ";
    let request_date_time = NaiveDateTime::parse_from_str(&request_date, fmt)
        .context("解析日期错误")?
        .and_utc();
    let end_date = request_date_time + chrono::Duration::seconds(expires);
    if end_date < Utc::now() {
        return Ok(false);
    }
    let string_to_sign = {
        let mut string_to_sign = String::new();
        string_to_sign.push_str("AWS4-HMAC-SHA256\n");
        string_to_sign.push_str(&request_date);
        string_to_sign.push('\n');
        string_to_sign.push_str(&format!(
            "{}/{}/{}/{}\n",
            date, region, service, aws4_request
        ));

        let mut hashed_canonical_request = String::new();
        hashed_canonical_request.push_str(&format!("{}\n", http_method));
        hashed_canonical_request.push_str(&format!("{}\n", uri));

        if !query_string.is_empty() {
            let query_params = parse_query_params(&query_string);
            let mut query_string_builder = String::new();
            let mut query_params_vec: Vec<_> = query_params.iter().collect();
            query_params_vec.sort_by(|a, b| a.0.cmp(b.0));
            for (key, value) in query_params_vec {
                query_string_builder.push_str(&format!("{}={}&", key, value));
            }
            query_string_builder.pop(); // Remove the trailing '&'
            hashed_canonical_request.push_str(&format!("{}\n", query_string_builder));
        } else {
            hashed_canonical_request.push('\n');
        }

        let headers = request.headers();
        for name in signed_headers {
            if let Some(header_value) = headers.get(name) {
                hashed_canonical_request.push_str(&format!(
                    "{}:{}\n",
                    name,
                    header_value.to_str()?
                ));
            }
        }
        hashed_canonical_request.push('\n');
        hashed_canonical_request.push_str(&format!("{}\n", signed_header));
        hashed_canonical_request.push_str(content_hash);

        string_to_sign.push_str(&do_hex(&hashed_canonical_request));

        string_to_sign
    };
    let k_secret = format!("AWS4{}", secret_access_key);
    let k_secret = k_secret.as_bytes();
    let k_date = do_hmac_sha256(k_secret, date)?;
    let k_region = do_hmac_sha256(&k_date, region)?;
    let k_service = do_hmac_sha256(&k_region, service)?;
    let signature_key = do_hmac_sha256(&k_service, aws4_request)?;
    let auth_signature = do_hmac_sha256(&signature_key, &string_to_sign)?;
    let str_hex_signature = do_bytes_to_hex(&auth_signature);
    Ok(signature == str_hex_signature)
}
fn parse_query_params(query_string: &str) -> HashMap<String, String> {
    let mut query_params = HashMap::new();
    for param in query_string.split('&') {
        let parts: Vec<&str> = param.splitn(2, '=').collect();
        if parts.len() == 2 {
            let key = parts[0].to_owned();
            let value = parts[1].to_owned();
            query_params.insert(key, value);
        }
    }
    query_params
}
