/* main.c — C primitive sizes + scope demo. */
#include <stdio.h>
#include <stdint.h>
#include <stdbool.h>
#include <limits.h>

static int file_scope = 100;   /* visible only in this .c file */

int main(void) {
    printf("== sizeof primitives (this platform) ==\n");
    printf("  char:        %2zu byte\n",  sizeof(char));
    printf("  short:       %2zu bytes\n", sizeof(short));
    printf("  int:         %2zu bytes\n", sizeof(int));
    printf("  long:        %2zu bytes\n", sizeof(long));
    printf("  long long:   %2zu bytes\n", sizeof(long long));
    printf("  float:       %2zu bytes\n", sizeof(float));
    printf("  double:      %2zu bytes\n", sizeof(double));
    printf("  void*:       %2zu bytes\n", sizeof(void*));
    printf("  bool:        %2zu byte\n",  sizeof(bool));

    printf("\n== sizeof <stdint.h> fixed-width (always the same) ==\n");
    printf("  int8_t:      %2zu byte\n", sizeof(int8_t));
    printf("  int16_t:     %2zu bytes\n", sizeof(int16_t));
    printf("  int32_t:     %2zu bytes\n", sizeof(int32_t));
    printf("  int64_t:     %2zu bytes\n", sizeof(int64_t));

    printf("\n== Ranges ==\n");
    printf("  INT_MIN  = %d\n", INT_MIN);
    printf("  INT_MAX  = %d\n", INT_MAX);
    printf("  UINT_MAX = %u\n", UINT_MAX);
    printf("  LLONG_MAX = %lld\n", LLONG_MAX);

    printf("\n== Scope demo (shadowing) ==\n");
    int x = 1;
    printf("  outer x = %d\n", x);
    {
        int x = 2;       /* new binding, shadows the outer */
        printf("  inner x = %d  (shadows outer)\n", x);
        {
            int x = 3;
            printf("    deeper x = %d  (shadows inner)\n", x);
        }
        printf("  inner x = %d  (back to inner binding)\n", x);
    }
    printf("  outer x = %d  (back to outer)\n", x);

    printf("\n== File scope ==\n");
    printf("  file_scope = %d (visible only in this translation unit)\n", file_scope);
    return 0;
}
