# ironic-space-lisp

Weird VM-lisp in Rust.

## Plan

lisp strings -> AST -> simplified AST -> VM -> scheduler

Currently working on "bytecode" -> VM.

### lisp strings -> AST

Use some kind of parser library. Hopefully parses to built in data structures.

### AST -> simplified AST

"Undo" all the sugar

### simplified AST -> VM

Run the simplified AST directly in the stepped VM. VM has to be stepped to
enable preemptive scheduling.

### VM -> scheduler

Run multiple VMs on multiple threads in an N:M model with preemptive scheduling.
