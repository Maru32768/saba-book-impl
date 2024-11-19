use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::clone::Clone;

const SUPPORTED_PROTOCOLS: [&'static str; 1] = ["http"];

#[derive(Debug, Clone, PartialEq)]
pub struct Url {
    url: String,
    host: String,
    port: String,
    path: String,
    searchpart: String,
}

impl Url {
    pub fn new(url: String) -> Result<Self, String> {
        if !is_supported_protocol(&url) {
            return Err("Invalid scheme.".to_string());
        }

        Ok(
            Self {
                url: url.clone(),
                host: extract_host(&url),
                port: extract_port(&url),
                path: extract_path(&url),
                searchpart: extract_searchpart(&url),
            }
        )
    }
}

fn is_supported_protocol(url: &str) -> bool {
    for protocol in SUPPORTED_PROTOCOLS {
        let a = format!("{}://", protocol);
        if url.starts_with(&a) {
            return true;
        }
    }

    false
}

fn extract_host(url: &str) -> String {
    let scheme_removed = remove_scheme(url);
    if let Some(path_start) = scheme_removed.find("/") {
        let host = &scheme_removed[..path_start];
        host.split(":").next().unwrap_or("").to_string()
    } else {
        scheme_removed.split(":").next().unwrap_or("").to_string()
    }
}

fn extract_port(url: &str) -> String {
    let scheme_removed = remove_scheme(url);
    if let Some(index) = scheme_removed.find(":") {
        let a: Vec<&str> = scheme_removed.splitn(2, "/").collect();
        a[0][index + 1..].to_string()
    } else {
        "80".to_string()
    }
}

fn extract_path(url: &str) -> String {
    let scheme_removed = remove_scheme(url);
    let url_parts: Vec<&str> = scheme_removed.splitn(2, "/").collect();

    if url_parts.len() < 2 {
        return "".to_string();
    }

    let path_and_searchpart: Vec<&str> = url_parts[1].splitn(2, "?").collect();
    path_and_searchpart[0].to_string()
}

fn extract_searchpart(url: &str) -> String {
    let a: Vec<&str> = url.splitn(2, "?").collect();
    if a.len() < 2 {
        "".to_string()
    } else {
        a[1].to_string()
    }
}

fn remove_scheme(url: &str) -> String {
    if let Some(scheme_end) = url.find("://") {
        return url[scheme_end + 3..].to_string();
    }

    url.to_string()
}

#[cfg(test)]
mod tests {
    use crate::url::Url;
    use alloc::string::ToString;

    #[test]
    fn test_url_host() {
        let url = "http://example.com".to_string();
        let expected = Url {
            url: url.clone(),
            host: "example.com".to_string(),
            port: "80".to_string(),
            path: "".to_string(),
            searchpart: "".to_string(),
        };
        assert_eq!(expected, Url::new(url).unwrap())
    }

    #[test]
    fn test_url_port() {
        let url = "http://example.com:8888".to_string();
        let expected = Url {
            url: url.clone(),
            host: "example.com".to_string(),
            port: "8888".to_string(),
            path: "".to_string(),
            searchpart: "".to_string(),
        };
        assert_eq!(expected, Url::new(url).unwrap())
    }

    #[test]
    fn test_url_host_port_path() {
        let url = "http://example.com:8888/index.html".to_string();
        let expected = Url {
            url: url.clone(),
            host: "example.com".to_string(),
            port: "8888".to_string(),
            path: "index.html".to_string(),
            searchpart: "".to_string(),
        };
        assert_eq!(expected, Url::new(url).unwrap())
    }

    #[test]
    fn test_url_host_port_path_seachquery() {
        let url = "http://example.com:8888/index.html?a=123&b=456".to_string();
        let expected = Url {
            url: url.clone(),
            host: "example.com".to_string(),
            port: "8888".to_string(),
            path: "index.html".to_string(),
            searchpart: "a=123&b=456".to_string(),
        };
        assert_eq!(expected, Url::new(url).unwrap())
    }

    #[test]
    fn test_no_scheme() {
        let url = "example.com".to_string();
        let expected = Err("Invalid scheme.".to_string());
        assert_eq!(expected, Url::new(url))
    }

    #[test]
    fn test_unsupported_scheme() {
        let url = "https://example.com/".to_string();
        let expected = Err("Invalid scheme.".to_string());
        assert_eq!(expected, Url::new(url))
    }
}
