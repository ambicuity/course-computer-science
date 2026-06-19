use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
struct Span {
    trace_id: String,
    span_id: String,
    parent_span_id: Option<String>,
    operation_name: String,
    start_time_ms: u64,
    duration_ms: u64,
    tags: HashMap<String, String>,
}

impl Span {
    fn new(
        trace_id: &str,
        span_id: &str,
        parent_span_id: Option<&str>,
        operation_name: &str,
        start_time_ms: u64,
        duration_ms: u64,
    ) -> Self {
        Span {
            trace_id: trace_id.to_string(),
            span_id: span_id.to_string(),
            parent_span_id: parent_span_id.map(|s| s.to_string()),
            operation_name: operation_name.to_string(),
            start_time_ms,
            duration_ms,
            tags: HashMap::new(),
        }
    }

    fn with_tag(mut self, key: &str, value: &str) -> Self {
        self.tags.insert(key.to_string(), value.to_string());
        self
    }
}

#[derive(Clone, Debug)]
struct Trace {
    trace_id: String,
    spans: Vec<Span>,
}

impl Trace {
    fn new(trace_id: &str) -> Self {
        Trace {
            trace_id: trace_id.to_string(),
            spans: Vec::new(),
        }
    }

    fn add_span(&mut self, span: Span) {
        self.spans.push(span);
    }

    fn total_duration_ms(&self) -> u64 {
        self.spans
            .iter()
            .filter(|s| s.parent_span_id.is_none())
            .map(|s| s.duration_ms)
            .max()
            .unwrap_or(0)
    }

    fn root_span(&self) -> Option<&Span> {
        self.spans.iter().find(|s| s.parent_span_id.is_none())
    }

    fn children_of(&self, parent_span_id: &str) -> Vec<&Span> {
        self.spans
            .iter()
            .filter(|s| s.parent_span_id.as_deref() == Some(parent_span_id))
            .collect()
    }

    fn critical_path(&self) -> Vec<&Span> {
        let root = match self.root_span() {
            Some(r) => r,
            None => return Vec::new(),
        };
        let mut path = vec![root];
        let mut current_id = root.span_id.as_str();
        loop {
            let children = self.children_of(current_id);
            if children.is_empty() {
                break;
            }
            let slowest = children
                .iter()
                .max_by_key(|s| s.duration_ms)
                .unwrap();
            path.push(slowest);
            current_id = &slowest.span_id;
        }
        path
    }

    fn find_slowest_leaf_span(&self) -> Option<&Span> {
        let leaf_ids: std::collections::HashSet<_> = self
            .spans
            .iter()
            .filter_map(|s| s.parent_span_id.as_deref())
            .collect();
        self.spans
            .iter()
            .filter(|s| !leaf_ids.contains(s.span_id.as_str()))
            .max_by_key(|s| s.duration_ms)
    }

    fn find_slowest_span(&self) -> Option<&Span> {
        self.find_slowest_leaf_span()
    }

    fn visualize(&self) -> String {
        let root = match self.root_span() {
            Some(r) => r,
            None => return String::from("(empty trace)"),
        };
        let mut lines = Vec::new();
        lines.push(format!(
            "Trace {} (total: {}ms)",
            self.trace_id,
            self.total_duration_ms()
        ));
        self.visualize_span(root, "", &mut lines);
        lines.join("\n")
    }

    fn visualize_span(&self, span: &Span, indent: &str, lines: &mut Vec<String>) {
        let marker = if span == self.find_slowest_span().unwrap_or(span) {
            " ← SLOW SPAN"
        } else {
            ""
        };
        lines.push(format!(
            "{}├── {} ({}ms → {}ms, duration: {}ms){}",
            indent,
            span.operation_name,
            span.start_time_ms,
            span.start_time_ms + span.duration_ms,
            span.duration_ms,
            marker,
        ));
        let children = self.children_of(&span.span_id);
        for (i, child) in children.iter().enumerate() {
            let child_indent = if i == children.len() - 1 {
                format!("{}    ", indent)
            } else {
                format!("{}│   ", indent)
            };
            self.visualize_span(child, &child_indent, lines);
        }
    }
}

struct Metrics {
    counters: HashMap<String, u64>,
    gauges: HashMap<String, f64>,
    histograms: HashMap<String, Vec<f64>>,
}

