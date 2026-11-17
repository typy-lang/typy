/* Runs a command and writes its wall time in nanoseconds and its peak resident size in KiB to a
   file: `measure RESULT COMMAND...`. The peak the kernel reports for a process carries over an
   exec, so the command is forked from this small program, not from the large one that starts
   it, whose own peak it would otherwise report. Exits with the command's status. */
#include <stdio.h>
#include <stdlib.h>
#include <sys/resource.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>

int main(int argc, char **argv) {
    if (argc < 3) {
        fprintf(stderr, "usage: measure RESULT COMMAND...\n");
        return 2;
    }
    struct timespec start, end;
    clock_gettime(CLOCK_MONOTONIC, &start);
    pid_t child = fork();
    if (child < 0) return 2;
    if (child == 0) {
        execvp(argv[2], argv + 2);
        _exit(127);
    }
    int status = 0;
    struct rusage usage;
    if (wait4(child, &status, 0, &usage) < 0) return 2;
    clock_gettime(CLOCK_MONOTONIC, &end);
    long long nanoseconds =
        (end.tv_sec - start.tv_sec) * 1000000000LL + (end.tv_nsec - start.tv_nsec);
    FILE *result = fopen(argv[1], "w");
    if (!result) return 2;
    fprintf(result, "%lld %ld\n", nanoseconds, usage.ru_maxrss);
    fclose(result);
    return WIFEXITED(status) ? WEXITSTATUS(status) : 128 + WTERMSIG(status);
}
