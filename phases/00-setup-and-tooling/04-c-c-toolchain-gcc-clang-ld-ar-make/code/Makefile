# Lesson Makefile — builds the `greet` binary three ways:
#   1. From .o files directly
#   2. From a static library (libgreet.a)
#   3. From a shared library (libgreet.so / .dylib)
#
# Read this file from top to bottom — the order matters.

# ── Variables ────────────────────────────────────────────────────────
CC      = cc                       # use whatever 'cc' resolves to (gcc or clang)
CFLAGS  = -Wall -Wextra -O0 -g     # warnings on, debug build by default
ARFLAGS = rcs

# OS-specific shared-library extension
UNAME_S := $(shell uname -s)
ifeq ($(UNAME_S),Darwin)
  SHARED_EXT = dylib
  SHARED_FLAGS = -dynamiclib
else
  SHARED_EXT = so
  SHARED_FLAGS = -shared
endif

# ── Phony targets ────────────────────────────────────────────────────
.PHONY: all clean test pipeline static shared

all: greet

# ── Default build ────────────────────────────────────────────────────
# Pattern rule: any %.o depends on the matching %.c.
# $< = first prerequisite, $@ = target name.
%.o: %.c greet.h
	$(CC) $(CFLAGS) -c $< -o $@

greet: main.o greet.o
	$(CC) $^ -o $@

# ── Static-library build ─────────────────────────────────────────────
libgreet.a: greet.o
	ar $(ARFLAGS) $@ $^

static: main.o libgreet.a
	$(CC) main.o -L. -lgreet -o greet_static

# ── Shared-library build (requires PIC objects) ──────────────────────
greet.pic.o: greet.c greet.h
	$(CC) $(CFLAGS) -fPIC -c greet.c -o $@

libgreet.$(SHARED_EXT): greet.pic.o
	$(CC) $(SHARED_FLAGS) $^ -o $@

shared: main.o libgreet.$(SHARED_EXT)
	$(CC) main.o -L. -lgreet -o greet_shared

# ── Pipeline demonstration ───────────────────────────────────────────
# Produce each stage's intermediate file so you can inspect them.
pipeline:
	$(CC) -E main.c > pipeline.i
	$(CC) -S main.c -o pipeline.s
	$(CC) -c main.c -o pipeline.o
	@echo "Inspect: pipeline.i (preprocessed), pipeline.s (asm), pipeline.o (object)."

# ── Test ─────────────────────────────────────────────────────────────
test: greet
	./greet | grep -q "hello, world"
	./greet alice | grep -q "hello, alice"
	@echo "  ✓ greet runs and produces expected output"

# ── Cleanup ──────────────────────────────────────────────────────────
clean:
	rm -f *.o *.i *.s *.a *.so *.dylib greet greet_static greet_shared pipeline.*
