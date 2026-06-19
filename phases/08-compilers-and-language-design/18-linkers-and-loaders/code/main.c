/*
 * Lesson 18: Linkers and Loaders — Simplified Linker
 *
 * Demonstrates the core concepts of linking:
 *   - Symbol tables with defined and undefined symbols
 *   - Object files with sections (.text, .data, .bss)
 *   - Relocation entries and relocation processing
 *   - Symbol resolution across multiple object files
 *
 * Compile: gcc -o linker main.c
 * Run:     ./linker
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <stdbool.h>

/* ------------------------------------------------------------------ */
/* Constants                                                          */
/* ------------------------------------------------------------------ */

#define MAX_SYMBOLS     64
#define MAX_RELOCS      64
#define MAX_SECTIONS    8
#define MAX_OBJECTS     8
#define MAX_TEXT_SIZE    4096
#define MAX_DATA_SIZE    1024
#define MAX_OUTPUT_SIZE  8192

/* ------------------------------------------------------------------ */
/* Symbol                                                             */
/* ------------------------------------------------------------------ */

typedef enum {
    SYMBOL_DEFINED,
    SYMBOL_UNDEFINED,
} SymbolBinding;

typedef struct {
    char name[64];
    SymbolBinding binding;
    /* For defined symbols: section + offset */
    int  section_index;   /* -1 if undefined */
    uint32_t offset;      /* offset within section */
    /* Resolved absolute address (filled by linker) */
    uint32_t resolved_addr;
} Symbol;

/* ------------------------------------------------------------------ */
/* Relocation                                                         */
/* ------------------------------------------------------------------ */

typedef enum {
    REL_PC32,   /* PC-relative 32-bit (like R_X86_64_PC32) */
    REL_ABS64,  /* Absolute address (like R_X86_64_64) */
} RelocType;

typedef struct {
    RelocType type;
    int  section_index;   /* which section contains the fixup */
    uint32_t offset;      /* offset within that section */
    char symbol_name[64]; /* symbol being relocated */
    int  addend;          /* addend for the relocation */
} Relocation;

/* ------------------------------------------------------------------ */
/* Section                                                            */
/* ------------------------------------------------------------------ */

typedef enum {
    SEC_TEXT,
    SEC_DATA,
    SEC_BSS,
} SectionType;

typedef struct {
    SectionType type;
    char name[16];
    uint8_t data[MAX_TEXT_SIZE];
    uint32_t size;
} Section;

/* ------------------------------------------------------------------ */
/* Object File                                                        */
/* ------------------------------------------------------------------ */

typedef struct {
    char filename[64];

    Section sections[MAX_SECTIONS];
    int     num_sections;

    Symbol  symbols[MAX_SYMBOLS];
    int     num_symbols;

    Relocation relocs[MAX_RELOCS];
    int        num_relocs;
} ObjectFile;

/* ------------------------------------------------------------------ */
/* Helper: find section index by type                                 */
/* ------------------------------------------------------------------ */

static int find_section(ObjectFile *obj, SectionType type) {
    for (int i = 0; i < obj->num_sections; i++) {
        if (obj->sections[i].type == type) return i;
    }
    return -1;
}

/* ------------------------------------------------------------------ */
/* Helper: find symbol by name                                        */
/* ------------------------------------------------------------------ */

static Symbol *find_symbol(ObjectFile *obj, const char *name) {
    for (int i = 0; i < obj->num_symbols; i++) {
        if (strcmp(obj->symbols[i].name, name) == 0) {
            return &obj->symbols[i];
        }
    }
    return NULL;
}

/* ------------------------------------------------------------------ */
/* Create a simple object file (simulated compilation unit)           */
/* ------------------------------------------------------------------ */

static void create_object_file(ObjectFile *obj, const char *filename) {
    memset(obj, 0, sizeof(*obj));
    strncpy(obj->filename, filename, sizeof(obj->filename) - 1);
}

static void add_section(ObjectFile *obj, SectionType type, const char *name) {
    Section *sec = &obj->sections[obj->num_sections++];
    sec->type = type;
    strncpy(sec->name, name, sizeof(sec->name) - 1);
    sec->size = 0;
}

