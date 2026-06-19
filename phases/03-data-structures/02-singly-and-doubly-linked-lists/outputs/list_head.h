/*
 * list_head.h — Linux-style intrusive doubly-linked list.
 *
 * Embed `struct list_head member;` in your struct, then add the struct via
 * its member to any number of lists. Recover the user struct from the
 * list_head pointer with container_of / list_entry.
 *
 * Example:
 *
 *     struct task { int pid; struct list_head tasks; };
 *
 *     struct list_head all = LIST_HEAD_INIT(all);
 *     struct task t = { .pid = 100 };
 *     list_add_tail(&t.tasks, &all);
 *
 *     struct list_head *pos;
 *     list_for_each(pos, &all) {
 *         struct task *p = list_entry(pos, struct task, tasks);
 *         printf("pid=%d\n", p->pid);
 *     }
 */
#ifndef LIST_HEAD_H
#define LIST_HEAD_H

#include <stddef.h>

struct list_head { struct list_head *prev, *next; };

#define LIST_HEAD_INIT(name) { &(name), &(name) }
#define LIST_HEAD(name) struct list_head name = LIST_HEAD_INIT(name)
#define INIT_LIST_HEAD(h) do { (h)->prev = (h); (h)->next = (h); } while (0)

static inline int list_empty(const struct list_head *h) { return h->next == h; }

static inline void list_add(struct list_head *node, struct list_head *head) {
    node->next = head->next;
    node->prev = head;
    head->next->prev = node;
    head->next = node;
}

static inline void list_add_tail(struct list_head *node, struct list_head *head) {
    node->prev = head->prev;
    node->next = head;
    head->prev->next = node;
    head->prev = node;
}

static inline void list_del(struct list_head *node) {
    node->prev->next = node->next;
    node->next->prev = node->prev;
    node->prev = node->next = NULL;
}

#define list_for_each(pos, head) \
    for (pos = (head)->next; pos != (head); pos = pos->next)

#define list_for_each_safe(pos, n, head) \
    for (pos = (head)->next, n = pos->next; pos != (head); pos = n, n = pos->next)

#ifndef container_of
#define container_of(ptr, type, member) \
    ((type *)((char *)(ptr) - offsetof(type, member)))
#endif
#define list_entry(ptr, type, member) container_of(ptr, type, member)

#endif /* LIST_HEAD_H */
