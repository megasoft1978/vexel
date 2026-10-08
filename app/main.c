#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main(int argc, char *argv[])
{
    if (argc == 2 && strcmp(argv[1], "--version") == 0) {
        if (puts("svgworkbench 0.0.1") == EOF) {
            return EXIT_FAILURE;
        }
        return EXIT_SUCCESS;
    }

    (void)fputs("usage: svgworkbench --version\n", stderr);
    return EXIT_FAILURE;
}
