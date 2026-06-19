# Same model as Lesson 04: pattern rule + automatic vars.
# Intentionally does NOT depend on greet.h — Step 1 of "build flakiness" demo.

CC      = cc
CFLAGS  = -Wall -Wextra -O0 -g

.PHONY: all clean test

all: hello

hello: hello.o greet.o
	$(CC) $^ -o $@

%.o: %.c
	$(CC) $(CFLAGS) -c $< -o $@

test: hello
	./hello | grep -q "hello, world"
	@echo "  ✓ make test passed"

clean:
	rm -f *.o hello
