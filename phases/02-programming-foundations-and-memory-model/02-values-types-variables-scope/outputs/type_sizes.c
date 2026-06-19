/* type_sizes.c — portable scanner of C primitive type sizes on this platform.
 *
 * Build:  gcc type_sizes.c -o type_sizes
 * Run:    ./type_sizes
 *
 * Useful when porting code to a new platform (embedded boards, Windows, ARM).
 */

#include <stdio.h>
#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <limits.h>

int main(void) {
    printf("type            size  signed?  min                 max\n");
    printf("--------------- ----  -------  ------------------  ------------------\n");
    printf("char            %4zu  %-7s  %18d  %18d\n",
           sizeof(char), CHAR_MIN < 0 ? "yes" : "no", CHAR_MIN, CHAR_MAX);
    printf("short           %4zu  yes      %18d  %18d\n",
           sizeof(short), SHRT_MIN, SHRT_MAX);
    printf("int             %4zu  yes      %18d  %18d\n",
           sizeof(int), INT_MIN, INT_MAX);
    printf("long            %4zu  yes      %18ld  %18ld\n",
           sizeof(long), LONG_MIN, LONG_MAX);
    printf("long long       %4zu  yes      %18lld  %18lld\n",
           sizeof(long long), LLONG_MIN, LLONG_MAX);
    printf("unsigned char   %4zu  no                            %18u\n",
           sizeof(unsigned char), UCHAR_MAX);
    printf("unsigned int    %4zu  no                            %18u\n",
           sizeof(unsigned), UINT_MAX);
    printf("unsigned long   %4zu  no                            %18lu\n",
           sizeof(unsigned long), ULONG_MAX);
    printf("float           %4zu\n",  sizeof(float));
    printf("double          %4zu\n",  sizeof(double));
    printf("long double     %4zu\n",  sizeof(long double));
    printf("void *          %4zu\n",  sizeof(void*));
    printf("size_t          %4zu  no\n", sizeof(size_t));
    printf("ptrdiff_t       %4zu  yes\n", sizeof(ptrdiff_t));
    return 0;
}
