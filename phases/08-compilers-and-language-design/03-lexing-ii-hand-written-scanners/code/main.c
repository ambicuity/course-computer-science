/*
 * Lesson 03: Hand-Written Scanner — C implementation
 *
 * A compact hand-written lexer for a subset of tokens:
 * identifiers, keywords, integers, strings, operators, punctuation.
 * Demonstrates the state-machine dispatch approach used by GCC and Clang.
 */

#include <ctype.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* ── token types ─────────────────────────────────────────────────────── */

typedef enum {
    TOK_KEYWORD, TOK_IDENT, TOK_INT, TOK_STRING,
    TOK_OP,
    TOK_LPAREN, TOK_RPAREN, TOK_LBRACE, TOK_RBRACE,
    TOK_SEMI, TOK_COMMA, TOK_ASSIGN,
    TOK_EOF, TOK_ERROR,
} TokenKind;

typedef struct {
    TokenKind kind;
    char      text[256];
    int       int_val;
    int       line;
    int       col;
} Token;

static const char *token_kind_name(TokenKind k) {
    switch (k) {
        case TOK_KEYWORD: return "KEYWORD";
        case TOK_IDENT:   return "IDENT";
        case TOK_INT:     return "INT";
        case TOK_STRING:  return "STRING";
        case TOK_OP:      return "OP";
        case TOK_LPAREN:  return "LPAREN";
        case TOK_RPAREN:  return "RPAREN";
        case TOK_LBRACE:  return "LBRACE";
        case TOK_RBRACE:  return "RBRACE";
        case TOK_SEMI:    return "SEMI";
        case TOK_COMMA:   return "COMMA";
        case TOK_ASSIGN:  return "ASSIGN";
        case TOK_EOF:     return "EOF";
        case TOK_ERROR:   return "ERROR";
    }
    return "?";
}

static void print_token(const Token *tok) {
    if (tok->kind == TOK_INT) {
        printf("%s(%d)\n", token_kind_name(tok->kind), tok->int_val);
    } else if (tok->text[0]) {
        printf("%s(%s)\n", token_kind_name(tok->kind), tok->text);
    } else {
        printf("%s\n", token_kind_name(tok->kind));
    }
}

/* ── keywords ────────────────────────────────────────────────────────── */

static const char *keywords[] = {
    "if", "else", "while", "for", "return", "int", "char", "void", "struct", NULL
};

static int is_keyword(const char *s) {
    for (int i = 0; keywords[i]; i++) {
        if (strcmp(s, keywords[i]) == 0) return 1;
    }
    return 0;
}

/* ── scanner ─────────────────────────────────────────────────────────── */

typedef struct {
    const char *src;
    int         pos;
    int         line;
    int         col;
} Scanner;

static void scanner_init(Scanner *s, const char *src) {
    s->src  = src;
    s->pos  = 0;
    s->line = 1;
    s->col  = 1;
}

static char sc_peek(const Scanner *s) {
    return s->src[s->pos];
}

static char sc_peek2(const Scanner *s) {
    return s->src[s->pos + 1];
}

static char sc_advance(Scanner *s) {
    char c = s->src[s->pos];
    if (c == '\0') return '\0';
    s->pos++;
    if (c == '\n') { s->line++; s->col = 1; }
    else           { s->col++; }
    return c;
}

static void skip_whitespace_and_comments(Scanner *s) {
    for (;;) {
        char c = sc_peek(s);
        if (isspace((unsigned char)c)) {
            sc_advance(s);
        } else if (c == '/' && sc_peek2(s) == '/') {
            while (sc_peek(s) && sc_peek(s) != '\n') sc_advance(s);
        } else {
            break;
        }
    }
}

static Token scan_identifier(Scanner *s) {
    Token tok;
    tok.line = s->line;
    tok.col  = s->col;
    int i = 0;
    while (isalnum((unsigned char)sc_peek(s)) || sc_peek(s) == '_') {
        if (i < 255) tok.text[i++] = sc_advance(s);
        else sc_advance(s);
    }
    tok.text[i] = '\0';
    tok.kind = is_keyword(tok.text) ? TOK_KEYWORD : TOK_IDENT;
    tok.int_val = 0;
    return tok;
}

