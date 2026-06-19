package main

import (
	"context"
	"encoding/json"
	"fmt"
	"log"
	"math/rand"
	"net/http"
	"os"
	"strconv"
	"sync/atomic"
	"time"
)

type LogEntry struct {
	Timestamp  string `json:"timestamp"`
	Level      string `json:"level"`
	Message    string `json:"message"`
	Service    string `json:"service"`
	TraceID    string `json:"trace_id,omitempty"`
	SpanID     string `json:"span_id,omitempty"`
	Route      string `json:"route,omitempty"`
	StatusCode int    `json:"status_code,omitempty"`
	LatencyMs  int64  `json:"latency_ms,omitempty"`
	Error      string `json:"error,omitempty"`
	UserID     string `json:"user_id,omitempty"`
}

var (
	logger = log.New(os.Stdout, "", 0)

	requestTotal   atomic.Int64
	errorTotal     atomic.Int64
	latencySum     atomic.Int64
	latencyCount   atomic.Int64
	latencyBuckets [12]atomic.Int64

	bucketBounds = []float64{5, 10, 25, 50, 100, 250, 500, 1000, 2500, 5000, 10000}
)

func emitLog(entry LogEntry) {
	if entry.Timestamp == "" {
		entry.Timestamp = time.Now().UTC().Format(time.RFC3339Nano)
	}
	data, _ := json.Marshal(entry)
	logger.Println(string(data))
}

func generateTraceID() string {
	b := make([]byte, 16)
	for i := range b {
		b[i] = "0123456789abcdef"[rand.Intn(16)]
	}
	return string(b)
}

func generateSpanID() string {
	b := make([]byte, 8)
	for i := range b {
		b[i] = "0123456789abcdef"[rand.Intn(16)]
	}
	return string(b)
}

type contextKey string

const (
	traceIDKey contextKey = "trace_id"
	spanIDKey  contextKey = "span_id"
)

func withTraceContext(ctx context.Context, traceID, spanID string) context.Context {
	ctx = context.WithValue(ctx, traceIDKey, traceID)
	ctx = context.WithValue(ctx, spanIDKey, spanID)
	return ctx
}

func traceIDFromContext(ctx context.Context) string {
	if v, ok := ctx.Value(traceIDKey).(string); ok {
		return v
	}
	return ""
}

func spanIDFromContext(ctx context.Context) string {
	if v, ok := ctx.Value(spanIDKey).(string); ok {
		return v
	}
	return ""
}

func recordLatency(ms int64) {
	latencySum.Add(ms)
	latencyCount.Add(1)
	for i, bound := range bucketBounds {
		if float64(ms) <= bound {
			latencyBuckets[i].Add(1)
			return
		}
	}
	latencyBuckets[len(bucketBounds)].Add(1)
}

func recordRequest(statusCode int, latencyMs int64) {
	requestTotal.Add(1)
	if statusCode >= 400 {
		errorTotal.Add(1)
	}
	recordLatency(latencyMs)
}

func renderHistogram() string {
	var lines []string
	lines = append(lines, "# HELP http_request_duration_seconds Histogram of request durations")
	lines = append(lines, "# TYPE http_request_duration_seconds histogram")
	count := int64(0)
	for i, bound := range bucketBounds {
		count += latencyBuckets[i].Load()
		lines = append(lines, fmt.Sprintf(
			"http_request_duration_seconds_bucket{le=\"%g\"} %d",
			bound/1000.0, count,
		))
	}
	count += latencyBuckets[len(bucketBounds)].Load()
	lines = append(lines, fmt.Sprintf(
		"http_request_duration_seconds_bucket{le=\"+Inf\"} %d", count,
	))
	lines = append(lines, fmt.Sprintf(
		"http_request_duration_seconds_sum %f",
		float64(latencySum.Load())/1000.0,
	))
	lines = append(lines, fmt.Sprintf(
		"http_request_duration_seconds_count %d",
		latencyCount.Load(),
	))
	var result string
	for _, l := range lines {
		result += l + "\n"
	}
	return result
}

func renderCounters() string {
	var out string
	out += "# HELP http_requests_total Total HTTP requests\n"
	out += "# TYPE http_requests_total counter\n"
	out += fmt.Sprintf("http_requests_total %d\n", requestTotal.Load())
	out += "# HELP http_errors_total Total HTTP errors (status >= 400)\n"
	out += "# TYPE http_errors_total counter\n"
	out += fmt.Sprintf("http_errors_total %d\n", errorTotal.Load())
	return out
}

