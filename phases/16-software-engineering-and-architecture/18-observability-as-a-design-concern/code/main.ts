interface LogEntry {
  timestamp: string;
  level: "TRACE" | "DEBUG" | "INFO" | "WARN" | "ERROR" | "FATAL";
  message: string;
  service: string;
  trace_id?: string;
  span_id?: string;
  route?: string;
  status_code?: number;
  latency_ms?: number;
  error?: string;
  user_id?: string;
}

interface Span {
  traceId: string;
  spanId: string;
  parentId?: string;
  operation: string;
  startTime: number;
  endTime?: number;
  attributes: Record<string, string>;
  status: "ok" | "error";
}

interface MetricPoint {
  name: string;
  value: number;
  labels: Record<string, string>;
  timestamp: number;
  type: "counter" | "gauge" | "histogram";
}

const SERVICE_NAME = "api-server";
const spanStorage: Span[] = [];
const metricStorage: MetricPoint[] = [];
const histogramBuckets = [5, 10, 25, 50, 100, 250, 500, 1000, 2500, 5000, 10000];
const latencyHistogram: Record<string, number> = {};
let latencySum = 0;
let latencyCount = 0;

for (const bound of histogramBuckets) {
  latencyHistogram[`le_${bound}`] = 0;
}
latencyHistogram["le_inf"] = 0;

function emitLog(entry: Partial<LogEntry> & { message: string }): void {
  const full: LogEntry = {
    timestamp: new Date().toISOString(),
    level: entry.level ?? "INFO",
    message: entry.message,
    service: entry.service ?? SERVICE_NAME,
    ...entry,
  } as LogEntry;
  console.log(JSON.stringify(full));
}

function generateId(length: number): string {
  const chars = "0123456789abcdef";
  let id = "";
  for (let i = 0; i < length; i++) {
    id += chars[Math.floor(Math.random() * chars.length)];
  }
  return id;
}

function generateTraceId(): string {
  return generateId(16);
}

function generateSpanId(): string {
  return generateId(8);
}

interface RequestContext {
  traceId: string;
  spanId: string;
  parentId?: string;
}

function createRequestContext(
  incomingTraceId?: string,
  parentSpanId?: string
): RequestContext {
  const traceId = incomingTraceId ?? generateTraceId();
  const spanId = generateSpanId();
  return { traceId, spanId, parentId: parentSpanId };
}

function startSpan(
  ctx: RequestContext,
  operation: string,
  attributes?: Record<string, string>
): Span {
  const span: Span = {
    traceId: ctx.traceId,
    spanId: ctx.spanId,
    parentId: ctx.parentId,
    operation,
    startTime: Date.now(),
    attributes: attributes ?? {},
    status: "ok",
  };
  spanStorage.push(span);
  return span;
}

function endSpan(span: Span): void {
  span.endTime = Date.now();
}

function recordCounter(
  name: string,
  value: number,
  labels: Record<string, string>
): void {
  metricStorage.push({
    name,
    value,
    labels,
    timestamp: Date.now(),
    type: "counter",
  });
}

function recordLatency(ms: number): void {
  latencySum += ms;
  latencyCount += 1;
  for (const bound of histogramBuckets) {
    if (ms <= bound) {
      latencyHistogram[`le_${bound}`] += 1;
    }
  }
  latencyHistogram["le_inf"] += 1;
}

function recordRedMetrics(
  route: string,
  statusCode: number,
  latencyMs: number
): void {
  recordCounter("http_requests_total", 1, { route, method: "GET" });
  if (statusCode >= 400) {
    recordCounter("http_errors_total", 1, {
      route,
      method: "GET",
      status_code: String(statusCode),
    });
  }
  recordLatency(latencyMs);
}

function formatMetrics(): string {
  const lines: string[] = [];
  const agg: Record<string, { value: number; type: string; labels: Record<string, string> }> = {};
  for (const m of metricStorage) {
    const key = `${m.name}:${JSON.stringify(m.labels)}`;
    if (!agg[key]) {
      agg[key] = { value: 0, type: m.type, labels: m.labels };
    }
    if (m.type === "counter") {
      agg[key].value += m.value;
    } else if (m.type === "gauge") {
      agg[key].value = m.value;
    }
  }
  lines.push("# HELP http_requests_total Total HTTP requests");
  lines.push("# TYPE http_requests_total counter");
  lines.push(`http_requests_total ${Object.values(agg).find(a => Object.keys(a.labels).includes("route") && a.labels.route)?.value ?? 0}`);

  lines.push("# HELP http_errors_total Total HTTP errors");
  lines.push("# TYPE http_errors_total counter");

  lines.push("# HELP http_request_duration_ms Histogram of request durations");
  lines.push("# TYPE http_request_duration_ms histogram");
  let cumulative = 0;
  for (const bound of histogramBuckets) {
    cumulative += latencyHistogram[`le_${bound}`];
    lines.push(`http_request_duration_ms_bucket{le="${bound}"} ${cumulative}`);
  }
  lines.push(`http_request_duration_ms_bucket{le="+Inf"} ${latencyHistogram["le_inf"]}`);
  lines.push(`http_request_duration_ms_sum ${latencySum}`);
  lines.push(`http_request_duration_ms_count ${latencyCount}`);
  return lines.join("\n");
}

interface HandlerResult {
  statusCode: number;
  body: Record<string, unknown>;
  error?: string;
}

function healthHandler(_ctx: RequestContext): HandlerResult {
  return { statusCode: 200, body: { status: "ok" } };
}

