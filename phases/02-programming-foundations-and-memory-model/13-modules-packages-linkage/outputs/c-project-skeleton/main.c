/* main.c — consumer of the calc module. */
#include <stdio.h>
#include "calc.h"

int main(void) {
    printf("add(3, 4) = %d\n", add(3, 4));
    printf("sub(10, 7) = %d\n", sub(10, 7));
    printf("calls so far = %d\n", read_counter());
    return 0;
}
