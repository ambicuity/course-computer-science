# Minimal multi-file C project Makefile.
#
# Per-translation-unit compilation, then link.

CC      = cc
CFLAGS  = -Wall -Wextra -O0 -g

OBJS = main.o calc.o

.PHONY: all clean

all: app

app: $(OBJS)
	$(CC) $^ -o $@

# Pattern rule: any .o depends on the matching .c plus its header (if any).
%.o: %.c calc.h
	$(CC) $(CFLAGS) -c $< -o $@

clean:
	rm -f *.o app