impl Metrics {
    fn new() -> Self {
        Metrics {
            counters: HashMap::new(),
            gauges: HashMap::new(),
            histograms: HashMap::new(),
        }
    }

    fn incr_counter(&mut self, name: &str, delta: u64) {
        *self.counters.entry(name.to_string()).or_insert(0) += delta;
    }

    fn set_gauge(&mut self, name: &str, value: f64) {
        self.gauges.insert(name.to_string(), value);
    }

    fn observe_histogram(&mut self, name: &str, value: f64) {
        self.histograms
            .entry(name.to_string())
            .or_insert_with(Vec::new)
            .push(value);
    }

    fn counter_value(&self, name: &str) -> u64 {
        *self.counters.get(name).unwrap_or(&0)
    }

    fn gauge_value(&self, name: &str) -> Option<f64> {
        self.gauges.get(name).copied()
    }

    fn percentile(&self, name: &str, p: f64) -> Option<f64> {
        let values = self.histograms.get(name)?;
        if values.is_empty() {
            return None;
        }
        let mut sorted = values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let idx = (p / 100.0 * (sorted.len() as f64 - 1.0)).round() as usize;
        Some(sorted[idx.min(sorted.len() - 1)])
    }

    fn histogram_count(&self, name: &str) -> usize {
        self.histograms
            .get(name)
            .map(|v| v.len())
            .unwrap_or(0)
    }

    fn use_method_report(&self, resource: &str) -> String {
        let util = self
            .gauge_value(&format!("{}_utilization_pct", resource))
            .unwrap_or(0.0);
        let sat = self
            .gauge_value(&format!("{}_saturation_queue_depth", resource))
            .unwrap_or(0.0);
        let errs = self.counter_value(&format!("{}_errors_total", resource));
        format!(
            "USE report for '{}':\n  Utilization: {:.1}%\n  Saturation: {:.0} queued\n  Errors: {}",
            resource, util, sat, errs
        )
    }
}

struct StructuredLog {
    trace_id: String,
    span_id: String,
}

impl StructuredLog {
    fn new(trace_id: &str, span_id: &str) -> Self {
        StructuredLog {
            trace_id: trace_id.to_string(),
            span_id: span_id.to_string(),
        }
    }

