use reqwest::Url;

pub fn is_valid_proxy_art_url(url: &str) -> bool {
    if url.is_empty() || url.len() > 2048 {
        return false;
    }

    let Ok(url) = Url::parse(url) else {
        return false;
    };

    if url.scheme() != "https" {
        return false;
    }
    if !url.username().is_empty() || url.password().is_some() {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::is_valid_proxy_art_url;

    #[test]
    fn accepts_https_image_url() {
        assert!(is_valid_proxy_art_url("https://example.com/a/b.png"));
    }

    #[test]
    fn rejects_http_credentials_and_overlong() {
        assert!(!is_valid_proxy_art_url("http://example.com/a.png"));
        assert!(!is_valid_proxy_art_url(
            "https://user:pass@example.com/a.png"
        ));
        assert!(!is_valid_proxy_art_url(&format!(
            "https://example.com/{}",
            "x".repeat(2100)
        )));
        assert!(!is_valid_proxy_art_url(""));
    }
}
