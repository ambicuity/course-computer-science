// Backtracking, Branch & Bound
// Phase 04 — Algorithms & Complexity Analysis
// N-Queens (bitmask) and Sudoku solver for performance comparison.

#include <iostream>
#include <vector>
#include <array>
#include <chrono>
#include <string>
#include <cstdint>
#include <functional>

// ---------------------------------------------------------------------------
// N-Queens — Bitmask
// ---------------------------------------------------------------------------

struct NQueensResult {
    int solutions;
    int nodes_explored;
};

NQueensResult nqueens_bitmask(int n) {
    NQueensResult res{0, 0};
    uint32_t all_mask = (n == 32) ? 0xFFFFFFFFu : ((1u << n) - 1);

    std::function<void(int, uint32_t, uint32_t, uint32_t)> bt =
        [&](int row, uint32_t cols, uint32_t diag1, uint32_t diag2) {
        res.nodes_explored++;
        if (row == n) {
            res.solutions++;
            return;
        }
        uint32_t available = all_mask & ~(cols | diag1 | diag2);
        while (available) {
            uint32_t bit = available & (-available);
            available ^= bit;
            bt(row + 1, cols | bit, (diag1 | bit) << 1, (diag2 | bit) >> 1);
        }
    };

    bt(0, 0, 0, 0);
    return res;
}

// ---------------------------------------------------------------------------
// Sudoku Solver
// ---------------------------------------------------------------------------

bool solve_sudoku(std::array<std::array<int, 9>, 9>& board) {
    std::array<uint16_t, 9> rows{}, cols{}, boxes{};
    std::vector<std::pair<int, int>> empty;

    for (int r = 0; r < 9; r++) {
        for (int c = 0; c < 9; c++) {
            int v = board[r][c];
            if (v) {
                uint16_t mask = 1 << (v - 1);
                rows[r] |= mask;
                cols[c] |= mask;
                boxes[(r / 3) * 3 + c / 3] |= mask;
            } else {
                empty.push_back({r, c});
            }
        }
    }

    auto bt = [&](auto& self, int idx) -> bool {
        if (idx == (int)empty.size()) return true;
        auto [r, c] = empty[idx];
        int box = (r / 3) * 3 + c / 3;
        uint16_t used = rows[r] | cols[c] | boxes[box];
        for (int d = 0; d < 9; d++) {
            if (!(used & (1 << d))) {
                uint16_t mask = 1 << d;
                board[r][c] = d + 1;
                rows[r] |= mask; cols[c] |= mask; boxes[box] |= mask;
                if (self(self, idx + 1)) return true;
                board[r][c] = 0;
                rows[r] &= ~mask; cols[c] &= ~mask; boxes[box] &= ~mask;
            }
        }
        return false;
    };

    return bt(bt, 0);
}

void print_board(const std::array<std::array<int, 9>, 9>& board) {
    for (int r = 0; r < 9; r++) {
        std::cout << "  ";
        for (int c = 0; c < 9; c++) {
            std::cout << board[r][c] << " ";
            if (c % 3 == 2 && c < 8) std::cout << "| ";
        }
        std::cout << "\n";
        if (r % 3 == 2 && r < 8) std::cout << "  ------+-------+------\n";
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

int main() {
    std::cout << "=== Backtracking, Branch & Bound (C++) ===\n\n";

    // --- N-Queens benchmark ---
    std::cout << "--- N-Queens (bitmask) ---\n";
    for (int n : {8, 12, 15}) {
        auto t0 = std::chrono::high_resolution_clock::now();
        auto res = nqueens_bitmask(n);
        auto t1 = std::chrono::high_resolution_clock::now();
        double ms = std::chrono::duration<double, std::milli>(t1 - t0).count();
        std::cout << "  n=" << n << ": " << res.solutions << " solutions, "
                  << res.nodes_explored << " nodes, "
                  << ms << " ms\n";
    }
    std::cout << "\n";

    // --- Sudoku ---
    std::cout << "--- Sudoku Solver ---\n";
    std::array<std::array<int, 9>, 9> puzzle = {{
        {5, 3, 0, 0, 7, 0, 0, 0, 0},
        {6, 0, 0, 1, 9, 5, 0, 0, 0},
        {0, 9, 8, 0, 0, 0, 0, 6, 0},
        {8, 0, 0, 0, 6, 0, 0, 0, 3},
        {4, 0, 0, 8, 0, 3, 0, 0, 1},
        {7, 0, 0, 0, 2, 0, 0, 0, 6},
        {0, 6, 0, 0, 0, 0, 2, 8, 0},
        {0, 0, 0, 4, 1, 9, 0, 0, 5},
        {0, 0, 0, 0, 8, 0, 0, 7, 9},
    }};

    auto t0 = std::chrono::high_resolution_clock::now();
    bool ok = solve_sudoku(puzzle);
    auto t1 = std::chrono::high_resolution_clock::now();
    double ms = std::chrono::duration<double, std::milli>(t1 - t0).count();

    if (ok) {
        std::cout << "  Solved in " << ms << " ms\n";
        print_board(puzzle);
    } else {
        std::cout << "  No solution found.\n";
    }
    std::cout << "\n";

    return 0;
}
