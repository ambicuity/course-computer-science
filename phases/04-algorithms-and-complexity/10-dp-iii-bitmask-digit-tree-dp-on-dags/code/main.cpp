// DP III — Bitmask DP (TSP) — C++ mirror
// Phase 04 — Algorithms & Complexity Analysis
//
// C++ is the natural fit for bitmask DP because n=20 fits in a 32-bit int
// and vector indexing is fast.  The Python version works but is ~50× slower
// for n=20.

#include <algorithm>
#include <climits>
#include <iostream>
#include <vector>
using namespace std;

// ---------------------------------------------------------------------------
// TSP with Bitmask DP  —  O(2^n · n^2)
// ---------------------------------------------------------------------------
int tspBitmask(const vector<vector<int>>& dist, int n) {
    int FULL = (1 << n) - 1;
    vector<vector<int>> dp(1 << n, vector<int>(n, INT_MAX));
    dp[1][0] = 0;  // start at city 0

    for (int S = 1; S < (1 << n); S++) {
        for (int u = 0; u < n; u++) {
            if (!(S >> u & 1) || dp[S][u] == INT_MAX) continue;
            for (int v = 0; v < n; v++) {
                if (S >> v & 1) continue;
                int ns = S | (1 << v);
                int cost = dp[S][u] + dist[u][v];
                if (cost < dp[ns][v]) dp[ns][v] = cost;
            }
        }
    }

    int ans = INT_MAX;
    for (int u = 0; u < n; u++) {
        if (dp[FULL][u] != INT_MAX)
            ans = min(ans, dp[FULL][u] + dist[u][0]);
    }
    return ans;
}

// ---------------------------------------------------------------------------
// Count Hamiltonian paths starting at vertex 0 — bitmask DP
// ---------------------------------------------------------------------------
long long hamiltonianPathCount(const vector<vector<int>>& adj, int n) {
    // adj[i][j] = 1 if edge exists
    int FULL = (1 << n) - 1;
    vector<vector<long long>> dp(1 << n, vector<long long>(n, 0));
    dp[1][0] = 1;

    for (int S = 1; S < (1 << n); S++) {
        for (int u = 0; u < n; u++) {
            if (!(S >> u & 1) || dp[S][u] == 0) continue;
            for (int v = 0; v < n; v++) {
                if (S >> v & 1 || !adj[u][v]) continue;
                int ns = S | (1 << v);
                dp[ns][v] += dp[S][u];
            }
        }
    }

    long long total = 0;
    for (int u = 0; u < n; u++)
        total += dp[FULL][u];
    return total;
}

// ---------------------------------------------------------------------------
int main() {
    // TSP demo (same 4-city instance as Python)
    vector<vector<int>> dist = {
        {0, 10, 15, 20},
        {10,  0, 35, 25},
        {15, 35,  0, 30},
        {20, 25, 30,  0},
    };
    cout << "TSP min tour (4 cities): " << tspBitmask(dist, 4) << "\n";  // 80

    // Hamiltonian path count on complete graph K4
    vector<vector<int>> adj = {
        {0, 1, 1, 1},
        {1, 0, 1, 1},
        {1, 1, 0, 1},
        {1, 1, 1, 0},
    };
    cout << "Hamiltonian paths from 0 (K4): " << hamiltonianPathCount(adj, 4) << "\n";  // 6

    return 0;
}
