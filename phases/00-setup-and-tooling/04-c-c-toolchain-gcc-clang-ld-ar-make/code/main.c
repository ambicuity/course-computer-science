/* main.c — driver for the toolchain walkthrough.
 *
 * The three #ifdef gates below let us trigger one error per compilation stage,
 * so you can read the error message format produced by each tool.
 *
 *   BROKEN_INCLUDE → preprocessor error
 *   BROKEN_CALL    → compile error (use of undeclared identifier)
 *   (no flag)      → linker error if you forget to link greet.o
 */

#ifdef BROKEN_INCLUDE
#include <stio.h>          /* wrong header name on purpose */
#endif

#include <stdio.h>
#include "greet.h"

int main(int argc, char **argv) {
    const char *who = (argc > 1) ? argv[1] : "world";

#ifdef BROKEN_CALL
    nonexistent_function(who);   /* undeclared on purpose */
#endif

    greet(who);
    return 0;
}
