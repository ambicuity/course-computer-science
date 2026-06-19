#include <stdio.h>

const char *target(int x, int y) {
    if (x > 3) {
        if (y == x + 1) return "path_A";
        if (y < 0) return "path_B";
    } else {
        if (x + y == 0) return "path_C";
    }
    return "path_D";
}

int main(void) {
    printf("%s\n", target(4, 5));
    printf("%s\n", target(4, -1));
    printf("%s\n", target(2, -2));
    printf("%s\n", target(2, 2));
    return 0;
}
