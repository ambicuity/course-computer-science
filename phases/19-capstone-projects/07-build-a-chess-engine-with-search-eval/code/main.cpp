#include <iostream>
#include <vector>

int eval_material(const std::vector<int>& pieces) {
    int score = 0;
    for (int p : pieces) score += p;
    return score;
}

int main() {
    // Toy material vector: positive for side-to-move, negative for opponent.
    std::vector<int> pos = {1, 3, 3, 5, 9, -1, -3, -9};
    std::cout << "eval=" << eval_material(pos) << "\n";
    return 0;
}