static void add_text_bytes(ObjectFile *obj, const uint8_t *data, uint32_t len) {
    int idx = find_section(obj, SEC_TEXT);
    if (idx < 0) return;
    Section *sec = &obj->sections[idx];
    memcpy(sec->data + sec->size, data, len);
    sec->size += len;
}

static void add_data_bytes(ObjectFile *obj, const uint8_t *data, uint32_t len) {
    int idx = find_section(obj, SEC_DATA);
    if (idx < 0) return;
    Section *sec = &obj->sections[idx];
    memcpy(sec->data + sec->size, data, len);
    sec->size += len;
}

static void define_symbol(ObjectFile *obj, const char *name,
                          SectionType sec_type, uint32_t offset) {
    Symbol *sym = &obj->symbols[obj->num_symbols++];
    strncpy(sym->name, name, sizeof(sym->name) - 1);
    sym->binding = SYMBOL_DEFINED;
    sym->section_index = find_section(obj, sec_type);
    sym->offset = offset;
    sym->resolved_addr = 0;
}

static void add_undefined_symbol(ObjectFile *obj, const char *name) {
    Symbol *sym = &obj->symbols[obj->num_symbols++];
    strncpy(sym->name, name, sizeof(sym->name) - 1);
    sym->binding = SYMBOL_UNDEFINED;
    sym->section_index = -1;
    sym->offset = 0;
    sym->resolved_addr = 0;
}

static void add_relocation(ObjectFile *obj, RelocType type,
                           int sec_index, uint32_t offset,
                           const char *sym_name, int addend) {
    Relocation *rel = &obj->relocs[obj->num_relocs++];
    rel->type = type;
    rel->section_index = sec_index;
    rel->offset = offset;
    strncpy(rel->symbol_name, sym_name, sizeof(rel->symbol_name) - 1);
    rel->addend = addend;
}

/* ------------------------------------------------------------------ */
/* Print object file info                                             */
/* ------------------------------------------------------------------ */

static void print_object_file(const ObjectFile *obj) {
    printf("Object file: %s\n", obj->filename);

    printf("  Sections (%d):\n", obj->num_sections);
    for (int i = 0; i < obj->num_sections; i++) {
        const Section *sec = &obj->sections[i];
        printf("    [%d] %-8s  size=%u  type=%s\n",
               i, sec->name, sec->size,
               sec->type == SEC_TEXT ? "TEXT" :
               sec->type == SEC_DATA ? "DATA" : "BSS");
    }

    printf("  Symbols (%d):\n", obj->num_symbols);
    for (int i = 0; i < obj->num_symbols; i++) {
        const Symbol *sym = &obj->symbols[i];
        printf("    %-20s  %s", sym->name,
               sym->binding == SYMBOL_DEFINED ? "DEFINED  " : "UNDEFINED");
        if (sym->binding == SYMBOL_DEFINED) {
            printf("  sec=%d  offset=0x%x", sym->section_index, sym->offset);
        }
        printf("\n");
    }

    printf("  Relocations (%d):\n", obj->num_relocs);
    for (int i = 0; i < obj->num_relocs; i++) {
        const Relocation *rel = &obj->relocs[i];
        printf("    sec=%d  offset=0x%x  sym=%-20s  type=%s  addend=%d\n",
               rel->section_index, rel->offset, rel->symbol_name,
               rel->type == REL_PC32 ? "PC32" : "ABS64", rel->addend);
    }
    printf("\n");
}

/* ------------------------------------------------------------------ */
/* Linker: combine object files                                       */
/* ------------------------------------------------------------------ */

typedef struct {
    uint8_t text[MAX_OUTPUT_SIZE];
    uint32_t text_size;
    uint8_t data[MAX_OUTPUT_SIZE];
    uint32_t data_size;

    /* Global symbol table: name -> final address */
    Symbol global_symbols[MAX_SYMBOLS];
    int    num_global_symbols;
} LinkedOutput;

/* Find a symbol in the global table */
static Symbol *find_global_symbol(LinkedOutput *out, const char *name) {
    for (int i = 0; i < out->num_global_symbols; i++) {
        if (strcmp(out->global_symbols[i].name, name) == 0) {
            return &out->global_symbols[i];
        }
    }
    return NULL;
}

