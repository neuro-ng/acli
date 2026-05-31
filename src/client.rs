use crate::config::Profile;
use std::io::Read;

pub struct Client {
    pub base_url: String,
    pub email: String,
    pub api_token: String,
}

impl Client {
    pub fn new(profile: Profile) -> Self {
        Client {
            base_url: profile.atlassian_url.trim_end_matches('/').to_string(),
            email: profile.email,
            api_token: profile.api_token,
        }
    }

    pub fn get_cloud_id(&self) -> Result<String, String> {
        let resp = self.request("GET", "/_edge/tenant_info", None, None)?;
        let v: serde_json::Value = serde_json::from_str(&resp)
            .map_err(|e| format!("Failed to parse tenant info JSON: {}", e))?;
        v["cloudId"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "cloudId not found in tenant_info response".to_string())
    }

    pub fn request(
        &self,
        method: &str,
        path: &str,
        query: Option<&[(&str, &str)]>,
        body: Option<serde_json::Value>,
    ) -> Result<String, String> {
        let mut url = if path.starts_with("http://") || path.starts_with("https://") {
            path.to_string()
        } else {
            format!("{}{}", self.base_url, path)
        };

        if let Some(params) = query {
            if !params.is_empty() {
                let query_str: String = params
                    .iter()
                    .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
                    .collect::<Vec<String>>()
                    .join("&");
                url = format!("{}?{}", url, query_str);
            }
        }

        let mut req = ureq::request(method, &url);

        if !self.email.is_empty() {
            let encoded = base64_encode(&format!("{}:{}", self.email, self.api_token));
            req = req.set("Authorization", &format!("Basic {}", encoded));
        } else if !self.api_token.is_empty() {
            req = req.set("Authorization", &format!("Bearer {}", self.api_token));
        }

        req = req.set("Accept", "application/json");

        let response = if let Some(b) = body {
            req = req.set("Content-Type", "application/json");
            req.send_string(&b.to_string())
        } else {
            req.call()
        };

        match response {
            Ok(resp) => {
                let status: u16 = resp.status();
                let mut body_str = String::new();
                resp.into_reader()
                    .read_to_string(&mut body_str)
                    .map_err(|e| format!("Failed to read response body: {}", e))?;
                if (200..300).contains(&status) {
                    Ok(body_str)
                } else {
                    Err(format!("API error (HTTP {}): {}", status, body_str))
                }
            }
            Err(ureq::Error::Status(code, resp)) => {
                let mut body_str = String::new();
                let _ = resp.into_reader().read_to_string(&mut body_str);
                Err(format!("API error (HTTP {}): {}", code, body_str))
            }
            Err(e) => Err(format!("Network/HTTP request failed: {}", e)),
        }
    }

    /// Multipart file upload using a manually constructed boundary.
    /// Required by Jira's attachment API (`X-Atlassian-Token: no-check`).
    pub fn request_multipart(
        &self,
        method: &str,
        path: &str,
        file_path: &str,
    ) -> Result<String, String> {
        let file_name = std::path::Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file");

        let mut content = Vec::new();
        let mut file = std::fs::File::open(file_path)
            .map_err(|e| format!("Failed to open '{}': {}", file_path, e))?;
        file.read_to_end(&mut content)
            .map_err(|e| format!("Failed to read '{}': {}", file_path, e))?;

        let boundary = "acli-boundary-a1b2c3d4";
        let mut body: Vec<u8> = Vec::new();
        body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        body.extend_from_slice(
            format!(
                "Content-Disposition: form-data; name=\"file\"; filename=\"{}\"\r\n",
                file_name
            )
            .as_bytes(),
        );
        body.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
        body.extend_from_slice(&content);
        body.extend_from_slice(format!("\r\n--{}--\r\n", boundary).as_bytes());

        let url = format!("{}{}", self.base_url, path);
        let mut req = ureq::request(method, &url);

        if !self.email.is_empty() {
            let encoded = base64_encode(&format!("{}:{}", self.email, self.api_token));
            req = req.set("Authorization", &format!("Basic {}", encoded));
        } else if !self.api_token.is_empty() {
            req = req.set("Authorization", &format!("Bearer {}", self.api_token));
        }

        req = req
            .set("X-Atlassian-Token", "no-check")
            .set("Accept", "application/json")
            .set(
                "Content-Type",
                &format!("multipart/form-data; boundary={}", boundary),
            );

        match req.send_bytes(&body) {
            Ok(resp) => {
                let status = resp.status();
                let mut body_str = String::new();
                resp.into_reader()
                    .read_to_string(&mut body_str)
                    .map_err(|e| format!("Failed to read response body: {}", e))?;
                if (200..300).contains(&status) {
                    Ok(body_str)
                } else {
                    Err(format!("API error (HTTP {}): {}", status, body_str))
                }
            }
            Err(ureq::Error::Status(code, resp)) => {
                let mut body_str = String::new();
                let _ = resp.into_reader().read_to_string(&mut body_str);
                Err(format!("API error (HTTP {}): {}", code, body_str))
            }
            Err(e) => Err(format!("Network/HTTP request failed: {}", e)),
        }
    }

    pub fn request_jsm(
        &self,
        method: &str,
        path: &str,
        query: Option<&[(&str, &str)]>,
        body: Option<serde_json::Value>,
    ) -> Result<String, String> {
        let cloud_id = self.get_cloud_id()?;
        let host = if self.base_url.contains("127.0.0.1") || self.base_url.contains("localhost") {
            self.base_url.clone()
        } else {
            "https://api.atlassian.com".to_string()
        };
        let full_path = format!("{}/jsm/ops/api/{}/v1{}", host, cloud_id, path);
        self.request(method, &full_path, query, body)
    }
}

pub fn base64_encode(input: &str) -> String {
    const CHARSET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = input.as_bytes();
    let mut result = String::new();
    let mut i = 0;
    while i < bytes.len() {
        let b0 = bytes[i];
        let b1 = if i + 1 < bytes.len() {
            Some(bytes[i + 1])
        } else {
            None
        };
        let b2 = if i + 2 < bytes.len() {
            Some(bytes[i + 2])
        } else {
            None
        };

        let c0 = b0 >> 2;
        let c1 = ((b0 & 0x03) << 4) | (b1.unwrap_or(0) >> 4);
        let c2 = b1.map(|b| ((b & 0x0f) << 2) | (b2.unwrap_or(0) >> 6));
        let c3 = b2.map(|b| b & 0x3f);

        result.push(CHARSET[c0 as usize] as char);
        result.push(CHARSET[c1 as usize] as char);
        result.push(c2.map_or('=', |c| CHARSET[c as usize] as char));
        result.push(c3.map_or('=', |c| CHARSET[c as usize] as char));
        i += 3;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_encode_three_bytes() {
        assert_eq!(base64_encode("Man"), "TWFu");
    }

    #[test]
    fn test_base64_encode_two_bytes() {
        assert_eq!(base64_encode("Ma"), "TWE=");
    }

    #[test]
    fn test_base64_encode_one_byte() {
        assert_eq!(base64_encode("M"), "TQ==");
    }

    #[test]
    fn test_base64_encode_empty() {
        assert_eq!(base64_encode(""), "");
    }

    #[test]
    fn test_base64_encode_credentials() {
        let encoded = base64_encode("user@example.com:token123");
        assert!(!encoded.is_empty());
        assert!(encoded
            .chars()
            .all(|c| c.is_alphanumeric() || c == '+' || c == '/' || c == '='));
    }
}
