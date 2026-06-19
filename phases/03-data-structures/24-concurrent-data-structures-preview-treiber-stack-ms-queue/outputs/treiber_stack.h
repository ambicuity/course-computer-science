/*
 * treiber_stack.h — single-header Treiber lock-free stack.
 *
 *   Treiber t = {0};
 *   atomic_init(&t.head, NULL);
 *   treiber_push(&t, value);
 *   int v; if (treiber_pop(&t, &v)) ...
 *
 * IMPORTANT: This stack LEAKS popped nodes. It demonstrates the algorithm,
 * not safe memory reclamation. For production: use hazard pointers
 * (Michael 2004) or epoch-based GC (crossbeam-rs).
 */
#ifndef TREIBER_STACK_H
#define TREIBER_STACK_H

#include <stdatomic.h>
#include <stdlib.h>
#include <stdbool.h>

typedef struct TreiberNode { int value; struct TreiberNode *next; } TreiberNode;
typedef struct { _Atomic(TreiberNode *) head; } Treiber;

static inline void treiber_push(Treiber *s, int v) {
    TreiberNode *n = (TreiberNode *)malloc(sizeof(*n));
    n->value = v;
    TreiberNode *old_head = atomic_load_explicit(&s->head, memory_order_relaxed);
    do {
        n->next = old_head;
    } while (!atomic_compare_exchange_weak_explicit(
        &s->head, &old_head, n,
        memory_order_release, memory_order_relaxed));
}

static inline bool treiber_pop(Treiber *s, int *out) {
    TreiberNode *old_head = atomic_load_explicit(&s->head, memory_order_acquire);
    while (old_head) {
        TreiberNode *next = old_head->next;
        if (atomic_compare_exchange_weak_explicit(
                &s->head, &old_head, next,
                memory_order_acq_rel, memory_order_acquire)) {
            *out = old_head->value;
            /* LEAK by design — see header comment */
            return true;
        }
    }
    return false;
}

#endif /* TREIBER_STACK_H */