func metricsHandler(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "text/plain; version=0.0.4")
	fmt.Fprint(w, renderCounters())
	fmt.Fprint(w, renderHistogram())
}

type observedHandler struct {
	next    http.HandlerFunc
	route   string
	service string
}

func (h *observedHandler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	traceID := r.Header.Get("traceparent")
	if traceID == "" {
		traceID = generateTraceID()
	}
	spanID := generateSpanID()
	ctx := withTraceContext(r.Context(), traceID, spanID)

	start := time.Now()

	rw := &responseWriter{ResponseWriter: w, statusCode: 200}
	h.next(rw, r.WithContext(ctx))

	latencyMs := time.Since(start).Milliseconds()
	recordRequest(rw.statusCode, latencyMs)

	level := "INFO"
	if rw.statusCode >= 500 {
		level = "ERROR"
	} else if rw.statusCode >= 400 {
		level = "WARN"
	}

	entry := LogEntry{
		Level:      level,
		Message:    "request completed",
		Service:    h.service,
		TraceID:    traceID,
		SpanID:     spanID,
		Route:      h.route,
		StatusCode: rw.statusCode,
		LatencyMs:  latencyMs,
	}
	if rw.statusCode >= 400 {
		entry.Error = fmt.Sprintf("HTTP %d on %s", rw.statusCode, h.route)
	}
	emitLog(entry)
}

type responseWriter struct {
	http.ResponseWriter
	statusCode int
}

func (rw *responseWriter) WriteHeader(code int) {
	rw.statusCode = code
	rw.ResponseWriter.WriteHeader(code)
}

func healthHandler(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]string{"status": "ok"})
}

func usersHandler(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()
	userID := r.URL.Query().Get("id")
	if userID == "" {
		userID = strconv.Itoa(rand.Intn(1000) + 1)
	}

	childSpanID := generateSpanID()
	ctx = withTraceContext(ctx, traceIDFromContext(ctx), childSpanID)

	emitLog(LogEntry{
		Level:   "DEBUG",
		Message: "fetching user",
		Service: "user-service",
		TraceID: traceIDFromContext(ctx),
		SpanID:  childSpanID,
		UserID:  userID,
		Route:   "GET /users",
	})

	if rand.Float64() < 0.15 {
		time.Sleep(time.Duration(rand.Intn(2000)+500) * time.Millisecond)
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]string{
		"id":   userID,
		"name": fmt.Sprintf("user_%s", userID),
	})
}

func ordersHandler(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()
	orderID := r.URL.Query().Get("id")
	if orderID == "" {
		orderID = strconv.Itoa(rand.Intn(5000) + 1)
	}

	childSpanID := generateSpanID()
	ctx = withTraceContext(ctx, traceIDFromContext(ctx), childSpanID)

	emitLog(LogEntry{
		Level:   "DEBUG",
		Message: "processing order",
		Service: "order-service",
		TraceID: traceIDFromContext(ctx),
		SpanID:  childSpanID,
		Route:   "GET /orders",
	})

	if rand.Float64() < 0.1 {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusServiceUnavailable)
		json.NewEncoder(w).Encode(map[string]string{
			"error": "payment service unavailable",
		})
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]string{
		"order_id": orderID,
		"status":   "confirmed",
	})
}

func main() {
	mux := http.NewServeMux()
	mux.Handle("/health", &observedHandler{
		next: healthHandler, route: "GET /health", service: "api-server",
	})
	mux.Handle("/users", &observedHandler{
		next: usersHandler, route: "GET /users", service: "api-server",
	})
	mux.Handle("/orders", &observedHandler{
		next: ordersHandler, route: "GET /orders", service: "api-server",
	})
	mux.HandleFunc("/metrics", metricsHandler)

	port := os.Getenv("PORT")
	if port == "" {
		port = "8080"
	}

	emitLog(LogEntry{
		Level:   "INFO",
		Message: "server starting",
		Service: "api-server",
		Route:   fmt.Sprintf(":%s", port),
	})

	log.Fatal(http.ListenAndServe(":"+port, mux))
}