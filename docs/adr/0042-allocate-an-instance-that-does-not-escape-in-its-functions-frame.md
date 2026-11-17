# ADR-0042: Allocate an instance that does not escape in its function's frame

## Status

Accepted

## Date

2026-11-17

## Context

ADR-0001 sets out the layers that keep TyPy's memory management cheap, and
its second is stack allocation: an object that escape analysis proves does
not outlive its function is allocated on the stack (MEM-03). Every object of
TyPy 0.4 comes from the heap, through `crate::allocator`, and is freed when its
count reaches zero.

An instance of a class of the program is created by `NewInstance` in TyIR,
then counted like any object: a variable that holds it, an argument that
borrows it and a field that stores it each follow the rules of MEM-04 and
MEM-09. An instance created inside a loop is created again on each turn, and
an earlier one may still be held when the next is made, as in
`previous = current; current = Point(...)`.

Three questions decide the design: which instances qualify, how a counted
object can live in a frame without the count ever freeing it, and how an
instance made in a loop gets memory of its own when the last one is still
alive.

## Options considered

1. **Prove the last use, and free nothing.** The analysis also proves where
   each instance's last reference goes, and the compiler leaves out its
   counting. It needs a liveness analysis of every slot and block parameter
   along every path, and an error in it frees memory that is still read.
2. **A counted object in the frame, guarded by its count.** Each allocation
   site whose instances do not escape gets a block of the instance's size in
   the frame. The instance's count starts with a large bias, so the ordinary
   decrements of the program never bring it to zero, and the block's
   attributes are released when the function returns. A site inside a loop
   reuses its block only when the count shows the instance before is no
   longer held, and otherwise takes memory from the heap as before.
3. **Region allocation**: every instance of a call in an arena freed when the
   call returns. Instances that do escape would need copying out.

## Decision

TyPy uses option 2.

- **What qualifies.** An instance of a class of the program, not of an
  exception class, made by `NewInstance`, whose value and every value that
  may hold it, through slots, block parameters, casts and optionals, is only
  read, has its attributes read or stored, is counted, is compared by
  identity, or is passed to a parameter that does not escape. It escapes when
  it is returned, stored in an attribute of another object or in a module
  variable, put in a tuple, passed to a runtime function other than one that
  only reads its type, or passed to a parameter that escapes.
- **Parameters.** A parameter escapes by the same rules, found for every
  function of the program together, starting from none and adding each
  parameter shown to escape until nothing changes. A call through a method
  table counts the function that each table of the class, and of every
  class derived from it, holds at that position.
- **The frame block.** The block's header is cleared when the function
  starts. `typy_rt_instance_on_stack` makes the instance in it with a count of
  2^30 + 1; when the block is in use and its count is back to 2^30, the
  instance before is unreferenced, so its attributes are released and the
  block is used again; when the count is higher, a heap instance is made.
  Before every return, `typy_rt_instance_off_stack` releases the attributes of
  a block in use, after checking that no reference is left, and stops the
  program with a message if one is, so a flaw in the analysis cannot corrupt
  memory silently.
- **Not yet.** Lists, dictionaries, sets, strings and exception objects that
  do not escape come from the heap, planned for 1.0 (MEM-10).

## Consequences

- An instance used only within its function, such as a value object built
  for a computation, costs no allocator call and no free, and is the same
  object in every other way: its identity, its attributes and its methods.
- The attributes of a frame block are released when the function returns or
  when its site makes the next instance, not when the instance's last
  reference goes; no finaliser can observe the difference, since TyPy runs
  none, but the attributes' memory may be held a little longer.
- Each qualifying site adds the size of one instance to its function's frame.
- The analysis runs over the whole program once it is lowered, for the debug
  and the release build alike, so both run alike (NFR-15).

## Related requirements

MEM-01, MEM-03, MEM-04, MEM-09, MEM-10, NFR-15, ADR-0001, ADR-0021.
