pub struct AddressBar {
    pub text: String,
    pub is_focused: bool,
}

impl AddressBar {
    pub fn new(initial_url: &str) -> Self {
        Self {
            text: initial_url.to_string(),
            is_focused: false,
        }
    }

    /// Called when user hits Enter
    /// Figures out if input is a URL or a search query
    pub fn resolve(&self) -> String {
        let input = self.text.trim();

        // Already a full URL
        if input.starts_with("http://") || input.starts_with("https://") {
            return input.to_string();
        }

        // Looks like a domain (has dot, no spaces)
        if input.contains('.') && !input.contains(' ') {
            return format!("https://{}", input);
        }

        // Treat as search query
        format!(
            "https://duckduckgo.com/?q={}",
            input.replace(' ', "+")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve() {
        let bar = AddressBar::new("");

        // Full URL passthrough
        let mut b = AddressBar::new("https://google.com");
        assert_eq!(b.resolve(), "https://google.com");

        // Domain gets https prepended
        b.text = "github.com".to_string();
        assert_eq!(b.resolve(), "https://github.com");

        // Search query
        b.text = "how to make a browser".to_string();
        assert!(b.resolve().contains("duckduckgo.com"));
        assert!(b.resolve().contains("how+to+make+a+browser"));
    }
}