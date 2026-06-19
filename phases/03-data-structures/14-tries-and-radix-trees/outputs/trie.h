/*
 * trie.h — single-header character trie (lowercase a-z).
 *
 *   TrieNode *t = trie_new();
 *   trie_insert(t, "hello");
 *   if (trie_contains(t, "hello")) ...
 *   trie_prefix(t, "he", visitor, ctx);
 *   trie_free(t);
 */
#ifndef TRIE_H
#define TRIE_H

#include <stdlib.h>
#include <stdbool.h>

typedef struct TrieNode {
    struct TrieNode *children[26];
    int              terminal;
} TrieNode;

typedef void (*TrieVisitor)(const char *word, void *ctx);

static inline TrieNode *trie_new(void) { return (TrieNode *)calloc(1, sizeof(TrieNode)); }

static inline void trie_insert(TrieNode *root, const char *word) {
    TrieNode *cur = root;
    for (const char *p = word; *p; ++p) {
        int c = *p - 'a';
        if (c < 0 || c >= 26) continue;
        if (!cur->children[c]) cur->children[c] = trie_new();
        cur = cur->children[c];
    }
    cur->terminal = 1;
}

static inline bool trie_contains(const TrieNode *cur, const char *word) {
    for (const char *p = word; *p; ++p) {
        int c = *p - 'a';
        if (c < 0 || c >= 26 || !cur->children[c]) return false;
        cur = cur->children[c];
    }
    return cur->terminal != 0;
}

static inline void trie__walk(const TrieNode *cur, char *buf, int depth, TrieVisitor f, void *ctx) {
    if (cur->terminal) { buf[depth] = '\0'; f(buf, ctx); }
    for (int c = 0; c < 26; ++c) {
        if (cur->children[c]) {
            buf[depth] = (char)('a' + c);
            trie__walk(cur->children[c], buf, depth + 1, f, ctx);
        }
    }
}

static inline void trie_prefix(const TrieNode *root, const char *prefix, TrieVisitor f, void *ctx) {
    const TrieNode *cur = root;
    char buf[256];
    int depth = 0;
    for (const char *p = prefix; *p; ++p) {
        int c = *p - 'a';
        if (c < 0 || c >= 26 || !cur->children[c]) return;
        buf[depth++] = *p;
        cur = cur->children[c];
    }
    trie__walk(cur, buf, depth, f, ctx);
}

static inline void trie_free(TrieNode *n) {
    if (!n) return;
    for (int i = 0; i < 26; ++i) trie_free(n->children[i]);
    free(n);
}

#endif /* TRIE_H */
