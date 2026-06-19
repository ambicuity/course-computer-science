/* main.c — struct layout, alignment, padding, unions, bitfields. */
#include <stdio.h>
#include <stdint.h>
#include <stddef.h>

struct A {
    char a;
    int  b;
    char c;
};

struct B {
    int  b;       /* reordered: largest alignment first */
    char a;
    char c;
};

struct __attribute__((packed)) P {
    char a;
    int  b;
    char c;
};

struct flags {
    unsigned int active   : 1;
    unsigned int priority : 3;
    unsigned int channel  : 4;
    unsigned int reserved : 24;
};

union FloatBits {
    float    f;
    uint32_t u;
};

int main(void) {
    printf("== struct A (poor field order) ==\n");
    printf("  sizeof(struct A) = %zu  (= 12, due to padding)\n", sizeof(struct A));
    printf("  offsetof(A, a) = %zu\n", offsetof(struct A, a));
    printf("  offsetof(A, b) = %zu  (3 bytes of padding after a)\n", offsetof(struct A, b));
    printf("  offsetof(A, c) = %zu\n", offsetof(struct A, c));

    printf("\n== struct B (reordered, largest alignment first) ==\n");
    printf("  sizeof(struct B) = %zu  (= 8, 4 bytes saved)\n", sizeof(struct B));
    printf("  offsetof(B, b) = %zu\n", offsetof(struct B, b));
    printf("  offsetof(B, a) = %zu\n", offsetof(struct B, a));
    printf("  offsetof(B, c) = %zu\n", offsetof(struct B, c));

    printf("\n== struct P (packed) ==\n");
    printf("  sizeof(struct P) = %zu  (= 6, no padding; b may be misaligned)\n",
           sizeof(struct P));
    struct P p;
    printf("  &p.a=%p, &p.b=%p, &p.c=%p\n", (void*)&p.a, (void*)&p.b, (void*)&p.c);

    printf("\n== Union: float ↔ uint32_t (lawful in C99+) ==\n");
    union FloatBits fb;
    fb.f = 1.0f;
    printf("  1.0f as IEEE 754 bits = 0x%08x  (expected 0x3f800000)\n", fb.u);
    fb.f = -2.0f;
    printf("  -2.0f as IEEE 754 bits = 0x%08x\n", fb.u);

    printf("\n== Bitfield: pack 8 bits into 1 byte's worth of storage ==\n");
    struct flags fl = {0};
    fl.active = 1;
    fl.priority = 5;
    fl.channel = 9;
    printf("  sizeof(struct flags) = %zu\n", sizeof(struct flags));
    printf("  flags: active=%u, priority=%u, channel=%u\n",
           fl.active, fl.priority, fl.channel);

    printf("\n== Alignment requirements ==\n");
    printf("  _Alignof(char)   = %zu\n", _Alignof(char));
    printf("  _Alignof(short)  = %zu\n", _Alignof(short));
    printf("  _Alignof(int)    = %zu\n", _Alignof(int));
    printf("  _Alignof(double) = %zu\n", _Alignof(double));
    printf("  _Alignof(void*)  = %zu\n", _Alignof(void*));

    return 0;
}
