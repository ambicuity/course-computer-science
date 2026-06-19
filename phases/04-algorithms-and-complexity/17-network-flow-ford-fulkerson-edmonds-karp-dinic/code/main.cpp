// Network Flow — Ford-Fulkerson, Edmonds-Karp, Dinic
// Phase 04 — Algorithms & Complexity Analysis
//
// Dinic's algorithm with adjacency-list edge struct for performance on
// larger graphs.

#include <algorithm>
#include <iostream>
#include <limits>
#include <queue>
#include <vector>

struct Edge {
    int to;
    int capacity;
    int rev;  // index of reverse edge in adj[to]
};

class Dinic {
public:
    explicit Dinic(int n) : n_(n), adj_(n), level_(n), ptr_(n) {}

    void add_edge(int u, int v, int cap) {
        Edge fwd{v, cap, static_cast<int>(adj_[v].size())};
        Edge rev{u, 0, static_cast<int>(adj_[u].size())};
        adj_[u].push_back(fwd);
        adj_[v].push_back(rev);
    }

    int max_flow(int s, int t) {
        int flow = 0;
        while (bfs(s, t)) {
            std::fill(ptr_.begin(), ptr_.end(), 0);
            while (int pushed = dfs(s, t, INF)) {
                flow += pushed;
            }
        }
        return flow;
    }

    // After max_flow, find reachable nodes from s in the residual graph.
    std::vector<bool> min_cut_side(int s) const {
        std::vector<bool> visited(n_, false);
        std::queue<int> q;
        q.push(s);
        visited[s] = true;
        while (!q.empty()) {
            int u = q.front();
            q.pop();
            for (const auto& e : adj_[u]) {
                if (!visited[e.to] && e.capacity > 0) {
                    visited[e.to] = true;
                    q.push(e.to);
                }
            }
        }
        return visited;
    }

private:
    static constexpr int INF = std::numeric_limits<int>::max() / 2;

    bool bfs(int s, int t) {
        std::fill(level_.begin(), level_.end(), -1);
        level_[s] = 0;
        std::queue<int> q;
        q.push(s);
        while (!q.empty()) {
            int u = q.front();
            q.pop();
            for (const auto& e : adj_[u]) {
                if (level_[e.to] == -1 && e.capacity > 0) {
                    level_[e.to] = level_[u] + 1;
                    q.push(e.to);
                }
            }
        }
        return level_[t] != -1;
    }

    int dfs(int u, int t, int pushed) {
        if (u == t) return pushed;
        for (int& cid = ptr_[u]; cid < static_cast<int>(adj_[u].size()); ++cid) {
            auto& e = adj_[u][cid];
            if (level_[e.to] == level_[u] + 1 && e.capacity > 0) {
                int bottleneck = dfs(e.to, t, std::min(pushed, e.capacity));
                if (bottleneck > 0) {
                    e.capacity -= bottleneck;
                    adj_[e.to][e.rev].capacity += bottleneck;
                    return bottleneck;
                }
            }
        }
        return 0;
    }

    int n_;
    std::vector<std::vector<Edge>> adj_;
    std::vector<int> level_;
    std::vector<int> ptr_;
};

int main() {
    // Worked example:  s=0, a=1, b=2, t=3
    //
    //       10
    //  0 ──────► 1 ──────► 3
    //  │  4       8       │
    //  │         ▲        │
    //  └────► 2 ─┘        │
    //     9      6        │
    //     └─────►─────────┘
    //              10
    {
        std::cout << "=== Pipeline network (worked example) ===" << std::endl;
        Dinic din(4);
        din.add_edge(0, 1, 10);  // s -> a
        din.add_edge(0, 2, 9);   // s -> b
        din.add_edge(1, 3, 8);   // a -> t
        din.add_edge(2, 1, 6);   // b -> a
        din.add_edge(2, 3, 10);  // b -> t

        int flow = din.max_flow(0, 3);
        std::cout << "  Max flow: " << flow << std::endl;

        auto side = din.min_cut_side(0);
        std::cout << "  Min-cut S-side: ";
        for (int i = 0; i < 4; ++i) {
            if (side[i]) std::cout << i << " ";
        }
        std::cout << std::endl;
    }

    // Bipartite matching: 4 workers, 5 jobs
    // source=0, workers=1..4, jobs=5..9, sink=10
    {
        std::cout << "\n=== Bipartite matching (4 workers, 5 jobs) ===" << std::endl;
        int left = 4, right = 5;
        int src = 0, snk = left + right + 1;
        Dinic din(snk + 1);

        for (int l = 0; l < left; ++l)
            din.add_edge(src, 1 + l, 1);
        for (int r = 0; r < right; ++r)
            din.add_edge(1 + left + r, snk, 1);

        // Edges: (worker, job)
        int edges[][2] = {
            {0, 0}, {0, 1},
            {1, 1}, {1, 2},
            {2, 0}, {2, 2}, {2, 3},
            {3, 3}, {3, 4}
        };
        for (auto& e : edges)
            din.add_edge(1 + e[0], 1 + left + e[1], 1);

        int matching = din.max_flow(src, snk);
        std::cout << "  Maximum matching size: " << matching << std::endl;
    }

    // Larger random-ish graph to show performance
    {
        std::cout << "\n=== Stress test: 100 nodes, 500 edges ===" << std::endl;
        int V = 100;
        Dinic din(V);
        // Deterministic "random" edges
        for (int i = 0; i < 500; ++i) {
            int u = (i * 7 + 3) % V;
            int v = (i * 11 + 5) % V;
            if (u != v) {
                int cap = (i * 13 % 50) + 1;
                din.add_edge(u, v, cap);
            }
        }
        int flow = din.max_flow(0, V - 1);
        std::cout << "  Max flow: " << flow << std::endl;
    }

    return 0;
}
