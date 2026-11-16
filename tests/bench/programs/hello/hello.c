/* Startup: a program that prints one line. */
#include <stdio.h>

int main(int argc, char **argv) {
    (void)argv;
    printf("hello %d\n", argc);
    return 0;
}
