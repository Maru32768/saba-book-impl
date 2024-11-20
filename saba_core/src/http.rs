use crate::error::Error;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub version: String,
    pub status_code: u32,
    pub reason: String,
    pub headers: Vec<Header>,
    pub body: String,
}

impl HttpResponse {
    pub fn new(raw_response: String) -> Result<Self, Error> {
        let preprocessed_response = raw_response.trim_start().replace("\n\r", "\n");

        let (status_line, remaining) = match preprocessed_response.split_once("\n") {
            Some((s, r)) => (s, r),
            None => { return Err(Error::Network(format!("Invalid HTTP response: {}", preprocessed_response))) }
        };

        let (headers, body) = match remaining.split_once("\n\n") {
            Some((h, b)) => {
                let mut headers = Vec::new();
                for header in h.split("\n") {
                    let splitted: Vec<&str> = header.splitn(2, ":").collect();
                    headers.push(Header::new(
                        splitted[0].trim().to_string(),
                        splitted[1].trim().to_string(),
                    ))
                }
                (headers, b)
            }
            None => (Vec::new(), remaining),
        };
        let statuses: Vec<&str> = status_line.split(" ").collect();

        Ok(Self {
            version: statuses[0].to_string(),
            status_code: statuses[1].parse().unwrap_or(404),
            reason: statuses[2].to_string(),
            headers,
            body: body.to_string(),
        })
    }

    pub fn header_value(&self, name: &str) -> Result<String, String> {
        for h in &self.headers {
            if h.name == name {
                return Ok(h.value.clone());
            }
        }

        Err(format!("Failed to find {} in headers", name))
    }
}

#[derive(Debug, Clone)]
pub struct Header {
    name: String,
    value: String,
}

impl Header {
    pub fn new(name: String, value: String) -> Self {
        Self { name, value }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_status_line_only() {
        let raw = "HTTP/1.1 200 OK\n\n".to_string();
        let res = HttpResponse::new(raw).expect("Failed to parse HTTP response");
        assert_eq!("HTTP/1.1", res.version);
        assert_eq!(200, res.status_code);
        assert_eq!("OK", res.reason)
    }

    #[test]
    fn test_one_header() {
        let raw = "HTTP/1.1 200 OK\nDate:xx xx xx\n\n".to_string();
        let res = HttpResponse::new(raw).expect("Failed to parse HTTP response");

        assert_eq!("HTTP/1.1", res.version);
        assert_eq!(200, res.status_code);
        assert_eq!("OK", res.reason);

        assert_eq!(Ok("xx xx xx".to_string()), res.header_value("Date"));
    }

    #[test]
    fn test_two_headers_with_white_space() {
        let raw = "HTTP/1.1 200 OK\nDate: xx xx xx\nContent-Length: 42\n\n".to_string();
        let res = HttpResponse::new(raw).expect("Failed to parse HTTP response");

        assert_eq!("HTTP/1.1", res.version);
        assert_eq!(200, res.status_code);
        assert_eq!("OK", res.reason);

        assert_eq!(Ok("xx xx xx".to_string()), res.header_value("Date"));
        assert_eq!(Ok("42".to_string()), res.header_value("Content-Length"));
    }

    #[test]
    fn test_body() {
        let raw = "HTTP/1.1 200 OK\nDate: xx xx xx\n\nbody message".to_string();
        let res = HttpResponse::new(raw).expect("Failed to parse HTTP response");

        assert_eq!("HTTP/1.1", res.version);
        assert_eq!(200, res.status_code);
        assert_eq!("OK", res.reason);

        assert_eq!(Ok("xx xx xx".to_string()), res.header_value("Date"));

        assert_eq!("body message".to_string(), res.body);
    }

    #[test]
    fn test_invalid() {
        let raw = "HTTP/1.1 200 OK".to_string();
        assert!(HttpResponse::new(raw).is_err())
    }
}
