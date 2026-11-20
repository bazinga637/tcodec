// src/natives.rs
use crate::interpreter::RuntimeValue;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{self, Write};
use std::rc::Rc;

pub type NativeFn = fn(args: &[RuntimeValue]) -> RuntimeValue;

pub fn get_native_registry() -> HashMap<String, NativeFn> {
    let mut registry: HashMap<String, NativeFn> = HashMap::new();

    macro_rules! register {
        ($($name:expr => $func:expr),* $(,)?) => {
            $( registry.insert($name.to_string(), $func); )*
        };
    }

    register! {
        "print" => native_print,
        "readline" => native_readline,
        "len" => native_len,
        "split" => native_split,
        "to_number" => native_to_number,
        "panic" => native_panic,
        "type" => native_type,
        "range" => native_range,
        "assert" => native_assert,
        "exit" => native_exit,
        "chr" => native_chr,
        "ord" => native_ord,
        "randint" => native_randint,
        "randflt" => native_randflt,
        "int" => native_int,
        "float" => native_float,
        "string" => native_string,
    }

    registry
}

// turns any list of printable arguments into a RuntimeValue::Error()
macro_rules! error {
    ($first:expr $(, $extra:expr)* $(,)?) => {{
        let err = format!("Runtime Error: {}", $first);
        $(
            let err = format!("{}{}", err, $extra);
        )*
        RuntimeValue::Error(err)
        }};
}

// defines how to handle print() as a native function
fn stringify_value(val: &RuntimeValue) -> String {
    match val {
        RuntimeValue::Int(n) => n.to_string(),
        RuntimeValue::Float(n) => n.to_string(),
        RuntimeValue::String(s) => s.as_ref().clone(),
        RuntimeValue::Boolean(b) => b.to_string(),
        RuntimeValue::Nil => "nil".to_string(),
        RuntimeValue::Array(list_ptr) => {
            let list = list_ptr.borrow();
            let mut result = String::new();
            result.push('[');
            for (i, item) in list.iter().enumerate() {
                if i > 0 {
                    result.push_str(", ");
                }
                // Recursively stringify strings/numbers inside the list cleanly
                if let RuntimeValue::String(s) = item {
                    // Wrap strings in quotes ONLY inside arrays to look like a real list
                    result.push_str(&format!("\"{}\"", s));
                } else {
                    result.push_str(&stringify_value(item));
                }
            }
            result.push(']');
            result
        }
        RuntimeValue::Error(s) => s.to_string(),
    }
}

// Uses the stringify helper to clean up terminal logs
fn native_print(args: &[RuntimeValue]) -> RuntimeValue {
    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            print!(" "); // Add spaces between multiple arguments
        }
        print!("{}", stringify_value(arg));
    }
    println!();
    let _ = io::stdout().flush();
    RuntimeValue::Nil
}

fn native_readline(_args: &[RuntimeValue]) -> RuntimeValue {
    let mut input = String::new();

    // Read the line from the standard input stream
    match io::stdin().read_line(&mut input) {
        Ok(_) => {
            // Rust's read_line keeps the trailing '\n' or '\r\n'.
            // We use .trim_end() to strip it off so it's a clean string.
            let clean_input = input.trim_end().to_string();
            RuntimeValue::String(Rc::new(clean_input))
        }
        Err(_) => {
            // Fallback placeholder if reading input fails (e.g., EOF reached)
            RuntimeValue::Nil
        }
    }
}

fn native_len(args: &[RuntimeValue]) -> RuntimeValue {
    match args.first() {
        Some(RuntimeValue::Array(list_ptr)) => RuntimeValue::Int(list_ptr.borrow().len() as i32),
        Some(RuntimeValue::String(s)) => RuntimeValue::Int(s.len() as i32),
        _ => error!("Function 'len()' expects an <array> or <string> argument."),
    }
}

fn native_split(args: &[RuntimeValue]) -> RuntimeValue {
    // split expects 2 arguments: split("target_string", "delimiter")
    if args.len() < 2 {
        return error!("Function 'split()' expects exactly 2 arguments (<string>, <string>).");
    }

    match (&args[0], &args[1]) {
        (RuntimeValue::String(target), RuntimeValue::String(delimiter)) => {
            // Split the target string by the delimiter and convert segments to RuntimeValue::String
            let segments: Vec<RuntimeValue> = target
                .split(delimiter.as_str())
                .map(|s| RuntimeValue::String(Rc::new(s.to_string())))
                .collect();

            RuntimeValue::Array(Rc::new(RefCell::new(segments)))
        }

        (RuntimeValue::String(_), _) => {
            error!("Second argument of function 'split()' must be a <string> type.")
        }

        (_, RuntimeValue::String(_)) => {
            error!("First argument of function 'split()' must be a <string> type.")
        }

        _ => error!("Both arguments of function 'split()' must be <string> types."),
    }
}