    fn emit(&self, level: &str, message: &str, extra: &[(&str, &str)]) -> String {
        let mut fields = vec![
            format!(r#""timestamp":"{}""#, chrono_now()),
            format!(r#""level":"{}""#, level),
            format!(r#""trace_id":"{}""#, self.trace_id),
            format!(r#""span_id":"{}""#, self.span_id),
            format!(r#""message":"{}""#, message),
        ];
        for (k, v) in extra {
            fields.push(format!(r#""{}":"{}""#, k, v));
        }
        format!("{{{}}}", fields.join(", "))
    }
}

fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", duration.as_millis())
}

struct TraceContext {
    trace_id: String,
    parent_span_id: Option<String>,
}

impl TraceContext {
    fn new_root(trace_id: &str) -> Self {
        TraceContext {
            trace_id: trace_id.to_string(),
            parent_span_id: None,
        }
    }

    fn child(&self, parent_span_id: &str) -> Self {
        TraceContext {
            trace_id: self.trace_id.clone(),
            parent_span_id: Some(parent_span_id.to_string()),
        }
    }

    fn propagating_headers(&self) -> Vec<(String, String)> {
        let mut headers = vec![(
            "traceparent".to_string(),
            format!("00-{}-{}-01", self.trace_id, self.parent_span_id.as_deref().unwrap_or("0")),
        )];
        if let Some(ref psid) = self.parent_span_id {
            headers.push(("X-B3-ParentSpanId".to_string(), psid.clone()));
        }
        headers
    }
}

struct TraceCollector {
    traces: HashMap<String, Trace>,
}

impl TraceCollector {
    fn new() -> Self {
        TraceCollector {
            traces: HashMap::new(),
        }
    }

    fn collect(&mut self, span: Span) {
        let trace = self
            .traces
            .entry(span.trace_id.clone())
            .or_insert_with(|| Trace::new(&span.trace_id));
        trace.add_span(span);
    }

    fn get_trace(&self, trace_id: &str) -> Option<&Trace> {
        self.traces.get(trace_id)
    }

    fn all_trace_ids(&self) -> Vec<String> {
        self.traces.keys().cloned().collect()
    }
}

struct MiniExporter<'a> {
    collector: &'a TraceCollector,
    metrics: &'a Metrics,
}

impl<'a> MiniExporter<'a> {
    fn new(collector: &'a TraceCollector, metrics: &'a Metrics) -> Self {
        MiniExporter { collector, metrics }
    }

    fn export_traces(&self) {
        println!("\n{}", "═".repeat(60));
        println!("TRACE EXPORT");
        println!("{}", "═".repeat(60));
        for trace_id in self.collector.all_trace_ids() {
            if let Some(trace) = self.collector.get_trace(&trace_id) {
                println!();
                println!("{}", trace.visualize());
                println!();
                println!("Critical path (bottleneck chain):");
                let path = trace.critical_path();
                for span in &path {
                    println!(
                        "  → {} ({}ms)",
                        span.operation_name, span.duration_ms
                    );
                }
                println!("  Total trace duration: {}ms", trace.total_duration_ms());
                if let Some(slowest) = trace.find_slowest_span() {
                    println!(
                        "\nSlowest span: {} at {}ms",
                        slowest.operation_name, slowest.duration_ms
                    );
                }
            }
        }
    }

    fn export_metrics(&self) {
        println!("\n{}", "═".repeat(60));
        println!("METRICS EXPORT");
        println!("{}", "═".repeat(60));
        println!("\nCounters:");
        for (name, value) in &self.metrics.counters {
            println!("  {} {}", name, value);
        }
        println!("\nGauges:");
        for (name, value) in &self.metrics.gauges {
            println!("  {} {:.1}", name, value);
        }
        println!("\nHistograms:");
        for (name, _) in &self.metrics.histograms {
            let count = self.metrics.histogram_count(name);
            let p50 = self.metrics.percentile(name, 50.0);
            let p95 = self.metrics.percentile(name, 95.0);
            let p99 = self.metrics.percentile(name, 99.0);
            println!(
                "  {} count={} p50={:.1}ms p95={:.1}ms p99={:.1}ms",
                name,
                count,
                p50.unwrap_or(0.0),
                p95.unwrap_or(0.0),
                p99.unwrap_or(0.0),
            );
        }
    }

    fn export_use_report(&self, resource: &str) {
        println!("\n{}", self.metrics.use_method_report(resource));
    }
}

fn simulate_request() {
    let trace_id = "abc123def456";
    let mut collector = TraceCollector::new();
    let mut metrics = Metrics::new();

    let ctx = TraceContext::new_root(trace_id);
    let gw_span_id = "span_gw_001";
    let auth_span_id = "span_auth_001";
    let orders_span_id = "span_ord_001";
    let db_span_id = "span_db_001";
    let cache_span_id = "span_cache_001";

    metrics.incr_counter("http_requests_total", 1);
    metrics.set_gauge("db_connection_pool_utilization_pct", 94.0);
    metrics.set_gauge("db_connection_pool_saturation_queue_depth", 12.0);
    metrics.incr_counter("db_connection_pool_errors_total", 3);

    let gw_span = Span::new(trace_id, gw_span_id, None, "gateway.handle_request", 0, 2341)
        .with_tag("http.method", "GET")
        .with_tag("http.path", "/orders/42");

    let log_gw = StructuredLog::new(trace_id, gw_span_id);
    println!("{}", log_gw.emit("info", "request started", &[("service", "gateway"), ("http.method", "GET"), ("http.path", "/orders/42")]));

    let headers = ctx.propagating_headers();
    println!("[gateway → auth] Propagating headers: {:?}", headers);

    let auth_ctx = ctx.child(gw_span_id);
    let auth_span = Span::new(trace_id, auth_span_id, Some(gw_span_id), "auth.validate_token", 1, 15)
        .with_tag("service", "auth")
        .with_tag("user.id", "user_789");

    let log_auth = StructuredLog::new(trace_id, auth_span_id);
    println!("{}", log_auth.emit("info", "token validated", &[("service", "auth"), ("user.id", "user_789")]));

    let orders_ctx = auth_ctx.child(gw_span_id);
    let orders_span = Span::new(trace_id, orders_span_id, Some(gw_span_id), "orders.get_order", 16, 2325)
        .with_tag("service", "orders")
        .with_tag("order.id", "42");

    let log_orders = StructuredLog::new(trace_id, orders_span_id);
    println!("{}", log_orders.emit("info", "fetching order", &[("service", "orders"), ("order.id", "42")]));

    let db_ctx = orders_ctx.child(orders_span_id);
    let db_span = Span::new(trace_id, db_span_id, Some(orders_span_id), "db.query", 17, 2318)
        .with_tag("service", "db")
        .with_tag("db.statement", "SELECT * FROM orders WHERE id = 42")
        .with_tag("error", "true");

    let log_db = StructuredLog::new(trace_id, db_span_id);
    println!("{}", log_db.emit("error", "database connection pool exhausted", &[
        ("service", "db"),
        ("pool_active", "50"),
        ("pool_max", "50"),
        ("pool_waiters", "12"),
        ("error_code", "CONN_POOL_EXHAUSTED"),
    ]));

    metrics.observe_histogram("http_request_duration_ms", 2341.0);

    let _cache_ctx = db_ctx.child(orders_span_id);
    let cache_span = Span::new(trace_id, cache_span_id, Some(orders_span_id), "cache.lookup", 2335, 2)
        .with_tag("service", "cache")
        .with_tag("cache.result", "miss");

    let log_cache = StructuredLog::new(trace_id, cache_span_id);
    println!("{}", log_cache.emit("warn", "cache miss for order", &[("service", "cache"), ("order.id", "42")]));

    collector.collect(gw_span);
    collector.collect(auth_span);
    collector.collect(orders_span);
    collector.collect(db_span);
    collector.collect(cache_span);

    let exporter = MiniExporter::new(&collector, &metrics);

    println!("\n{}", "═".repeat(60));
    println!("STRUCTURED LOGS (correlated by trace_id)");
    println!("{}", "═".repeat(60));
    println!("  All logs above share trace_id={}", trace_id);
    println!("  Jump from metric alert → trace → log lines instantly.");

    exporter.export_traces();
    exporter.export_metrics();
    exporter.export_use_report("db_connection_pool");

    println!("\n{}", "═".repeat(60));
    println!("DEBUGGING JOURNEY");
    println!("{}", "═".repeat(60));
    println!("  1. ALERT: p99 latency > 2s (metric breach detected)");
    println!("  2. TRACE: Find trace {} → gateway took 2341ms", trace_id);
    println!("  3. SPAN:  db.query is the slow span at 2318ms");
    println!("  4. LOG:   Search logs for trace_id={} span_id={}", trace_id, db_span_id);
    println!("  5. ROOT CAUSE: \"connection pool exhausted\" — pool at capacity (50/50)");
}

fn simulate_percentiles() {
    println!("\n{}", "═".repeat(60));
    println!("PERCENTILE DEMO");
    println!("{}", "═".repeat(60));

    let mut metrics = Metrics::new();
    let latencies = [
        12.0, 15.0, 18.0, 20.0, 22.0, 25.0, 28.0, 30.0, 35.0, 40.0,
        45.0, 50.0, 55.0, 60.0, 70.0, 80.0, 100.0, 150.0, 300.0, 2341.0,
    ];

    for lat in &latencies {
        metrics.observe_histogram("request_latency_ms", *lat);
    }

    println!("  {} observations, latency range: {:.0}ms - {:.0}ms", latencies.len(), latencies[0], latencies[latencies.len()-1]);
    println!("  p50  = {:.0}ms  (half of requests are faster than this)", metrics.percentile("request_latency_ms", 50.0).unwrap());
    println!("  p95  = {:.0}ms  (5% of requests are slower than this)", metrics.percentile("request_latency_ms", 95.0).unwrap());
    println!("  p99  = {:.0}ms  (1% of requests are slower than this)", metrics.percentile("request_latency_ms", 99.0).unwrap());
    println!();
    println!("  Average would be {:.0}ms — misleading because one 2341ms outlier", latencies.iter().sum::<f64>() / latencies.len() as f64);
    println!("  p50 tells you the typical experience; p99 tells you the worst case.");
}

fn main() {
    println!("{}", "═".repeat(60));
    println!("OBSERVABILITY: METRICS, TRACES, LOGS");
    println!("{}", "═".repeat(60));

    simulate_request();
    simulate_percentiles();
}