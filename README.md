# tcodec

**TCode (`tcodec`)** is a higher level Rust and Lua inspired programming language that compiles to custom bytecode ran by a stack based virtual machine.

I built this language mostly for fun as a grade 11 student, but also because Rust's borrow checker system is too good to just be low level. To make something higher level while keeping the borrow checker system, I made borrowing implicit but still kept mutability explicit.

---

## Key Features

* **Compile Time Borrow & Mutability Checker:** Enforces explicit `mut` declarations and tracks shared and exclusive variable loans across scopes, preventing data races and container mutation during active `for` loop iteration.
* **Static Type System:** Supports both explicit type annotations (`let count: int = 0`) and automatic type inference as seen commonly in higher level languages, catching type mismatches before bytecode generation.
* **Custom Stack Based Bytecode VM:** Compiles AST nodes into an `OpCode` instruction set executed by a frame based Virtual Machine with short circuit boolean evaluation and `Rc<RefCell<...>>` heap managed collections.
* **Standard Library & Methods:** Standard built in native functions (`range`, `randint`, `readline`, `split`, `chr`, `ord`) and methods (`.append()`, `.join()`, `.is_in()`, `.contains()`, `.trim()`). Definitely going to expand as I work on this, its very small right now.

---

## Quickstart

### Prerequisites

* Just [Rust & Cargo](https://rustup.rs)

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

Running a program is just like you'd expect for a language with a VM:

```bash
tcodec filename.tc
```

---

## Language Syntax Example

Inspired by Rust and Lua, TCode uses familiar syntax from Rust while removing lifetime annotations and low-level memory boilerplate:

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

### Compile Time Safety

Attempting to mutate an array while iterating over it triggers a compile time borrow error before the VM ever starts for memory safety:

```text
Compile Error [Line 4, Col 5]: Cannot mutably borrow 'items' because it is currently read-only borrowed by active loops.
```

---

## Compiler & VM

`tcodec` processes the source files through a 5 step pipeline:

1. **Lexer (`src/lexer.rs`):** Tokenizes source code into an array of tokens while tracking `Span` (line and column) metadata for error diagnostics.
2. **Parser (`src/parser.rs`):** Builds a strongly typed Abstract Syntax Tree (AST) with full operator precedence, unary negation, indexing, and block scoping.
3. **Type Checker (`src/typechecker.rs`):** Performs a two pass static verification over top level function signatures and expression trees.
4. **Bytecode Generator (`src/generator.rs`):** Lowers the AST into custom `OpCode` instructions, backpatches conditional jump offsets (`Jump`, `JumpIfFalse`), and validates variable loan lifetimes.
5. **Virtual Machine (`src/interpreter.rs`):** Executes bytecode using an operand stack and a `CallFrame` stack for isolated function local slots.

### Sample Bytecode Instruction Set (`OpCode`)

| Instruction | Operands | Description |
| :--- | :--- | :--- |
| `PushInt` / `PushString` | `val` | Pushes a primitive constant onto the VM stack. |
| `GetLocal` / `SetLocal` | `slot: usize` | Reads from or writes to a local variable slot in the active `CallFrame`. |
| `BuildArray` | `count: usize` | Pops `N` items from the stack and allocates a heap-backed `Rc<RefCell<Vec>>`. |
| `CallFunction` | `name, arg_count` | Dispatches to either a native Rust function or pushes a new user `CallFrame`. |
| `JumpIfFalse` | `target: usize` | Pops top boolean, jumps instruction pointer (`ip`) if `false`. |
| `LineUpdate` | `Span` | Updates the VM's active source span for runtime error reporting. |

---
<!--
## Project Structure

```text
tcodec/
├── src/
│   ├── main.rs          # CLI entry point and pipeline orchestrator
│   ├── lexer.rs         # Tokenizer and Span tracking
│   ├── parser.rs        # Recursive descent AST parser
│   ├── typechecker.rs   # Static type inference and validation pass
│   ├── generator.rs     # Bytecode emitter and compile time borrow checker
│   ├── interpreter.rs   # Bytecode Virtual Machine
│   ├── nativefns.rs     # Built in global functions (print, range, randint, etc.)
│   └── nativemthds.rs   # Built in type methods (.append, .join, .is_in, etc.)
```

---
-->

## Roadmap

* [ ] Being able to compile program into standalone binary files
* [ ] Add TCode to Cargo as a crate for easier installation
* [ ] `struct` and `enum` definitions
* [ ] Command flags (`--bytecode`, etc.)

I'll figure out what I want to do, these are all just ideas.

---

## License

So I can post to Cargo

Distributed under the MIT License. See `LICENSE` for more information.