fn native_to_number(args: &[RuntimeValue]) -> RuntimeValue {
    match args.first() {
        Some(RuntimeValue::String(s)) => {
            // Attempt to parse the string into an f64 float number
            match s.parse::<f64>() {
                Ok(num) => {
                    if num.fract() == 0.0 {
                        RuntimeValue::Int(num as i32)
                    } else {
                        RuntimeValue::Float(num)
                    }
                }
                Err(_) => {
                    // if parsing fails (e.g. to_number("abc")), return
                    RuntimeValue::Int(0)
                }
            }
        }
        Some(RuntimeValue::Int(n)) => RuntimeValue::Int(*n), // Already a number, return it
        Some(RuntimeValue::Float(n)) => RuntimeValue::Float(*n), // Already a number, return it

        _ => error!("Function 'to_number()' expects an argument of <string> or <number> type."),
    }
}

fn native_panic(args: &[RuntimeValue]) -> RuntimeValue {
    if let Some(RuntimeValue::String(msg)) = args.first() {
        error!("Panic - ", msg)
    } else {
        error!("Panic - (No error message provided)")
    }
}

fn native_type(args: &[RuntimeValue]) -> RuntimeValue {
    match args.first() {
        Some(RuntimeValue::Int(_)) => RuntimeValue::String(Rc::new("int".to_string())),
        Some(RuntimeValue::Float(_)) => RuntimeValue::String(Rc::new("float".to_string())),
        Some(RuntimeValue::String(_)) => RuntimeValue::String(Rc::new("string".to_string())),
        Some(RuntimeValue::Boolean(_)) => RuntimeValue::String(Rc::new("boolean".to_string())),
        Some(RuntimeValue::Array(_)) => RuntimeValue::String(Rc::new("array".to_string())),
        Some(RuntimeValue::Nil) => RuntimeValue::String(Rc::new("nil".to_string())),
        Some(RuntimeValue::Error(_)) => RuntimeValue::String(Rc::new("error".to_string())),
        None => error!("Runtime Error: Function 'type()' expects 1 argument, found 0."),
    }
}

fn native_range(args: &[RuntimeValue]) -> RuntimeValue {
    if args.len() < 2 {
        return error!("Function 'range()' expects exactly 2 numeric arguments (start, end).");
    }
    match (&args[0], &args[1]) {
        (RuntimeValue::Int(start), RuntimeValue::Int(end)) => {
            let mut list = Vec::new();
            let mut current = *start;
            while current < *end {
                list.push(RuntimeValue::Int(current));
                current += 1;
            }
            RuntimeValue::Array(Rc::new(RefCell::new(list)))
        }
        _ => error!("Arguments to function 'range()' must be <number> types."),
    }
}

fn native_assert(args: &[RuntimeValue]) -> RuntimeValue {
    if args.is_empty() {
        return error!("Function 'assert()' expects at least 1 <bool> type argument.");
    }
    let assertion_passed = match &args[0] {
        RuntimeValue::Boolean(b) => *b,
        RuntimeValue::Nil => false,
        _ => true,
    };
    if !assertion_passed {
        if let Some(RuntimeValue::String(custom_msg)) = args.get(1) {
            error!("Assertion failed - ", custom_msg)
        } else {
            error!("Assertion failed - Condition evaluated to false.")
        }
    } else {
        RuntimeValue::Nil
    }
}

fn native_exit(args: &[RuntimeValue]) -> RuntimeValue {
    let exit_code = match args.first() {
        Some(RuntimeValue::Int(n)) => *n,
        _ => 0,
    };
    std::process::exit(exit_code);
}

fn native_chr(args: &[RuntimeValue]) -> RuntimeValue {
    match args.first() {
        Some(RuntimeValue::Int(n)) => {
            let code = *n as u32;
            match char::from_u32(code) {
                Some(c) => RuntimeValue::String(Rc::new(c.to_string())),
                None => error!(
                    "Function 'chr()' value ",
                    code, " is not a valid Unicode scalar."
                ),
            }
        }
        _ => error!("Function 'chr()' expects a Unicode <int> type argument."),
    }
}

fn native_ord(args: &[RuntimeValue]) -> RuntimeValue {
    match args.first() {
        Some(RuntimeValue::String(s)) => {
            if let Some(c) = s.chars().next() {
                RuntimeValue::Int(c as u32 as i32)
            } else {
                error!("Function 'ord()' cannot read an empty <string> layout frame.")
            }
        }
        _ => error!("Function 'ord()' expects a valid <string> context argument."),
    }
}

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};

