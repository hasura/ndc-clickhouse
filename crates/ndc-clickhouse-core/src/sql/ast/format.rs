use std::fmt;

pub struct EscapedString<'a>(&'a str);
impl<'a> fmt::Display for EscapedString<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for c in self.0.chars() {
            match c {
                '\t' => write!(f, "\\t")?,
                '\n' => write!(f, "\\n")?,
                '\r' => write!(f, "\\r")?,
                '\'' => write!(f, "\\'")?,
                '\\' => write!(f, "\\\\")?,
                _ => write!(f, "{c}")?,
            }
        }
        Ok(())
    }
}
/// clickhouse docs state that backslash and single quotes must be escaped
/// docs: https://clickhouse.com/docs/en/sql-reference/syntax#syntax-string-literal
pub fn escape_string(s: &str) -> EscapedString {
    EscapedString(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_escape_string() {
        let test_cases = vec![
            ("", ""),
            ("foo", "foo"),
            ("foo\nbar", "foo\\nbar"),
            ("\\\n\t\r'", "\\\\\\n\\t\\r\\'"),
        ];

        for (raw, escaped) in test_cases {
            assert_eq!(escape_string(raw).to_string().as_str(), escaped)
        }
    }
}
