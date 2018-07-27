# ironic-space-lisp

Weird VM-lisp in Rust.

## Plan

lisp strings -> AST -> simplified AST -> "bytecode" -> VM -> scheduler

Currentlying working on "bytecode" -> VM.

### lisp strings -> AST

Use some kind of parser library

### AST -> simplified AST

"Undo" all the sugar

### simplified AST -> "bytecode"

Convert prefix to infix, and all code into opcodes.

Maybe the above step should convert regular lisp code into prefix "opcodes", and
this converts to infix and makes it real "bytecode".

"bytecode" because it's not really bytecode, it's a weird enum mix of literals
and opcodes.

### "bytecode" -> VM

VM runs the "bytecode" in its own environment. Because the "bytecode" is
concatenative infix, can easy run single or batched steps.

### VM -> scheduler

Run multiple VMs on multiple threads in an N:M model with preemptive scheduling.
