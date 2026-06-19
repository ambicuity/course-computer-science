# CDNs and Anycast

> How Netflix serves video to 200 million subscribers from servers they've never heard of — and why a single IP address can exist in 300 cities simultaneously.

**Type:** Learn (Reference/Reading)
**Languages:** Markdown
**Prerequisites:** Phase 09 lessons 01–16
**Time:** ~45 minutes

## Learning Objectives

- Explain how a CDN caches content at edge locations to reduce latency.
- Describe how Anycast routing works via BGP to direct clients to the nearest server.
- Compare cache invalidation strategies: TTL, purge APIs, stale-while-revalidate.
- Identify when to use a CDN and when to bypass it.

## The Problem

Your origin server is in Virginia. A user in Tokyo requests your 2 MB hero image. The round trip: Tokyo → Virginia → Tokyo = ~240ms RTT, and the image transfers at whatever the transpacific bottleneck allows. For a 50 Mbps link that's ~320ms of transfer time. Total: ~560ms.

Now serve that same image from a Tokyo edge node. RTT drops to ~5ms. Transfer completes in ~32ms. Total: ~37ms — **15× faster**. Multiply this by millions of users and thousands of assets and you understand why CDNs exist.

## The Concept

### What Is a CDN?

A **Content Delivery Network** is a geographically distributed network of proxy servers (edge nodes / Points of Presence) that cache content close to users. The CDN sits between your users and your origin server:

```
User (Tokyo)
  |
  | Request cached at Tokyo PoP
  v
CDN Edge Node (Tokyo) ← cache hit? serve immediately
  |
  | Cache miss → fetch from origin
  v
CDN Regional Node (Singapore)
  |
  | Still miss → fetch from origin
  v
Origin Server (Virginia)
```

### How CDN Routing Works

CDNs route users to the nearest edge node using two primary mechanisms:

**1. DNS-based load balancing**: The CDN's authoritative DNS returns different IPs based on the client's location.

```
User in Tokyo queries cdn.example.com
  → CDN DNS sees source IP is in Asia
  → Returns 203.0.113.50 (Tokyo edge node)

User in London queries cdn.example.com
  → CDN DNS sees source IP is in Europe
  → Returns 198.51.100.20 (London edge node)
```

**2. Anycast**: The same IP address is announced from multiple locations via BGP. The Internet's routing infrastructure automatically directs each client to the topologically nearest instance.

### Anycast

Anycast is a network addressing and routing methodology where a single IP address is assigned to multiple servers in different locations. BGP (Border Gateway Protocol) advertises the same prefix from each location, and routers select the shortest AS-path:

```
                    ┌─── 198.51.100.1 (Tokyo)  ───┐
                    │                               │
Client ──→ Internet─┼─── 198.51.100.1 (London) ───┤──→ BGP picks nearest
                    │                               │
                    └─── 198.51.100.1 (São Paulo)──┘
```

Key properties:
- **One IP, many servers**: All instances share the same IP. BGP decides which one gets the packet.
- **Automatic failover**: If Tokyo goes down, BGP withdraws the route, and traffic shifts to the next-closest node within seconds.
- **DDoS absorption**: Attack traffic is distributed across all Anycast locations, diluting volumetric attacks.

**Used by**: DNS root servers (13 root letters, all Anycasted), Cloudflare's edge network (~300 cities), AWS Global Accelerator.

### Cache Hierarchy

CDNs organize caches in tiers:

```
Edge cache (PoP)    → 1–5ms from user, serves 80–95% of requests
  ↓ miss
Regional cache      → 20–50ms, larger storage, covers a continent
  ↓ miss
Origin shield      → Single point in front of origin, protects from thundering herd
  ↓ miss
Origin server      → Your actual application server
```

**Cache hit ratio** is the critical metric. A 95% hit ratio means 19 out of 20 requests never touch your origin. At 99%, your origin handles 1% of the traffic it would without a CDN.

### Cache Invalidation

Cached content eventually goes stale. Four strategies:

| Strategy | How it works | Trade-off |
|----------|-------------|-----------|
| **TTL** | Object expires after N seconds. Revalidate on next request. | Simple, but serves stale content until next request. |
| **Purge API** | Origin sends explicit invalidation to CDN. | Immediate, but requires infrastructure. |
| **Stale-while-revalidate** | Serve stale content immediately, fetch fresh copy in background. | Zero latency penalty, brief window of stale content. |
| **Cache busting** | Append version hash to URL: `style.v2.css`. | Guarantees freshness, but old copies linger in cache. |

Production systems combine all four. Example: CSS/JS use cache busting + long TTL (1 year). API responses use short TTL + stale-while-revalidate + purge on deploy.

### CDN Providers and Architecture

| Provider | Edge locations | Protocol | Notable feature |
|----------|---------------|----------|----------------|
| **Cloudflare** | 300+ cities | HTTP/3, Anycast | Workers (edge compute), free tier |
| **Akamai** | 4,200+ locations | DNS-based | Largest network, media delivery |
| **AWS CloudFront** | 600+ PoPs | DNS + Anycast | Lambda@Edge, S3 integration |
| **Fastly** | ~90 PoPs | Anycast | Instant purge (<150ms), real-time logs |
| **Google Cloud CDN** | 200+ PoPs | Anycast | Global load balancer integration |

### Anycast vs Unicast

```
Unicast:  Each server has a unique IP. DNS maps users to the "best" IP.
          DNS TTL delays failover. Load balancer adds a hop.

Anycast:  All servers share one IP. BGP picks the nearest.
          Failover in seconds (BGP withdrawal). No extra hop.
```

