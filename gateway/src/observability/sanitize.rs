use std::io::{self, Write};
use std::sync::Arc;

use regex::Regex;
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer};

pub const SENSITIVE_FIELDS: &[&str] = &[
    "GATEWAY_BEARER_TOKEN",
    "BRIDGE_BEARER_TOKEN",
    "TELEGRAM_BOT_TOKEN",
    "SHOPIFY_CLIENT_SECRET",
    "DATABASE_URL",
    "POSTGRES_PASSWORD",
];

#[derive(Clone)]
pub struct Sanitizer {
    field_key_regex: Regex,
    url_regex: Regex,
    sensitive_values: Arc<Vec<String>>,
}

impl Sanitizer {
    pub fn new_from_env() -> Self {
        let sensitive_values = collect_sensitive_values_from_env();
        Self::build(sensitive_values)
    }

    #[cfg(test)]
    pub fn new_for_test(sensitive_values: Vec<String>) -> Self {
        Self::build(sensitive_values)
    }

    fn build(sensitive_values: Vec<String>) -> Self {
        let key_list = SENSITIVE_FIELDS
            .iter()
            .map(|v| regex::escape(v))
            .collect::<Vec<_>>()
            .join("|");
        let field_key_regex = Regex::new(&format!(r#"("(?:{})"\s*:\s*")([^"]*)(")"#, key_list))
            .expect("SENSITIVE_FIELDS regex must compile");

        let url_regex =
            Regex::new(r#"postgres(?:ql)?://[^\s\"]+"#).expect("database url regex must compile");

        Self {
            field_key_regex,
            url_regex,
            sensitive_values: Arc::new(sensitive_values),
        }
    }

    pub fn redact_line(&self, line: &str) -> String {
        let out = self
            .field_key_regex
            .replace_all(line, "$1[REDACTED]$3")
            .to_string();

        let mut out = self
            .url_regex
            .replace_all(&out, "postgres://[REDACTED]")
            .to_string();

        for value in self.sensitive_values.iter() {
            if value.is_empty() {
                continue;
            }
            out = out.replace(value, "[REDACTED]");
        }

        out
    }
}

fn collect_sensitive_values_from_env() -> Vec<String> {
    let mut values = Vec::new();

    for key in SENSITIVE_FIELDS {
        if let Ok(v) = std::env::var(key) {
            if !v.is_empty() {
                values.push(v);
            }
        }
    }

    for (k, v) in std::env::vars() {
        let key_upper = k.to_ascii_uppercase();
        if key_upper.contains("SHOPIFY") && key_upper.contains("SECRET") && !v.is_empty() {
            values.push(v);
        }
    }

    values.sort();
    values.dedup();
    values
}

#[derive(Clone)]
pub struct SanitizingMakeWriter {
    sanitizer: Sanitizer,
}

impl SanitizingMakeWriter {
    pub fn stdout() -> Self {
        Self {
            sanitizer: Sanitizer::new_from_env(),
        }
    }

    fn make_writer(&self) -> SanitizingWriter {
        SanitizingWriter {
            inner: io::stdout(),
            sanitizer: self.sanitizer.clone(),
            buf: Vec::new(),
        }
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for SanitizingMakeWriter {
    type Writer = SanitizingWriter;

    fn make_writer(&'a self) -> Self::Writer {
        self.make_writer()
    }
}

pub struct SanitizingWriter {
    inner: io::Stdout,
    sanitizer: Sanitizer,
    buf: Vec<u8>,
}

impl SanitizingWriter {
    fn flush_lines(&mut self) -> io::Result<()> {
        while let Some(pos) = self.buf.iter().position(|&b| b == b'\n') {
            let line_bytes: Vec<u8> = self.buf.drain(..=pos).collect();
            let line = String::from_utf8_lossy(&line_bytes);
            let sanitized = self.sanitizer.redact_line(&line);
            self.inner.write_all(sanitized.as_bytes())?;
        }
        Ok(())
    }
}

impl Write for SanitizingWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buf.extend_from_slice(buf);
        self.flush_lines()?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if !self.buf.is_empty() {
            let line = String::from_utf8_lossy(&self.buf);
            let sanitized = self.sanitizer.redact_line(&line);
            self.inner.write_all(sanitized.as_bytes())?;
            self.buf.clear();
        }
        self.inner.flush()
    }
}

#[derive(Default)]
pub struct SanitizeLayer;

impl<S> Layer<S> for SanitizeLayer
where
    S: tracing::Subscriber + for<'span> LookupSpan<'span>,
{
    fn on_event(&self, _event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        // Redaction is performed by SanitizingWriter to ensure all formatted output
        // is sanitized before it reaches stdout.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_sensitive_field_by_name() {
        let sanitizer = Sanitizer::new_for_test(vec![]);
        let input = r#"{"GATEWAY_BEARER_TOKEN":"abc123","message":"ok"}"#;
        let output = sanitizer.redact_line(input);

        assert!(output.contains(r#""GATEWAY_BEARER_TOKEN":"[REDACTED]""#));
        assert!(!output.contains("abc123"));
    }

    #[test]
    fn keeps_non_sensitive_fields_intact() {
        let sanitizer = Sanitizer::new_for_test(vec![]);
        let input = r#"{"chat_id":"chat-1","message":"ok"}"#;
        let output = sanitizer.redact_line(input);
        assert_eq!(input, output);
    }

    #[test]
    fn redacts_exact_sensitive_values() {
        let sanitizer = Sanitizer::new_for_test(vec!["bridge-secret".to_string()]);
        let input = r#"{"message":"token bridge-secret leaked"}"#;
        let output = sanitizer.redact_line(input);
        assert!(!output.contains("bridge-secret"));
        assert!(output.contains("[REDACTED]"));
    }
}