/* Resolve a 32-bit value into a byte buffer (little-endian) */
static void write_u32(uint8_t *buf, uint32_t offset, uint32_t value) {
    buf[offset + 0] = (value >>  0) & 0xFF;
    buf[offset + 1] = (value >>  8) & 0xFF;
    buf[offset + 2] = (value >> 16) & 0xFF;
    buf[offset + 3] = (value >> 24) & 0xFF;
}

/* Read a 32-bit value from a byte buffer (little-endian) */
static uint32_t read_u32(const uint8_t *buf, uint32_t offset) {
    return (uint32_t)buf[offset]
        | ((uint32_t)buf[offset + 1] << 8)
        | ((uint32_t)buf[offset + 2] << 16)
        | ((uint32_t)buf[offset + 3] << 24);
}

/*
 * Simplified link:
 *   1. Lay out sections: all .text first, then all .data.
 *   2. Build global symbol table (defined symbols get final addresses).
 *   3. Resolve undefined symbols from the global table.
 *   4. Apply relocations.
 */
static bool link_objects(ObjectFile objects[], int num_objects,
                         LinkedOutput *output) {
    memset(output, 0, sizeof(*output));

    /* Text base address: 0x400000 (typical for executables) */
    uint32_t text_base = 0x400000;
    /* Data base: after text, aligned to 16 bytes */
    uint32_t text_total = 0;

    /* Pass 1: Lay out sections and compute final addresses */
    for (int i = 0; i < num_objects; i++) {
        ObjectFile *obj = &objects[i];
        int text_idx = find_section(obj, SEC_TEXT);
        int data_idx = find_section(obj, SEC_DATA);

        uint32_t obj_text_base = text_base + text_total;
        uint32_t obj_data_base = 0;

        if (text_idx >= 0) {
            Section *sec = &obj->sections[text_idx];
            memcpy(output->text + output->text_size, sec->data, sec->size);
            output->text_size += sec->size;
            text_total += sec->size;
        }
        /* Data goes after all text */
        if (data_idx >= 0) {
            Section *sec = &obj->sections[data_idx];
            obj_data_base = text_base + ((text_total + 15) & ~15);
            memcpy(output->data + output->data_size, sec->data, sec->size);
            output->data_size += sec->size;
        }

        /* Resolve defined symbols to final addresses */
        for (int j = 0; j < obj->num_symbols; j++) {
            Symbol *sym = &obj->symbols[j];
            if (sym->binding == SYMBOL_DEFINED) {
                uint32_t final_addr;
                if (sym->section_index >= 0 &&
                    obj->sections[sym->section_index].type == SEC_TEXT) {
                    final_addr = obj_text_base + sym->offset;
                } else {
                    final_addr = obj_data_base + sym->offset;
                }
                sym->resolved_addr = final_addr;

                /* Add to global symbol table (or detect duplicate) */
                Symbol *existing = find_global_symbol(output, sym->name);
                if (existing) {
                    fprintf(stderr, "Error: duplicate symbol '%s'\n", sym->name);
                    return false;
                }
                output->global_symbols[output->num_global_symbols++] = *sym;
            }
        }
    }

    /* Pass 2: Resolve undefined symbols */
    for (int i = 0; i < num_objects; i++) {
        ObjectFile *obj = &objects[i];
        for (int j = 0; j < obj->num_symbols; j++) {
            Symbol *sym = &obj->symbols[j];
            if (sym->binding == SYMBOL_UNDEFINED) {
                Symbol *resolved = find_global_symbol(output, sym->name);
                if (!resolved) {
                    fprintf(stderr, "Error: undefined symbol '%s'\n", sym->name);
                    return false;
                }
                sym->resolved_addr = resolved->resolved_addr;
            }
        }
    }

    /* Pass 3: Apply relocations */
    for (int i = 0; i < num_objects; i++) {
        ObjectFile *obj = &objects[i];
        /* Compute this object's text section base */
        uint32_t obj_text_base = text_base;
        for (int k = 0; k < i; k++) {
            int ti = find_section(&objects[k], SEC_TEXT);
            if (ti >= 0) obj_text_base += objects[k].sections[ti].size;
        }

        for (int j = 0; j < obj->num_relocs; j++) {
            Relocation *rel = &obj->relocs[j];

            /* Find the symbol's final address */
            Symbol *sym = find_global_symbol(output, rel->symbol_name);
            if (!sym) {
                /* Check in this object's symbols */
                sym = find_symbol(obj, rel->symbol_name);
            }
            if (!sym || sym->resolved_addr == 0) {
                fprintf(stderr, "Error: relocation for unresolved symbol '%s'\n",
                        rel->symbol_name);
                return false;
            }

            /* Compute the patch location in the output */
            uint32_t patch_offset = obj_text_base + rel->offset - text_base;
            uint32_t sym_addr = sym->resolved_addr + rel->addend;

            if (rel->type == REL_ABS64) {
                /* Absolute: just write the symbol address (lower 32 bits for demo) */
                write_u32(output->text, patch_offset, sym_addr);
            } else if (rel->type == REL_PC32) {
                /* PC-relative: target - (patch_location + 4) */
                uint32_t pc = text_base + patch_offset + 4;
                uint32_t displacement = sym_addr - pc;
                write_u32(output->text, patch_offset, displacement);
            }
        }
    }

    return true;
}