// Helper to generate a raw random u64 using std's OS-seeded RandomState
fn next_random_u64() -> u64 {
    RandomState::new().build_hasher().finish()
}

fn native_randint(args: &[RuntimeValue]) -> RuntimeValue {
    if args.len() < 2 {
        return error!("Function 'randint()' expects 2 <int> arguments (min, max).");
    }

    match (&args[0], &args[1]) {
        (RuntimeValue::Int(min), RuntimeValue::Int(max)) => {
            if min > max {
                return error!("Function 'randint()' min value cannot be greater than max value.");
            }
            // Inclusive range [min, max]
            let span = (*max as i64 - *min as i64 + 1) as u64;
            let offset = (next_random_u64() % span) as i64;
            RuntimeValue::Int((*min as i64 + offset) as i32)
        }
        _ => error!("Arguments to function 'randint()' must be <int> types."),
    }
}

fn native_randflt(args: &[RuntimeValue]) -> RuntimeValue {
    match args.len() {
        // randflt() with 0 arguments -> returns a float in [0.0, 1.0)
        0 => {
            let val = (next_random_u64() >> 11) as f64 / ((1u64 << 53) as f64);
            RuntimeValue::Float(val)
        }
        // randflt(min, max) with 2 arguments -> returns a float in [min, max)
        2 => match (&args[0], &args[1]) {
            (RuntimeValue::Float(min), RuntimeValue::Float(max)) => {
                if min > max {
                    return error!(
                        "Function 'randflt()' min value cannot be greater than max value."
                    );
                }
                let unit = (next_random_u64() >> 11) as f64 / ((1u64 << 53) as f64);
                RuntimeValue::Float(min + unit * (max - min))
            }
            // Also allow Int bounds (e.g., randflt(1, 5))
            (RuntimeValue::Int(min), RuntimeValue::Int(max)) => {
                let (min_f, max_f) = (*min as f64, *max as f64);
                if min_f > max_f {
                    return error!(
                        "Function 'randflt()' min value cannot be greater than max value."
                    );
                }
                let unit = (next_random_u64() >> 11) as f64 / ((1u64 << 53) as f64);
                RuntimeValue::Float(min_f + unit * (max_f - min_f))
            }
            _ => error!("Arguments to function 'randflt()' must be <number> types."),
        },
        _ => error!(
            "Function 'randflt()' expects either 0 arguments or 2 <number> arguments (min, max)."
        ),
    }
}

fn native_int(args: &[RuntimeValue]) -> RuntimeValue {
    match args.first() {
        Some(RuntimeValue::Int(n)) => RuntimeValue::Int(*n),
        Some(RuntimeValue::Float(n)) => RuntimeValue::Int(*n as i32),
        Some(RuntimeValue::Boolean(b)) => RuntimeValue::Int(if *b { 1 } else { 0 }),
        Some(RuntimeValue::String(s)) => {
            let trimmed = s.trim();
            if let Ok(n) = trimmed.parse::<i32>() {
                RuntimeValue::Int(n)
            } else if let Ok(f) = trimmed.parse::<f64>() {
                RuntimeValue::Int(f as i32)
            } else {
                error!(
                    "Function 'int()' could not parse string '",
                    s, "' as an <int>."
                )
            }
        }
        _ => error!("Function 'int()' expects a <number>, <bool>, or numeric <string> argument."),
    }
}

fn native_float(args: &[RuntimeValue]) -> RuntimeValue {
    match args.first() {
        Some(RuntimeValue::Float(n)) => RuntimeValue::Float(*n),
        Some(RuntimeValue::Int(n)) => RuntimeValue::Float(*n as f64),
        Some(RuntimeValue::Boolean(b)) => RuntimeValue::Float(if *b { 1.0 } else { 0.0 }),
        Some(RuntimeValue::String(s)) => match s.trim().parse::<f64>() {
            Ok(f) => RuntimeValue::Float(f),
            Err(_) => error!(
                "Function 'float()' could not parse string '",
                s, "' as a <float>."
            ),
        },
        _ => error!("Function 'float()' expects a <number>, <bool>, or numeric <string> argument."),
    }
}

fn native_string(args: &[RuntimeValue]) -> RuntimeValue {
    match args.first() {
        Some(val) => RuntimeValue::String(Rc::new(stringify_value(val))),
        None => error!("Function 'string()' expects 1 argument, found 0."),
    }
}
