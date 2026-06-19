// Matching — Bipartite, Hopcroft-Karp, Hungarian
// Phase 04 — Algorithms & Complexity Analysis, Lesson 18
//
// C++ Hopcroft-Karp for large bipartite graphs.

#include <iostream>
#include <vector>
#include <queue>
#include <random>
#include <algorithm>
#include <climits>
#include <utility>
using namespace std;

class HopcroftKarp {
    int n_left, n_right;
    vector<vector<int>> adj; // adj[u] = list of right-vertices (0-indexed into right side)
    vector<int> pair_u, pair_v, dist;

public:
    // nL = number of left vertices, nR = number of right vertices
    HopcroftKarp(int nL, int nR)
        : n_left(nL), n_right(nR), adj(nL), pair_u(nL, -1), pair_v(nR, -1), dist(nL) {}

    void add_edge(int u, int v) { adj[u].push_back(v); }

    bool bfs() {
        queue<int> q;
        for (int u = 0; u < n_left; u++) {
            if (pair_u[u] == -1) {
                dist[u] = 0;
                q.push(u);
            } else {
                dist[u] = INT_MAX;
            }
        }
        bool found = false;
        while (!q.empty()) {
            int u = q.front(); q.pop();
            for (int v : adj[u]) {
                int pu = pair_v[v];
                if (pu == -1) {
                    found = true;
                } else if (dist[pu] == INT_MAX) {
                    dist[pu] = dist[u] + 1;
                    q.push(pu);
                }
            }
        }
        return found;
    }

    bool dfs(int u) {
        for (int v : adj[u]) {
            int pu = pair_v[v];
            if (pu == -1 || (dist[pu] == dist[u] + 1 && dfs(pu))) {
                pair_u[u] = v;
                pair_v[v] = u;
                return true;
            }
        }
        dist[u] = INT_MAX;
        return false;
    }

    int max_matching() {
        int matching = 0;
        while (bfs()) {
            for (int u = 0; u < n_left; u++) {
                if (pair_u[u] == -1 && dfs(u)) {
                    matching++;
                }
            }
        }
        return matching;
    }

    // Returns list of (left_vertex, right_vertex) pairs
    vector<pair<int,int>> get_matching() const {
        vector<pair<int,int>> result;
        for (int u = 0; u < n_left; u++) {
            if (pair_u[u] != -1) {
                result.emplace_back(u, pair_u[u]);
            }
        }
        return result;
    }
};

int main() {
    ios::sync_with_stdio(false);
    cin.tie(nullptr);

    // Example bipartite graph:
    //   Left:  {0, 1, 2, 3}  (A, B, C, D)
    //   Right: {0, 1, 2}     (1, 2, 3)
    //   A(0)->1,2    B(1)->1,3    C(2)->2    D(3)->2,3

    int nL = 4, nR = 3;
    HopcroftKarp hk(nL, nR);

    hk.add_edge(0, 0); hk.add_edge(0, 1);  // A -> 1, 2
    hk.add_edge(1, 0); hk.add_edge(1, 2);  // B -> 1, 3
    hk.add_edge(2, 1);                       // C -> 2
    hk.add_edge(3, 1); hk.add_edge(3, 2);  // D -> 2, 3

    int sz = hk.max_matching();
    auto matching = hk.get_matching();

    cout << "Hopcroft-Karp maximum matching: " << sz << "\n";
    cout << "Pairs (left -> right):\n";
    for (auto [u, v] : matching) {
        cout << "  " << char('A' + u) << " -> " << (v + 1) << "\n";
    }

    // Stress test: random bipartite graph with larger sizes
    mt19937 rng(42);
    int bigL = 500, bigR = 500;
    HopcroftKarp hk2(bigL, bigR);
    double edge_prob = 0.1;
    for (int u = 0; u < bigL; u++) {
        for (int v = 0; v < bigR; v++) {
            if (uniform_real_distribution<>(0, 1)(rng) < edge_prob) {
                hk2.add_edge(u, v);
            }
        }
    }
    int big_size = hk2.max_matching();
    cout << "\nRandom graph (" << bigL << "x" << bigR << ", p=" << edge_prob << "): ";
    cout << "matching size = " << big_size << "\n";

    return 0;
}