static Token scan_number(Scanner *s) {
    Token tok;
    tok.line = s->line;
    tok.col  = s->col;
    tok.kind = TOK_INT;
    tok.text[0] = '\0';
    int val = 0;
    while (isdigit((unsigned char)sc_peek(s))) {
        val = val * 10 + (sc_advance(s) - '0');
    }
    tok.int_val = val;
    return tok;
}

static Token scan_string(Scanner *s) {
    Token tok;
    tok.line = s->line;
    tok.col  = s->col;
    tok.kind = TOK_STRING;
    tok.int_val = 0;
    sc_advance(s); /* opening quote */
    int i = 0;
    while (sc_peek(s) && sc_peek(s) != '"') {
        char c = sc_advance(s);
        if (c == '\\' && sc_peek(s)) {
            char esc = sc_advance(s);
            switch (esc) {
                case 'n': c = '\n'; break;
                case 't': c = '\t'; break;
                case '\\': c = '\\'; break;
                case '"': c = '"'; break;
                default: c = esc; break;
            }
        }
        if (i < 255) tok.text[i++] = c;
    }
    tok.text[i] = '\0';
    if (sc_peek(s) == '"') sc_advance(s); /* closing quote */
    return tok;
}

static Token scan_token(Scanner *s) {
    skip_whitespace_and_comments(s);
    char c = sc_peek(s);

    if (c == '\0') {
        Token tok = { .kind = TOK_EOF, .line = s->line, .col = s->col };
        tok.text[0] = '\0';
        tok.int_val = 0;
        return tok;
    }
    if (isalpha((unsigned char)c) || c == '_') return scan_identifier(s);
    if (isdigit((unsigned char)c))              return scan_number(s);
    if (c == '"')                               return scan_string(s);

    Token tok;
    tok.line = s->line;
    tok.col  = s->col;
    tok.text[1] = '\0';
    tok.int_val = 0;

    switch (sc_advance(s)) {
        case '(': tok.kind = TOK_LPAREN; tok.text[0] = '\0'; return tok;
        case ')': tok.kind = TOK_RPAREN; tok.text[0] = '\0'; return tok;
        case '{': tok.kind = TOK_LBRACE; tok.text[0] = '\0'; return tok;
        case '}': tok.kind = TOK_RBRACE; tok.text[0] = '\0'; return tok;
        case ';': tok.kind = TOK_SEMI;   tok.text[0] = '\0'; return tok;
        case ',': tok.kind = TOK_COMMA;  tok.text[0] = '\0'; return tok;
        default: break;
    }

    /* operators */
    tok.kind = TOK_OP;
    tok.text[0] = c; tok.text[1] = '\0';
    char next = sc_peek(s);
    if ((c == '=' && next == '=') || (c == '!' && next == '=') ||
        (c == '<' && next == '=') || (c == '>' && next == '=') ||
        (c == '&' && next == '&') || (c == '|' && next == '|')) {
        tok.text[1] = sc_advance(s);
        tok.text[2] = '\0';
    }
    if (c == '=' && next != '=') {
        tok.kind = TOK_ASSIGN;
        tok.text[0] = '\0';
        /* un-advance: we already consumed '=', re-check */
        /* actually, just re-assign */
        tok.kind = TOK_ASSIGN;
    }
    return tok;
}

/* ── main ────────────────────────────────────────────────────────────── */

int main(void) {
    const char *source =
        "int fibonacci(int n) {\n"
        "    if (n <= 1) {\n"
        "        return n;\n"
        "    }\n"
        "    return fibonacci(n - 1) + fibonacci(n - 2);\n"
        "}\n"
        "\n"
        "int main() {\n"
        "    int result = fibonacci(10);\n"
        "    // compute the answer\n"
        "    char *msg = \"Hello, World!\";\n"
        "    return 0;\n"
        "}\n";

    Scanner s;
    scanner_init(&s, source);

    for (;;) {
        Token tok = scan_token(&s);
        print_token(&tok);
        if (tok.kind == TOK_EOF) break;
    }

    return 0;
}
