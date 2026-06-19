/* struct_layout.c — print field offsets and inferred padding for any struct.
 *
 * Build:  gcc -DSTRUCT_LAYOUT_DEMO struct_layout.c -o struct_layout_demo
 * Run:    ./struct_layout_demo
 *
 * Macro usage:
 *   PRINT_FIELD(StructName, fieldName)
 */

#include <stdio.h>
#include <stddef.h>
#include <stdint.h>

#define PRINT_FIELD(S, F) \
    printf("  %-25s  offset=%-3zu  size=%zu\n", \
           "." #F, offsetof(struct S, F), sizeof(((struct S *)0)->F))

#define PRINT_STRUCT(S) \
    printf("struct %s — sizeof = %zu\n", #S, sizeof(struct S))

#ifdef STRUCT_LAYOUT_DEMO

/* Example struct with a deliberate poor layout */
struct Pixel {
    uint8_t   r;
    uint32_t  alpha;
    uint8_t   g;
    uint8_t   b;
    double    weight;
};

/* Reordered to minimize padding */
struct PixelTight {
    double    weight;     /* 8 bytes, alignment 8 */
    uint32_t  alpha;      /* 4 bytes, alignment 4 */
    uint8_t   r;
    uint8_t   g;
    uint8_t   b;
    /* 1 byte tail padding to round to multiple of 8 */
};

int main(void) {
    PRINT_STRUCT(Pixel);
    PRINT_FIELD(Pixel, r);
    PRINT_FIELD(Pixel, alpha);
    PRINT_FIELD(Pixel, g);
    PRINT_FIELD(Pixel, b);
    PRINT_FIELD(Pixel, weight);

    printf("\n");
    PRINT_STRUCT(PixelTight);
    PRINT_FIELD(PixelTight, weight);
    PRINT_FIELD(PixelTight, alpha);
    PRINT_FIELD(PixelTight, r);
    PRINT_FIELD(PixelTight, g);
    PRINT_FIELD(PixelTight, b);

    printf("\nSavings: %zu bytes per pixel × millions = MBs of memory.\n",
           sizeof(struct Pixel) - sizeof(struct PixelTight));
    return 0;
}
#endif
