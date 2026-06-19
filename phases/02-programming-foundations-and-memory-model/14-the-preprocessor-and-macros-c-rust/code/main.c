/* main.c — C preprocessor: macros, conditional compilation, X-macros. */
#include <stdio.h>

/* Three flavors of square macro. */
#define SQ_BAD(x)  x * x
#define SQ_OK(x)   ((x) * (x))
#define SQ_SAFE(x) ({ int _t = (x); _t * _t; })

/* Stringify and paste */
#define STR(x)        #x
#define CONCAT(a, b)  a ## b

/* X-macro for colors */
#define COLORS \
    X(RED,   "Red",   0xFF0000) \
    X(GREEN, "Green", 0x00FF00) \
    X(BLUE,  "Blue",  0x0000FF)

typedef enum {
#define X(name, label, rgb) COLOR_##name,
    COLORS
#undef X
    COLOR_COUNT
} Color;

const char *color_name(Color c) {
    switch (c) {
#define X(name, label, rgb) case COLOR_##name: return label;
        COLORS
#undef X
        default: return "?";
    }
}

unsigned color_rgb(Color c) {
    switch (c) {
#define X(name, label, rgb) case COLOR_##name: return rgb;
        COLORS
#undef X
        default: return 0;
    }
}

int main(void) {
    printf("== Macro footguns ==\n");
    printf("  SQ_BAD(3 + 4)  = %d   (wrong; expanded as 3 + 4 * 3 + 4)\n", SQ_BAD(3 + 4));
    printf("  SQ_OK(3 + 4)   = %d   (correct; parens around args)\n",    SQ_OK(3 + 4));
    printf("  SQ_SAFE(3 + 4) = %d   (safe via statement expression)\n",  SQ_SAFE(3 + 4));

    printf("\n== Multiple evaluation ==\n");
    int i = 0;
    SQ_OK(++i);
    printf("  After SQ_OK(++i): i = %d   (multiple-evaluation bug)\n", i);
    i = 0;
    SQ_SAFE(++i);
    printf("  After SQ_SAFE(++i): i = %d   (safe)\n", i);

    printf("\n== Stringify & paste ==\n");
    printf("  STR(hello)              = \"%s\"\n", STR(hello));
    int CONCAT(foo, bar) = 42;
    printf("  CONCAT(foo, bar) defines foobar = %d\n", foobar);

    printf("\n== X-macro: enum + name table + rgb table ==\n");
    for (Color c = 0; c < COLOR_COUNT; ++c) {
        printf("  %d: name=%-6s rgb=0x%06x\n", c, color_name(c), color_rgb(c));
    }

    printf("\n== Conditional compilation ==\n");
#ifdef DEBUG
    printf("  DEBUG build: extra checks active\n");
#else
    printf("  RELEASE build (define DEBUG to switch)\n");
#endif

    return 0;
}
