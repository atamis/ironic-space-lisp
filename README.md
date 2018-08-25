# ironic-space-lisp

Weird VM-lisp in Rust. It's inspired by Erlang, Clojure and Scheme. It's
architected on the assertion that if you never jump into input-based code, it's
much harder to get exploited.

## Plan

strings &rarr; sexprs &rarr; AST &rarr; "bytecode" &rarr; VM &rarr; scheduler

Currently working on "bytecode" &rarr; VM.

### strings &rarr; sexprs

Parse raw strings into raw lisp datums, called `Literal`s.

### sexprs &rarr; AST

Parse special forms to make subsequent operations easier. Also run some AST
passes, like searching for unbound vars.

### AST &rarr; "bytecode"

Convert prefix to infix, and all code into opcodes.

Maybe the above step should convert regular lisp code into prefix "opcodes", and
this converts to infix and makes it real "bytecode".

"bytecode" because it's not really bytecode, it's a weird enum mix of literals
and opcodes.

Potentially more compilations steps here to make code generation easier. In
particular, a pre-state where jumps and conditional jumps are relative rather
than absolute.

### "bytecode" &rarr; VM

VM runs the "bytecode" in its own environment. Because the "bytecode" is
concatenative postfix, can easy run single or batched steps.

### VM &rarr; scheduler

Run multiple VMs on multiple threads in an N:M model with preemptive scheduling.


## Reuse

The parser can be used to implement the perenial `parse` function. Additionally,
I implemented an interpreter for macro use. The interpreter could be used to
implement an `eval` function, but the language should be easily self-hosted,
so it's probably better to put that in the preamble.
