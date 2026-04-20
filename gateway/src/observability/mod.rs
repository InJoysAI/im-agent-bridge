use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub mod metrics;
pub mod sanitize;

use sanitize::{SanitizeLayer, SanitizingMakeWriter};

pub fn init_subscriber() {
    build_subscriber(SanitizingMakeWriter::stdout()).init();
}

fn build_subscriber<W>(writer: W) -> impl tracing::Subscriber + Send + Sync
where
    W: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let formatting = tracing_subscriber::fmt::layer().json().with_writer(writer);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(SanitizeLayer)
        .with(formatting)
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::sync::{Arc, Mutex};

    use tracing::Dispatch;
    use tracing_subscriber::fmt::MakeWriter;

    use super::build_subscriber;
    use super::sanitize::Sanitizer;

    #[derive(Clone, Default)]
    struct SharedWriter {
        buf: Arc<Mutex<Vec<u8>>>,
    }

    impl SharedWriter {
        fn to_string(&self) -> String {
            let bytes = self.buf.lock().expect("buffer lock poisoned").clone();
            String::from_utf8(bytes).expect("buffer should contain utf8")
        }
    }

    struct SharedWriterGuard {
        buf: Arc<Mutex<Vec<u8>>>,
    }

    impl io::Write for SharedWriterGuard {
        fn write(&mut self, data: &[u8]) -> io::Result<usize> {
            self.buf.lock().expect("buffer lock poisoned").extend(data);
            Ok(data.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl<'a> MakeWriter<'a> for SharedWriter {
        type Writer = SharedWriterGuard;

        fn make_writer(&'a self) -> Self::Writer {
            SharedWriterGuard {
                buf: self.buf.clone(),
            }
        }
    }

    #[test]
    fn json_log_sanitization_keeps_valid_json_and_redacts_sensitive_values() {
        let sanitizer = Sanitizer::new_for_test(vec!["top-secret".to_string()]);
        let line = r#"{"level":"INFO","message":"hello","token":"top-secret"}"#;
        let out = sanitizer.redact_line(line);

        let parsed: serde_json::Value = serde_json::from_str(&out).expect("valid json expected");
        assert_eq!(parsed["token"], "[REDACTED]");
    }

    #[test]
    fn init_subscriber_like_pipeline_outputs_json() {
        let writer = SharedWriter::default();
        let subscriber = build_subscriber(writer.clone());
        let dispatch = Dispatch::new(subscriber);

        tracing::dispatcher::with_default(&dispatch, || {
            tracing::info!(event_id = "evt-test", "hello-observability");
        });

        let output = writer.to_string();
        let line = output
            .lines()
            .find(|line| !line.trim().is_empty())
            .expect("expected at least one log line");
        let parsed: serde_json::Value = serde_json::from_str(line).expect("json log expected");
        assert_eq!(parsed["level"], "INFO");
        assert_eq!(parsed["fields"]["event_id"], "evt-test");
    }
}
