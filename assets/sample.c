/* Sample binary bundled with the web build of drinsider.
 * Build: gcc -O2 -o assets/sample assets/sample.c -lm */
#include <math.h>
#include <stdio.h>
#include <string.h>

static const char banner[] = "drinsider sample: analyze ELF binaries like a normal person";

int main(int argc, char **argv) {
    double x = argc > 1 ? (double)strlen(argv[1]) : 2.0;
    printf("%s\nsqrt(%.1f) = %f\n", banner, x, sqrt(x));
    return 0;
}