Anycast excels at stateless workloads (DNS, static content, DDoS mitigation). Stateful workloads (WebSocket, long-poll) prefer Unicast with sticky sessions.

### When to Use a CDN

**Use a CDN for**:
- Static assets: images, CSS, JavaScript, fonts
- Video streaming: HLS/DASH segments
- Download distribution: software updates, game patches
- API acceleration: cache GET responses, reduce origin load
- DDoS protection: absorb volumetric attacks at the edge

**Bypass a CDN for**:
- Real-time APIs that must not be cached (POST/PUT/DELETE)
- Data that varies per user and has no reuse (personal dashboards)
- Very low-latency requirements where even edge cache overhead matters
- Compliance: data sovereignty laws may prohibit cross-border caching

### Use It

**Static assets at scale**:
```nginx
# Nginx cache configuration (simplified)
proxy_cache_path /var/cache/nginx levels=1:2 keys_zone=cdn:10m;

location /static/ {
    proxy_cache cdn;
    proxy_cache_valid 200 1h;
    proxy_cache_valid 404 1m;
    add_header X-Cache-Status $upstream_cache_status;
}
```

**Cache-Control headers** that drive CDN behavior:
```
Cache-Control: public, max-age=31536000, immutable    # 1 year, never revalidate
Cache-Control: public, max-age=300, stale-while-revalidate=60   # 5min + 60s grace
Cache-Control: no-cache    # Always revalidate with origin
Cache-Control: no-store    # Never cache
```

**Cloudflare Workers example** (edge compute):
```javascript
addEventListener('fetch', event => {
  event.respondWith(handleRequest(event.request))
})

async function handleRequest(request) {
  const cache = caches.default
  let response = await cache.match(request)
  if (response) return response

  response = await fetch(request)
  event.waitUntil(cache.put(request, response.clone()))
  return response
}
```

### Performance Impact: Concrete Numbers

| Scenario | Without CDN | With CDN (edge hit) | Improvement |
|----------|------------|-------------------|-------------|
| 2 MB image, user 8000 km away | ~560ms | ~37ms | 15× |
| 200 KB API response | ~180ms | ~8ms | 22× |
| 1 GB video segment | ~120s | ~8s | 15× |
| DNS lookup | ~80ms | ~15ms | 5× |

*(Assumptions: 50 Mbps path, 240ms intercontinental RTT, 5ms local RTT.)*

## Read the Source

- `varnish-cache/bin/varnishd/cache/cache_hash.c` — Varnish hash lookup: the core of cache hit/miss logic.
- `nginx/src/http/ngx_http_upstream.c` — Nginx proxy cache: see `ngx_http_upstream_cache_get()` for cache key generation.
- Cloudflare's blog: [How Anycast works at Cloudflare](https://blog.cloudflare.com/cloudflares-architecture-eliminating-single-p/) — production Anycast design.

## Ship It

The reusable artifact produced by this lesson is knowledge — no code to ship. The mental model of CDN cache tiers + Anycast routing applies whenever you design a system that serves content to geographically distributed users.

## Exercises

1. **Easy** — A website has `Cache-Control: max-age=3600` on its homepage. A user loads the page, then a deploy happens 10 minutes later. What does the user see on their next visit within the hour? How would you fix this?

2. **Medium** — Design a cache invalidation strategy for an e-commerce site. Product images change rarely (use long TTL). Prices change frequently (use short TTL + stale-while-revalidate). Inventory counts change in real time (no cache). Map each URL pattern to the right Cache-Control headers.

3. **Hard** — Explain why Anycast works well for DNS (stateless, UDP, short-lived queries) but poorly for TCP connections that last minutes. What happens if a BGP route change moves an in-flight TCP connection to a different Anycast node mid-connection?

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| CDN | "Content delivery" | Geographically distributed cache network that serves content from edge nodes near users |
| PoP | "Point of presence" | A physical CDN node — servers in a data center that cache and serve content |
| Anycast | "One IP, many places" | Same IP address announced from multiple locations via BGP; routers pick nearest |
| Edge node | "Edge server" | A CDN's geographically proximate cache server, closest to the end user |
| Origin | "Your server" | The source-of-truth server that the CDN fetches from on cache miss |
| Cache hit ratio | "Hit rate" | Fraction of requests served from cache without contacting origin (target: 95%+) |
| Purge | "Invalidation" | Explicit request to remove a cached object from all CDN edge nodes |
| Stale-while-revalidate | "Background refresh" | Serve stale content immediately while fetching a fresh copy asynchronously |
| Cache busting | "Fingerprinting" | Append content hash to filename (e.g., `app.abc123.js`) to force re-download on change |
| Origin shield | "Shield cache" | Intermediate cache layer between edge and origin; reduces thundering herd on origin |

## Further Reading

- [Cloudflare Learning Center: What is a CDN?](https://www.cloudflare.com/learning/cdn/what-is-a-cdn/)
- RFC 7094 — Architectural Considerations of IP Anycast
- [Fastly: How caching works](https://www.fastly.com/blog/headers-we-dont-want) — Real-world Cache-Control header patterns
- [Netflix Open Connect](https://openconnect.netflix.com/) — How Netflix deploys custom CDN appliances inside ISPs
- [Akamai: State of the Internet](https://www.akamai.com/) — Annual report on CDN traffic patterns and edge computing trends
