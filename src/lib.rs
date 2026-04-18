use regex::Regex;

/// Extracts language specification from text comments or plain text.
///
/// **Note:** Language comments are optional! LanguageTool can auto-detect the language.
/// Use language comments only when you want to override auto-detection.
///
/// Supports multiple formats:
/// - HTML comments for Markdown: <!-- lang: xx_YY -->
/// - Shell-style: # lang: xx_YY
/// - C-style: // lang: xx_YY
/// - INI-style: ; lang: xx_YY
/// - LaTeX/TeX-style: % lang: xx_YY
/// - Plain text anywhere: lang: xx_YY
///
/// Format: "lang: xx_YY" where xx is language code and YY is country code
///
/// If multiple lang: specifications exist, the first one in the document is used.
/// If no language is specified, LanguageTool will auto-detect the language.
///
/// # Examples
///
/// ```
/// use languagetool_lsp::extract_lang;
///
/// assert_eq!(extract_lang("<!-- lang: en_US -->"), Some("en-US".to_string()));
/// assert_eq!(extract_lang("# lang: de_DE"), Some("de-DE".to_string()));
/// assert_eq!(extract_lang("// lang: de_DE"), Some("de-DE".to_string()));
/// assert_eq!(extract_lang("; lang: fr_FR"), Some("fr-FR".to_string()));
/// assert_eq!(extract_lang("% lang: es_ES"), Some("es-ES".to_string()));
/// assert_eq!(extract_lang("Some text lang: en_US here"), Some("en-US".to_string()));
/// assert_eq!(extract_lang("No lang here"), None);
/// ```
pub fn extract_lang(text: &str) -> Option<String> {
    // Combined regex that matches all comment styles and plain text
    // This ensures we find the FIRST lang: specification regardless of format
    let re = Regex::new(r"(?mi)(<!--\s*lang:\s*([A-Za-z_]+)\s*-->|^\s*(#|//|;|%)\s*lang:\s*([A-Za-z_]+)|lang:\s*([A-Za-z_]+))").unwrap();

    if let Some(caps) = re.captures(text) {
        // The regex has multiple groups: HTML comments (group 2), other comments (group 4), plain text (group 5)
        if let Some(html_lang) = caps.get(2) {
            return Some(normalize_lang_code(html_lang.as_str()));
        } else if let Some(other_lang) = caps.get(4) {
            return Some(normalize_lang_code(other_lang.as_str()));
        } else if let Some(plain_lang) = caps.get(5) {
            return Some(normalize_lang_code(plain_lang.as_str()));
        }
    }

    None
}

/// Normalize language code to LanguageTool format (xx-YY instead of xx_YY)
fn normalize_lang_code(lang: &str) -> String {
    lang.replace('_', "-")
}

/// Extract language specification and return both the language code and cleaned text
///
/// This removes language specification comments from the text before sending to LanguageTool
/// to avoid offset mismatches.
///
/// Returns (language_code, cleaned_text)
pub fn extract_lang_and_clean(text: &str) -> (String, String) {
    let lang = extract_lang(text).unwrap_or_else(|| "auto".to_string());

    // Remove language specification comments
    let re = Regex::new(r"(?mi)(<!--\s*lang:\s*[A-Za-z_]+\s*-->|^\s*(#|//|;|%)\s*lang:\s*[A-Za-z_]+|lang:\s*[A-Za-z_]+)").unwrap();
    let cleaned_text = re.replace_all(text, "").to_string();

    (lang, cleaned_text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_lang_and_clean_html_comment() {
        let text = "<!-- lang: de_DE -->\n\nIch möchte nicht dksadf";
        let (lang, cleaned) = extract_lang_and_clean(text);
        assert_eq!(lang, "de-DE".to_string());
        assert_eq!(cleaned, "\n\nIch möchte nicht dksadf");
    }

    #[test]
    fn test_extract_lang_and_clean_no_comment() {
        let text = "Ich möchte nicht dksadf";
        let (lang, cleaned) = extract_lang_and_clean(text);
        assert_eq!(lang, "auto".to_string());
        assert_eq!(cleaned, "Ich möchte nicht dksadf");
    }

    #[test]
    fn test_extract_lang_with_html_comment() {

    #[test]
    fn test_extract_lang_with_html_comment() {
        let text = "<!-- lang: en_US -->\nSome content here";
        assert_eq!(extract_lang(text), Some("en-US".to_string()));
    }

    #[test]
    fn test_extract_lang_with_hash_comment() {
        let text = "# lang: en_US\nSome content here";
        assert_eq!(extract_lang(text), Some("en-US".to_string()));
    }

    #[test]
    fn test_extract_lang_with_double_slash_comment() {
        let text = "// lang: de_DE\nSome content here";
        assert_eq!(extract_lang(text), Some("de-DE".to_string()));
    }

    #[test]
    fn test_extract_lang_with_semicolon_comment() {
        let text = "; lang: fr_FR\nSome content here";
        assert_eq!(extract_lang(text), Some("fr-FR".to_string()));
    }

    #[test]
    fn test_extract_lang_no_lang_specified() {
        let text = "# This is just a comment\nSome content";
        assert_eq!(extract_lang(text), None);
    }

    #[test]
    fn test_normalize_lang_code() {
        assert_eq!(normalize_lang_code("en_US"), "en-US");
        assert_eq!(normalize_lang_code("de_DE"), "de-DE");
        assert_eq!(normalize_lang_code("fr_FR"), "fr-FR");
        assert_eq!(normalize_lang_code("en-US"), "en-US"); // already normalized
    }

    #[test]
    fn test_extract_lang_multiple_matches_first_wins() {
        let text = "# lang: en_US\n// lang: de_DE";
        assert_eq!(extract_lang(text), Some("en-US".to_string()));
    }

    #[test]
    fn test_extract_lang_plain_text() {
        let text = "Some text lang: en_US here";
        assert_eq!(extract_lang(text), Some("en-US".to_string()));
    }

    #[test]
    fn test_extract_lang_empty_text() {
        let text = "";
        assert_eq!(extract_lang(text), None);
    }
}
}
