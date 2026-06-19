/* main.c — character trie + radix trie. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>

/* ============================================================ */
/* Plain trie (lowercase ASCII a-z + apostrophe ignored)        */
/* ============================================================ */

typedef struct TNode {
    struct TNode *children[26];
    int           terminal;
} TNode;

static TNode *tnew(void) { return calloc(1, sizeof(TNode)); }

static void trie_insert(TNode *root, const char *word) {
    TNode *cur = root;
    for (const char *p = word; *p; ++p) {
        int c = *p - 'a';
        if (c < 0 || c >= 26) continue;
        if (!cur->children[c]) cur->children[c] = tnew();
        cur = cur->children[c];
    }
    cur->terminal = 1;
}

static int trie_contains(const TNode *root, const char *word) {
    const TNode *cur = root;
    for (const char *p = word; *p; ++p) {
        int c = *p - 'a';
        if (c < 0 || c >= 26 || !cur->children[c]) return 0;
        cur = cur->children[c];
    }
    return cur->terminal;
}

static void trie_walk(const TNode *cur, char *buf, int depth, void (*visit)(const char *)) {
    if (cur->terminal) { buf[depth] = '\0'; visit(buf); }
    for (int c = 0; c < 26; ++c) {
        if (cur->children[c]) {
            buf[depth] = 'a' + c;
            trie_walk(cur->children[c], buf, depth + 1, visit);
        }
    }
}

static void trie_prefix_iter(const TNode *root, const char *prefix, void (*visit)(const char *)) {
    const TNode *cur = root;
    char buf[128];
    int depth = 0;
    for (const char *p = prefix; *p; ++p) {
        int c = *p - 'a';
        if (c < 0 || c >= 26 || !cur->children[c]) return;
        buf[depth++] = *p;
        cur = cur->children[c];
    }
    trie_walk(cur, buf, depth, visit);
}

static int trie_count_nodes(const TNode *n) {
    if (!n) return 0;
    int c = 1;
    for (int i = 0; i < 26; ++i) c += trie_count_nodes(n->children[i]);
    return c;
}

static void trie_free(TNode *n) {
    if (!n) return;
    for (int i = 0; i < 26; ++i) trie_free(n->children[i]);
    free(n);
}

/* ============================================================ */
/* Radix trie — uses a fixed-size child table for simplicity    */
/* ============================================================ */

typedef struct RNode {
    char         *edge;             /* substring on the incoming edge */
    int           terminal;
    int           n_children;
    struct RNode *children[26];
} RNode;

static RNode *rnew(const char *edge_start, size_t edge_len) {
    RNode *n = calloc(1, sizeof(RNode));
    n->edge = malloc(edge_len + 1);
    memcpy(n->edge, edge_start, edge_len);
    n->edge[edge_len] = '\0';
    return n;
}

static void rinsert(RNode *root, const char *word) {
    RNode *cur = root;
    while (*word) {
        int c = *word - 'a';
        if (c < 0 || c >= 26) return;
        RNode *child = cur->children[c];
        if (!child) {
            cur->children[c] = rnew(word, strlen(word));
            cur->children[c]->terminal = 1;
            cur->n_children++;
            return;
        }
        /* Find common prefix length between word and child->edge. */
        size_t i = 0;
        while (child->edge[i] && word[i] && child->edge[i] == word[i]) ++i;
        if (i == strlen(child->edge)) {
            /* Whole edge matched: descend into child. */
            cur = child;
            word += i;
            if (!*word) { cur->terminal = 1; return; }
            continue;
        }
        /* Partial match: split the edge. */
        RNode *new_mid = rnew(child->edge, i);
        new_mid->n_children = 1;
        /* Truncate child's edge to the divergence. */
        memmove(child->edge, child->edge + i, strlen(child->edge) - i + 1);
        new_mid->children[(unsigned)child->edge[0] - 'a'] = child;
        cur->children[c] = new_mid;
        /* Insert the remainder of the new word as a sibling. */
        if (word[i]) {
            new_mid->children[(unsigned)word[i] - 'a'] = rnew(word + i, strlen(word + i));
            new_mid->children[(unsigned)word[i] - 'a']->terminal = 1;
            new_mid->n_children++;
        } else {
            new_mid->terminal = 1;
        }
        return;
    }
}

static int rcontains(const RNode *cur, const char *word) {
    while (*word) {
        int c = *word - 'a';
        if (c < 0 || c >= 26 || !cur->children[c]) return 0;
        const RNode *ch = cur->children[c];
        size_t i = 0;
        while (ch->edge[i] && word[i] && ch->edge[i] == word[i]) ++i;
        if (ch->edge[i] != '\0') return 0;        /* edge doesn't fully match */
        word += i;
        cur = ch;
    }
    return cur->terminal;
}

static int rcount_nodes(const RNode *n) {
    if (!n) return 0;
    int c = 1;
    for (int i = 0; i < 26; ++i) c += rcount_nodes(n->children[i]);
    return c;
}

static void rfree(RNode *n) {
    if (!n) return;
    for (int i = 0; i < 26; ++i) rfree(n->children[i]);
    free(n->edge);
    free(n);
}

/* ============================================================ */
/* Demo                                                          */
/* ============================================================ */

static void print_word(const char *w) { printf("    %s\n", w); }

int main(void) {
    const char *words[] = {
        "cat", "car", "card", "care", "careful", "core", "cot",
        "dog", "dot", "door", "doom", "dorm", "door",
        "apple", "app", "application",
    };
    int n_words = sizeof(words) / sizeof(words[0]);

    TNode *t = tnew();
    for (int i = 0; i < n_words; ++i) trie_insert(t, words[i]);
    printf("== Plain trie ==\n");
    printf("  nodes: %d (for %d words)\n", trie_count_nodes(t), n_words);
    printf("  contains 'card': %d  contains 'car': %d  contains 'ca': %d\n",
           trie_contains(t, "card"), trie_contains(t, "car"), trie_contains(t, "ca"));
    printf("  prefix 'ca':\n");
    trie_prefix_iter(t, "ca", print_word);

    RNode *r = rnew("", 0);
    for (int i = 0; i < n_words; ++i) rinsert(r, words[i]);
    printf("\n== Radix trie ==\n");
    printf("  nodes: %d (compressed)\n", rcount_nodes(r));
    printf("  contains 'card': %d  contains 'care': %d  contains 'ca': %d\n",
           rcontains(r, "card"), rcontains(r, "care"), rcontains(r, "ca"));

    trie_free(t);
    rfree(r);
    return 0;
}