/* ------------------------------------------------------------------ */
/* Print linked output                                                */
/* ------------------------------------------------------------------ */

static void print_linked_output(const LinkedOutput *out) {
    printf("Linked Output:\n");
    printf("  Global Symbol Table (%d symbols):\n", out->num_global_symbols);
    for (int i = 0; i < out->num_global_symbols; i++) {
        const Symbol *sym = &out->global_symbols[i];
        printf("    %-20s  -> 0x%08x\n", sym->name, sym->resolved_addr);
    }

    printf("  .text section (%u bytes):\n", out->text_size);
    for (uint32_t i = 0; i < out->text_size; i++) {
        if (i % 16 == 0) printf("    ");
        printf("%02x ", out->text[i]);
        if (i % 16 == 15 || i == out->text_size - 1) printf("\n");
    }

    printf("  .data section (%u bytes):\n", out->data_size);
    for (uint32_t i = 0; i < out->data_size; i++) {
        if (i % 16 == 0) printf("    ");
        printf("%02x ", out->data[i]);
        if (i % 16 == 15 || i == out->data_size - 1) printf("\n");
    }
    printf("\n");
}

/* ------------------------------------------------------------------ */
/* Demo: Shell commands for examining real binaries                   */
/* ------------------------------------------------------------------ */

static void print_shell_demos(void) {
    printf("--- Useful Shell Commands for Examining Binaries ---\n\n");

    printf("# Disassemble a binary:\n");
    printf("objdump -d ./myprogram\n\n");

    printf("# List symbols in a binary:\n");
    printf("nm ./myprogram\n\n");

    printf("# Show ELF structure:\n");
    printf("readelf -h ./myprogram      # ELF header\n");
    printf("readelf -S ./myprogram      # Section headers\n");
    printf("readelf -r ./myprogram.o    # Relocation entries\n\n");

    printf("# Show shared library dependencies:\n");
    printf("ldd ./myprogram\n\n");

    printf("# Compile separate object files and link:\n");
    printf("gcc -c main.c -o main.o\n");
    printf("gcc -c utils.c -o utils.o\n");
    printf("gcc main.o utils.o -o program\n\n");

    printf("# Show the linker command invoked by gcc:\n");
    printf("gcc -v main.o utils.o -o program 2>&1 | grep collect2\n\n");
}

/* ------------------------------------------------------------------ */
/* Main: demonstrate linking                                          */
/* ------------------------------------------------------------------ */

