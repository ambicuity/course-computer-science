#include <stdio.h>
#include <stdlib.h>
#include "greet.h"

void greet(const char *who) {
    const char *prefix = getenv("GREET_PREFIX");
    if (prefix) {
        printf("%s, %s\n", prefix, who);
    } else {
        printf("hello, %s\n", who);
    }
}
