//! Boolean value parsing utilities
//!
//! Provides flexible parsing of boolean values from strings,
//! supporting common truthy representations.

/// Parse a string into a boolean value.
///
/// Returns `true` for: "1", "true", "yes", "on" (case-insensitive)
/// Returns `false` for everything else.
///
/// # Examples
///
/// ```
/// use ticca_config::boolean_parsing::parse_bool;
///
/// assert!(parse_bool("true"));
/// assert!(parse_bool("TRUE"));
/// assert!(parse_bool("1"));
/// assert!(parse_bool("yes"));
/// assert!(parse_bool("YES"));
/// assert!(parse_bool("on"));
/// assert!(parse_bool("ON"));
///
/// assert!(!parse_bool("false"));
/// assert!(!parse_bool("0"));
/// assert!(!parse_bool("no"));
/// assert!(!parse_bool("anything_else"));
/// ```
pub fn parse_bool(value: &str) -> bool {
    matches!(
        value.to_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

/// Parse an optional string into a boolean.
///
/// Returns `None` if the input is `None`, otherwise parses the value.
pub fn parse_bool_opt(value: Option<&str>) -> Option<bool> {
    value.map(parse_bool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truthy_values() {
        assert!(parse_bool("1"));
        assert!(parse_bool("true"));
        assert!(parse_bool("TRUE"));
        assert!(parse_bool("True"));
        assert!(parse_bool("yes"));
        assert!(parse_bool("YES"));
        assert!(parse_bool("Yes"));
        assert!(parse_bool("on"));
        assert!(parse_bool("ON"));
        assert!(parse_bool("On"));
    }

    #[test]
    fn test_falsy_values() {
        assert!(!parse_bool("0"));
        assert!(!parse_bool("false"));
        assert!(!parse_bool("FALSE"));
        assert!(!parse_bool("no"));
        assert!(!parse_bool("NO"));
        assert!(!parse_bool("off"));
        assert!(!parse_bool("OFF"));
        assert!(!parse_bool(""));
        assert!(!parse_bool("random"));
        assert!(!parse_bool("2"));
    }

    #[test]
    fn test_parse_bool_opt() {
        assert_eq!(parse_bool_opt(None), None);
        assert_eq!(parse_bool_opt(Some("true")), Some(true));
        assert_eq!(parse_bool_opt(Some("false")), Some(false));
    }
}