int main(void) {
    printf("=== Lesson 18: Linkers and Loaders ===\n\n");

    /* ----- Demo 1: Create two object files ----- */
    printf("--- Demo 1: Object Files ---\n\n");

    /* Object file 1: main.c — contains main() calling add_numbers() */
    ObjectFile obj_main;
    create_object_file(&obj_main, "main.o");

    add_section(&obj_main, SEC_TEXT, ".text");
    add_section(&obj_main, SEC_DATA, ".data");

    /*
     * Simulated machine code for main():
     *   call add_numbers    (4-byte PC-relative placeholder = 0x00000000)
     *   ret
     *
     * Bytes: e8 00 00 00 00  c3
     */
    uint8_t main_text[] = { 0xe8, 0x00, 0x00, 0x00, 0x00, 0xc3 };
    add_text_bytes(&obj_main, main_text, sizeof(main_text));

    define_symbol(&obj_main, "main", SEC_TEXT, 0);
    add_undefined_symbol(&obj_main, "add_numbers");

    /* Relocation: patch the call at offset 1 (the 4 bytes after e8) */
    add_relocation(&obj_main, REL_PC32, find_section(&obj_main, SEC_TEXT),
                   1, "add_numbers", -4);

    print_object_file(&obj_main);

    /* Object file 2: math.c — contains add_numbers() and global_var */
    ObjectFile obj_math;
    create_object_file(&obj_math, "math.o");

    add_section(&obj_math, SEC_TEXT, ".text");
    add_section(&obj_math, SEC_DATA, ".data");

    /*
     * Simulated machine code for add_numbers(a, b):
     *   lea eax, [rdi + rsi]    (3 bytes: 8d 04 37)
     *   ret                      (1 byte: c3)
     */
    uint8_t math_text[] = { 0x8d, 0x04, 0x37, 0xc3 };
    add_text_bytes(&obj_math, math_text, sizeof(math_text));

    /* A data section with a 4-byte integer value */
    uint8_t global_val[] = { 0x2a, 0x00, 0x00, 0x00 }; /* 42 */
    add_data_bytes(&obj_math, global_val, sizeof(global_val));

    define_symbol(&obj_math, "add_numbers", SEC_TEXT, 0);
    define_symbol(&obj_math, "global_var", SEC_DATA, 0);

    print_object_file(&obj_math);

    /* ----- Demo 2: Link the two object files ----- */
    printf("--- Demo 2: Linking ---\n\n");

    ObjectFile objects[] = { obj_main, obj_math };
    LinkedOutput output;

    if (link_objects(objects, 2, &output)) {
        printf("Link successful!\n\n");
        print_linked_output(&output);

        /* Verify the relocation was applied */
        uint32_t call_target = read_u32(output.text, 1);
        uint32_t add_numbers_addr = 0;
        for (int i = 0; i < output.num_global_symbols; i++) {
            if (strcmp(output.global_symbols[i].name, "add_numbers") == 0) {
                add_numbers_addr = output.global_symbols[i].resolved_addr;
                break;
            }
        }
        uint32_t expected_disp = add_numbers_addr - (0x400000 + 1 + 4);
        printf("Relocation verification:\n");
        printf("  add_numbers is at:        0x%08x\n", add_numbers_addr);
        printf("  Call displacement stored:  0x%08x\n", call_target);
        printf("  Expected displacement:     0x%08x\n", expected_disp);
        printf("  Match: %s\n\n",
               call_target == expected_disp ? "YES" : "NO");
    }

    /* ----- Demo 3: Error case — undefined symbol ----- */
    printf("--- Demo 3: Undefined Symbol Error ---\n\n");

    ObjectFile obj_bad;
    create_object_file(&obj_bad, "bad.o");
    add_section(&obj_bad, SEC_TEXT, ".text");
    uint8_t bad_text[] = { 0xe8, 0x00, 0x00, 0x00, 0x00 };
    add_text_bytes(&obj_bad, bad_text, sizeof(bad_text));
    define_symbol(&obj_bad, "start", SEC_TEXT, 0);
    add_undefined_symbol(&obj_bad, "nonexistent_func");
    add_relocation(&obj_bad, REL_PC32, find_section(&obj_bad, SEC_TEXT),
                   1, "nonexistent_func", 0);

    ObjectFile single[] = { obj_bad };
    LinkedOutput bad_output;
    if (!link_objects(single, 1, &bad_output)) {
        printf("Link failed as expected (undefined symbol).\n\n");
    }

    /* ----- Shell demos ----- */
    print_shell_demos();

    /* ----- Summary ----- */
    printf("--- Summary ---\n");
    printf("Object files: ELF sections (.text, .data, .bss), symbol table, relocations\n");
    printf("Static linking: resolve symbols, assign addresses, apply relocations\n");
    printf("Dynamic linking: PLT/GOT for lazy resolution of shared library symbols\n");
    printf("Loader: maps segments into memory, sets up stack, jumps to entry point\n");

    return 0;
}
