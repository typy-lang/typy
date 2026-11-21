# ADR-0075: Read the stack of each thread a library runs on from the thread

## Status

Accepted

## Date

2026-11-20

## Context

The runtime raises `RecursionError` when the stack pointer comes within a
margin of the stack's lowest address (FUN-02). A library records that
address each time an export is called, since its host may call it from any
thread (ADR-0044). On Linux it takes every thread's stack to be as large as
`RLIMIT_STACK`, read once from `getrlimit`. A thread its C host creates
with a smaller stack can therefore overflow before `RecursionError`, and
the process ends with a segmentation fault (OI-30).

`pthread_getattr_np` gives a thread its own stack, from the attributes it
was created with. For the main thread glibc reads `/proc/self/maps` on each
call, and the main thread's stack is `RLIMIT_STACK` in any case.

## Options considered

1. **Ask each thread once.** On a thread other than the main one, the first
   export call reads the thread's stack with `pthread_getattr_np` and keeps
   its size in the thread's own storage; the main thread keeps
   `getrlimit`.
2. **Document a minimum.** A host gives each thread it creates at least
   `RLIMIT_STACK` of stack.
3. **Let the host set it.** The library exports a function, such as
   `typy_set_stack_limit`, that a host calls on each thread it creates.

## Decision

TyPy takes option 1. The runtime tells the main thread from the others by
its thread ID, which equals the process ID only on the main thread. On any
other thread the first export call reads the lowest address of the
thread's stack with `pthread_getattr_np` and keeps it in thread-local
storage, beside the size the check already keeps; later calls on the
thread reuse it. The main thread's limit still comes from `getrlimit`, so
`/proc/self/maps` is never read. macOS already reads each thread's own
stack (ADR-0044) and is unchanged.

## Consequences

- A library raises `RecursionError` on a host's thread of any stack size,
  and a host needs to do nothing.
- Each thread pays one `pthread_getattr_np` call, on its first export
  call; the check itself is unchanged.
- The runtime binds `pthread_getattr_np`, `pthread_attr_getstack` and
  `pthread_attr_destroy`, which glibc and musl both provide.

## Related requirements

FFI-15, FUN-02, ADR-0044, OI-30.