function usersHandler(ctx: RequestContext): HandlerResult {
  const span = startSpan(ctx, "GET /users", { "http.method": "GET" });
  const userId = String(Math.floor(Math.random() * 1000) + 1);

  const childCtx: RequestContext = {
    traceId: ctx.traceId,
    spanId: generateSpanId(),
    parentId: ctx.spanId,
  };

  startSpan(childCtx, "db.query", {
    "db.system": "postgres",
    "db.operation": "SELECT",
    "user.id": userId,
  });

  emitLog({
    level: "DEBUG",
    message: "fetching user from database",
    service: "user-service",
    trace_id: ctx.traceId,
    span_id: childCtx.spanId,
    route: "GET /users",
    user_id: userId,
  });

  if (Math.random() < 0.15) {
    const delayMs = Math.floor(Math.random() * 2000) + 500;
    span.status = "error";
    span.attributes["error.type"] = "timeout";
    endSpan(span);
    return {
      statusCode: 504,
      body: { error: "database timeout" },
      error: `timeout after ${delayMs}ms querying user ${userId}`,
    };
  }

  endSpan(span);
  return {
    statusCode: 200,
    body: { id: userId, name: `user_${userId}`, email: `${userId}@example.com` },
  };
}

function ordersHandler(ctx: RequestContext): HandlerResult {
  const span = startSpan(ctx, "GET /orders", { "http.method": "GET" });
  const orderId = String(Math.floor(Math.random() * 5000) + 1);

  const childCtx: RequestContext = {
    traceId: ctx.traceId,
    spanId: generateSpanId(),
    parentId: ctx.spanId,
  };

  startSpan(childCtx, "http.client", {
    "http.method": "POST",
    "http.url": "http://payment-service/charge",
    "order.id": orderId,
  });

  emitLog({
    level: "DEBUG",
    message: "calling payment service for order",
    service: "order-service",
    trace_id: ctx.traceId,
    span_id: childCtx.spanId,
    route: "GET /orders",
    user_id: orderId,
  });

  if (Math.random() < 0.1) {
    span.status = "error";
    span.attributes["error.type"] = "upstream_error";
    endSpan(span);
    return {
      statusCode: 502,
      body: { error: "payment service unavailable" },
      error: `payment service error for order ${orderId}`,
    };
  }

  endSpan(span);
  return {
    statusCode: 200,
    body: { order_id: orderId, status: "confirmed", total: "29.99" },
  };
}

type RouteHandler = (ctx: RequestContext) => HandlerResult;

function observeHandler(
  handler: RouteHandler,
  route: string
): (traceparent?: string) => HandlerResult {
  return (traceparent?: string): HandlerResult => {
    const incomingTraceId = traceparent ?? undefined;
    const ctx = createRequestContext(incomingTraceId);
    const start = Date.now();

    const result = handler(ctx);

    const latencyMs = Date.now() - start;
    recordRedMetrics(route, result.statusCode, latencyMs);

    const level = result.statusCode >= 500 ? "ERROR" : result.statusCode >= 400 ? "WARN" : "INFO";
    emitLog({
      level: level as LogEntry["level"],
      message: "request completed",
      trace_id: ctx.traceId,
      span_id: ctx.spanId,
      route,
      status_code: result.statusCode,
      latency_ms: latencyMs,
      error: result.error,
    });

    return result;
  };
}

function simulateTraffic(): void {
  const routes: { path: string; handler: RouteHandler }[] = [
    { path: "/health", handler: healthHandler },
    { path: "/users", handler: usersHandler },
    { path: "/orders", handler: ordersHandler },
  ];

  const observed = routes.map((r) => ({
    path: r.path,
    handler: observeHandler(r.handler, `GET ${r.path}`),
  }));

  console.log("=== Simulating observed HTTP traffic ===\n");

  for (let i = 0; i < 20; i++) {
    const route = observed[Math.floor(Math.random() * observed.length)];
    const traceId = i < 3 ? "parent-trace-abc123" : undefined;
    const result = route.handler(traceId);
    console.log(
      `  -> ${route.path} => ${result.statusCode} ${JSON.stringify(result.body).slice(0, 60)}`
    );
  }

  console.log("\n=== Distributed Traces ===\n");
  const traces: Record<string, Span[]> = {};
  for (const span of spanStorage) {
    if (!traces[span.traceId]) traces[span.traceId] = [];
    traces[span.traceId].push(span);
  }

  for (const [traceId, spans] of Object.entries(traces).slice(0, 5)) {
    console.log(`Trace: ${traceId}`);
    for (const span of spans) {
      const duration = span.endTime ? span.endTime - span.startTime : "?";
      const parent = span.parentId ? ` (parent: ${span.parentId})` : "";
      console.log(
        `  Span ${span.spanId}: ${span.operation} [${duration}ms]${parent} status=${span.status}`
      );
    }
    console.log("");
  }

  console.log("=== Prometheus Metrics ===\n");
  console.log(formatMetrics());

  console.log("\n=== Observability Summary ===");
  console.log(`Total requests: ${latencyCount}`);
  console.log(`Total errors: ${metricStorage.filter(m => m.name === "http_errors_total").length}`);
  console.log(`Avg latency: ${latencyCount > 0 ? Math.round(latencySum / latencyCount) : 0}ms`);
  console.log(`Total spans: ${spanStorage.length}`);
  console.log(`Total traces: ${Object.keys(traces).length}`);
}

simulateTraffic();