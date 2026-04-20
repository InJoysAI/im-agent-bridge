use prometheus_client::encoding::text::encode;
use prometheus_client::metrics::counter::Counter;
use prometheus_client::registry::Registry;

#[derive(Clone, Debug)]
pub struct Metrics {
    pub messages_received_total: Counter,
    pub messages_replied_total: Counter,
    pub runtime_call_success_total: Counter,
    pub runtime_call_timeout_total: Counter,
    pub mcp_call_success_total: Counter,
    pub mcp_call_error_total: Counter,
    pub reply_write_success_total: Counter,
    pub reply_write_error_total: Counter,
    pub rate_limited_total: Counter,
    pub db_unavailable_total: Counter,
    pub runtime_log_write_failures_total: Counter,
}

impl Metrics {
    pub fn new(registry: &mut Registry) -> Self {
        let messages_received_total = Counter::default();
        registry.register(
            "messages_received",
            "Total inbound messages accepted into processing chain",
            messages_received_total.clone(),
        );

        let messages_replied_total = Counter::default();
        registry.register(
            "messages_replied",
            "Total replies successfully written to bridge",
            messages_replied_total.clone(),
        );

        let runtime_call_success_total = Counter::default();
        registry.register(
            "runtime_call_success",
            "Total successful runtime calls",
            runtime_call_success_total.clone(),
        );

        let runtime_call_timeout_total = Counter::default();
        registry.register(
            "runtime_call_timeout",
            "Total runtime timeout errors",
            runtime_call_timeout_total.clone(),
        );

        let mcp_call_success_total = Counter::default();
        registry.register(
            "mcp_call_success",
            "Total successful MCP calls (registered only in Gateway)",
            mcp_call_success_total.clone(),
        );

        let mcp_call_error_total = Counter::default();
        registry.register(
            "mcp_call_error",
            "Total MCP call errors (registered only in Gateway)",
            mcp_call_error_total.clone(),
        );

        let reply_write_success_total = Counter::default();
        registry.register(
            "reply_write_success",
            "Total successful bridge write operations",
            reply_write_success_total.clone(),
        );

        let reply_write_error_total = Counter::default();
        registry.register(
            "reply_write_error",
            "Total failed bridge write operations",
            reply_write_error_total.clone(),
        );

        let rate_limited_total = Counter::default();
        registry.register(
            "rate_limited",
            "Total inbound requests rejected by rate limiter",
            rate_limited_total.clone(),
        );

        let db_unavailable_total = Counter::default();
        registry.register(
            "db_unavailable",
            "Total database unavailable events observed in Gateway",
            db_unavailable_total.clone(),
        );

        let runtime_log_write_failures_total = Counter::default();
        registry.register(
            "runtime_log_write_failures",
            "Total runtime_logs insert failures",
            runtime_log_write_failures_total.clone(),
        );

        Self {
            messages_received_total,
            messages_replied_total,
            runtime_call_success_total,
            runtime_call_timeout_total,
            mcp_call_success_total,
            mcp_call_error_total,
            reply_write_success_total,
            reply_write_error_total,
            rate_limited_total,
            db_unavailable_total,
            runtime_log_write_failures_total,
        }
    }
}

pub fn encode_metrics(registry: &Registry) -> String {
    let mut out = String::new();
    encode(&mut out, registry).expect("encoding prometheus metrics should not fail");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_new_registers_all_counters() {
        let mut registry = Registry::default();
        let _metrics = Metrics::new(&mut registry);
        let output = encode_metrics(&registry);

        for name in [
            "messages_received_total",
            "messages_replied_total",
            "runtime_call_success_total",
            "runtime_call_timeout_total",
            "mcp_call_success_total",
            "mcp_call_error_total",
            "reply_write_success_total",
            "reply_write_error_total",
            "rate_limited_total",
            "db_unavailable_total",
            "runtime_log_write_failures_total",
        ] {
            assert!(
                output.contains(name),
                "expected metrics output to include {name}"
            );
        }

        assert!(
            output.contains("mcp_call_success_total 0"),
            "expected mcp_call_success_total to be present and initialized to 0"
        );
        assert!(
            output.contains("mcp_call_error_total 0"),
            "expected mcp_call_error_total to be present and initialized to 0"
        );
    }

    #[test]
    fn encode_reflects_counter_increment() {
        let mut registry = Registry::default();
        let metrics = Metrics::new(&mut registry);

        let before = encode_metrics(&registry);
        assert!(
            before.contains("messages_received_total"),
            "expected encoded output to contain the metric name"
        );

        metrics.messages_received_total.inc();
        let after = encode_metrics(&registry);
        assert!(
            before != after,
            "expected encoded output to change after counter increment"
        );
    }
}
