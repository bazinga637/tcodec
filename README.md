# tcodec

![Rust](https://shields.io)
![License](https://shields.io)
![Architecture](https://shields.io)

**TCode (`tcodec`)** is a high-level, Rust-inspired programming language that compiles to custom, platform-independent bytecode for a stack-based Virtual Machine.

Designed to combine Rust's safety ergonomics with the portability and simplicity of a managed runtime, `tcodec` enforces immutability by default, static type checking, and compile-time borrow tracking before executing on its custom VM.

---

## Key Features

* **Compile-Time Borrow & Mutability Checker:** Enforces explicit `mut` declarations and tracks shared (`Shared`) and exclusive (`Mut`) variable loans across lexical scopes—preventing data races and container mutation during active `for` loop iteration.
* **Hybrid Static Type System:** Supports both explicit type annotations (`let count: int = 0`) and automatic type inference, catching type mismatches prior to bytecode generation.
* **Custom Stack-Based Bytecode VM:** Compiles AST nodes into a compact, 28-instruction `OpCode` set executed by a frame-based Virtual Machine with short-circuit boolean evaluation and `Rc<RefCell<...>>` heap-managed collections.
* **Rich Standard Library & Methods:** Built-in native functions (`range`, `randint`, `readline`, `split`, `chr`, `ord`) and collection/string methods (`.append()`, `.join()`, `.is_in()`, `.contains()`, `.trim()`).
* **Pattern Matching & Implicit Returns:** Supports `match` expressions with wildcard arms (`_`) and Rust-style implicit expression returns from functions.

---

## Quickstart

### Prerequisites

* [Rust & Cargo](https://rustup.rs) (Edition 2021+)

### Installation

Install directly via Cargo:

```bash
cargo install --git https://github.com/bazinga637/tcodec
```

Or clone and build from source:

```bash
git clone https://github.com/bazinga637/tcodec.git
cd tcodec
cargo build --release
```

### Running a Program

Running a program is just like you'd expect:

```bash
tcodec filename.tc
```

---

## Language Syntax Example

Inspired by Rust and Lua, TCode borrows familiar syntax from Rust while removing lifetime annotations and low-level memory boilerplate:

```rust
fn bubble_sort(mut items) {
  let n = len(items)

  for pass in range(0, n) {
    let mut swapped = false

    for i in range(0, n - 1 - pass) {
      if items[i] > items[i + 1] {
        let temp = items[i + 1]
        items[i + 1] = items[i]
        items[i] = temp
        swapped = true
      }
    }

    if !swapped {
      return items
    }
  }
  items // Implicit return, like in Rust
}

fn main() {
  let mut data = [42, 17, 8, 99, 23]
  data = bubble_sort(data)
  print("Sorted:", data)
}
```

### Compile-Time Safety in Action

Attempting to mutate an array while iterating over it triggers a compile-time borrow error before the VM ever starts:

```text
Compile Error [Line 4, Col 5]: Cannot mutably borrow 'items' because it is currently read-only borrowed by active loops.
```

---

## Compiler & VM Architecture

`tcodec` processes `.tc` source files through a 5-stage pipeline:

```text
Source (.tc) ──▶ Lexer ──▶ Recursive Descent Parser ──▶ Static TypeChecker ──▶ Bytecode Generator & Borrow Checker ──▶ Stack VM
```

1. **Lexer (`src/lexer.rs`):** Tokenizes source code into a stream of tokens while tracking exact `Span` (line and column) metadata for error diagnostics.
2. **Parser (`src/parser.rs`):** Builds a strongly-typed Abstract Syntax Tree (AST) with full operator precedence, unary negation, indexing, and block scoping.
3. **Type Checker (`src/typechecker.rs`):** Performs a two-pass static verification over top-level function signatures and expression trees.
4. **Bytecode Generator (`src/generator.rs`):** Lowers the AST into custom `OpCode` instructions, backpatches conditional jump offsets (`Jump`, `JumpIfFalse`), and validates variable loan lifetimes.
5. **Virtual Machine (`src/interpreter.rs`):** Executes bytecode using an operand stack and a `CallFrame` stack for isolated function local slots.

### Sample Bytecode Instruction Set (`OpCode`)

| Instruction | Operands | Description |
| :--- | :--- | :--- |
| `PushInt` / `PushString` | `val` | Pushes a primitive constant onto the VM evaluation stack. |
| `GetLocal` / `SetLocal` | `slot: usize` | Reads from or writes to a local variable slot in the active `CallFrame`. |
| `BuildArray` | `count: usize` | Pops `N` items from the stack and allocates a heap-backed `Rc<RefCell<Vec>>`. |
| `CallFunction` | `name, arg_count` | Dispatches to either a native Rust function or pushes a new user `CallFrame`. |
| `JumpIfFalse` | `target: usize` | Pops top boolean; jumps instruction pointer (`ip`) if `false`. |
| `LineUpdate` | `Span` | Updates the VM's active source span for accurate runtime error reporting. |

---

## Project Structure

```text
tcodec/
├── src/
│   ├── main.rs          # CLI entry point & pipeline orchestrator
│   ├── lexer.rs         # Tokenizer & Span tracking
│   ├── parser.rs        # Recursive descent AST parser
│   ├── typechecker.rs   # Static type inference & validation pass
│   ├── generator.rs     # Bytecode emitter & compile-time borrow checker
│   ├── interpreter.rs   # Stack-based bytecode Virtual Machine
│   ├── nativefns.rs     # Built-in global functions (print, range, randint, etc.)
│   └── nativemthds.rs   # Built-in type methods (.append, .join, .is_in, etc.)
```

---

## Roadmap

* [x] Lexer, Parser, and Span-tracked error diagnostics
* [x] Static TypeChecker with type inference
* [x] Compile time mutability and borrow checker
* [x] Stack based Bytecode VM with call frames
* [ ] Bytecode serialization to standalone `.tcb` binary files
* [ ] Add TCode to Cargo as a crate for easier installation
* [ ] Custom `struct` and `enum` definitions
* [ ] First-class closures / anonymous functions
* [ ] CLI flags (`--emit-bytecode`, `--check`)

---

## License

Distributed under the MIT License. See `LICENSE` for more information.
