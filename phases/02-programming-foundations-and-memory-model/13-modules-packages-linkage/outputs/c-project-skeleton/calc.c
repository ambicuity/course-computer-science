/* calc.c — implementation of the calc module. */
#include "calc.h"

/* Internal linkage: this counter is invisible outside calc.c.
 * Any `extern int counter;` declaration in another .c will fail to link. */
static int counter = 0;

int add(int a, int b) { counter++; return a + b; }
int sub(int a, int b) { counter++; return a - b; }
int read_counter(void) { return counter; }
