/* defensive.h — assertion vocabulary for C/C++.
 *
 * Define DEFENSIVE_DEBUG to enable all checks (typical: in dev/CI builds).
 * Otherwise REQUIRE/ENSURE/INVARIANT compile to optimization hints, telling
 * the compiler "this condition is true," helping it generate better code.
 *
 * UNREACHABLE() marks a code path that should be impossible; both modes use
 * the compiler's __builtin_unreachable() so the optimizer can prune it.
 */
#ifndef DEFENSIVE_H
#define DEFENSIVE_H

#include <stdio.h>
#include <stdlib.h>

/* Active checks: assert at runtime; abort on failure. */
#define DEFENSIVE_FAIL(msg, cond) do {                                        \
        fprintf(stderr, "%s:%d: defensive check failed: %s\n  condition: %s\n", \
                __FILE__, __LINE__, (msg), #cond);                            \
        abort();                                                              \
    } while (0)

#ifdef DEFENSIVE_DEBUG
    #define REQUIRE(cond)   do { if (!(cond)) DEFENSIVE_FAIL("precondition",  cond); } while (0)
    #define ENSURE(cond)    do { if (!(cond)) DEFENSIVE_FAIL("postcondition", cond); } while (0)
    #define INVARIANT(cond) do { if (!(cond)) DEFENSIVE_FAIL("invariant",     cond); } while (0)
#else
    /* In release, tell the optimizer the condition is true. */
    #if defined(__GNUC__) || defined(__clang__)
        #define ASSUME(cond) do { if (!(cond)) __builtin_unreachable(); } while (0)
    #else
        #define ASSUME(cond) ((void)0)
    #endif
    #define REQUIRE(cond)   ASSUME(cond)
    #define ENSURE(cond)    ASSUME(cond)
    #define INVARIANT(cond) ASSUME(cond)
#endif

#if defined(__GNUC__) || defined(__clang__)
    #define UNREACHABLE() __builtin_unreachable()
#else
    #define UNREACHABLE() abort()
#endif

#endif /* DEFENSIVE_H */
