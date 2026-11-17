use crate::interpreter::RuntimeValue;
use std::collections::HashMap;

// The method type signature alias
pub type NativeMethodFn = fn(obj: &RuntimeValue, args: &[RuntimeValue]) -> RuntimeValue;

pub fn get_native_method_registry() -> HashMap<String, NativeMethodFn> {
    let mut registry: HashMap<String, NativeMethodFn> = HashMap::new();

    macro_rules! register {
        ($($name:expr => $func:expr),* $(,)?) => {
            $( registry.insert($name.to_string(), $func); )*
        };
    }

    register! {
        "append" => method_append,
        "get" => method_get,
        "remove" => method_remove,
        "clear" => method_clear,
        "expect" => method_expect,
        "is_numeric" => method_is_numeric,
        "pop" => method_pop,
        "trim" => method_trim,
        "starts_with" => method_starts_with,
        "ends_with" => method_ends_with,
        "join" => method_join,
        "is_in" => method_is_in,
        "contains" => method_contains,
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

///  compiler will call this to check if a method requires 'mut' permissions
pub fn is_mutating_method(name: &str) -> bool {
    match name {
        "append" | "clear" | "remove" | "pop" => true, // array manipulation
        _ => false,
    }
}

fn method_append(obj: &RuntimeValue, args: &[RuntimeValue]) -> RuntimeValue {
    match obj {
        RuntimeValue::Array(list_ptr) => {
            let append_value = args
                .first()
                .expect("Missing argument for .append() method.")
                .clone();
            list_ptr.borrow_mut().push(append_value);
            RuntimeValue::Nil
        }
        _ => error!("Method '.append()' can only be called on an <array> type."),
    }
}

fn method_get(obj: &RuntimeValue, args: &[RuntimeValue]) -> RuntimeValue {
    if let Some(RuntimeValue::Int(idx_int)) = args.first() {
        let idx = *idx_int as usize;

        match obj {
            RuntimeValue::Array(list_ptr) => {
                let list = list_ptr.borrow();
                if idx >= list.len() {
                    return error!(
                        "<array> index ",
                        idx,
                        " is out of bounds for length ",
                        list.len()
                    );
                }
                list[idx].clone()
            }
            RuntimeValue::String(source_str) => {
                if idx >= source_str.len() {
                    return error!(
                        "<string> index ",
                        idx,
                        " is out of bounds for length ",
                        source_str.len()
                    );
                }
                let target_char = source_str.chars().nth(idx).unwrap().to_string();
                RuntimeValue::String(target_char)
            }
            _ => error!("Method '.get()' can only be called on <array> or <string> types."),
        }
    } else {
        error!("Method '.get()' requires an <int> index argument.")
    }
}

// Pops an item out of an array by its position coordinate
fn method_remove(obj: &RuntimeValue, args: &[RuntimeValue]) -> RuntimeValue {
    if let Some(RuntimeValue::Int(idx_int)) = args.first() {
        let idx = *idx_int as usize;

        match obj {
            RuntimeValue::Array(list_ptr) => {
                let mut list = list_ptr.borrow_mut();
                if idx >= list.len() {
                    return error!(
                        "Method '.remove()' index ",
                        idx,
                        " is out of bounds for length ",
                        list.len()
                    );
                }
                list.remove(idx) // Returns the popped element back to your TCode stack
            }
            _ => error!("Method '.remove()' can only be called on an <array> type."),
        }
    } else {
        error!("Method '.remove()' requires an <int> index argument.")
    }
}

// Wipes an array completely
fn method_clear(obj: &RuntimeValue, _args: &[RuntimeValue]) -> RuntimeValue {
    match obj {
        RuntimeValue::Array(list_ptr) => {
            list_ptr.borrow_mut().clear();
            RuntimeValue::Nil
        }
        _ => error!("Method '.clear()' can only be called on an <array> type."),
    }
}

// Inline assertion fallback guard
fn method_expect(obj: &RuntimeValue, args: &[RuntimeValue]) -> RuntimeValue {
    let error_msg = match args.first() {
        Some(RuntimeValue::String(s)) => s.as_str(),
        _ => "Expected a non-nil value but execution failed.",
    };

    if let RuntimeValue::Nil | RuntimeValue::Boolean(false) = obj {
        error!("Expect Failed - ", error_msg)
    } else {
        obj.clone() // Passes the valid non-nil value straight forward
    }
}

fn method_is_numeric(obj: &RuntimeValue, _args: &[RuntimeValue]) -> RuntimeValue {
    match obj {
        // the .parse logic finds out if the string is numeric and returns a bool
        RuntimeValue::String(s) => RuntimeValue::Boolean(s.parse::<f64>().is_ok()),

        _ => error!("Method '.is_numeric()' can only be called on a <string> type."),
    }
}

fn method_pop(obj: &RuntimeValue, _args: &[RuntimeValue]) -> RuntimeValue {
    match obj {
        RuntimeValue::Array(list_ptr) => {
            let mut list = list_ptr.borrow_mut();
            list.pop().unwrap_or(RuntimeValue::Nil)
        }
        _ => error!("Method '.pop()' can only be called on an <array> type."),
    }
}

fn method_trim(obj: &RuntimeValue, _args: &[RuntimeValue]) -> RuntimeValue {
    match obj {
        RuntimeValue::String(source_str) => RuntimeValue::String(source_str.trim().to_string()),
        _ => error!("Method '.trim()' can only be called on a <string> type."),
    }
}

fn method_starts_with(obj: &RuntimeValue, args: &[RuntimeValue]) -> RuntimeValue {
    let Some(RuntimeValue::String(prefix)) = args.first() else {
        return error!("Method '.starts_with()' requires a <string> prefix argument.");
    };
    match obj {
        RuntimeValue::String(source_str) => RuntimeValue::Boolean(source_str.starts_with(prefix)),
        _ => {
            error!("Method '.starts_with()' can only be called on a <string> type.")
        }
    }
}

fn method_ends_with(obj: &RuntimeValue, args: &[RuntimeValue]) -> RuntimeValue {
    let Some(RuntimeValue::String(suffix)) = args.first() else {
        return error!("Method '.ends_with()' requires a <string> suffix argument.");
    };
    match obj {
        RuntimeValue::String(source_str) => RuntimeValue::Boolean(source_str.ends_with(suffix)),
        _ => error!("Method '.ends_with()' can only be called on a <string> type."),
    }
}

fn method_join(obj: &RuntimeValue, args: &[RuntimeValue]) -> RuntimeValue {
    let Some(RuntimeValue::String(separator)) = args.first() else {
        return error!("Method '.join()' requires a <string> separator argument.");
    };
    match obj {
        RuntimeValue::Array(list_ptr) => {
            let list = list_ptr.borrow();
            let mut result_string = String::new();
            for (i, item) in list.iter().enumerate() {
                if i > 0 {
                    result_string.push_str(separator);
                }
                match item {
                    RuntimeValue::String(s) => result_string.push_str(s),
                    RuntimeValue::Int(n) => result_string.push_str(&n.to_string()),
                    RuntimeValue::Float(n) => result_string.push_str(&n.to_string()),
                    RuntimeValue::Boolean(b) => result_string.push_str(&b.to_string()),
                    RuntimeValue::Nil => result_string.push_str("nil"),
                    _ => result_string.push_str("[object]"),
                }
            }
            RuntimeValue::String(result_string)
        }
        _ => error!("Method '.join()' can only be called on an <array> type."),
    }
}

// helper function for methods is_in() and contains()
fn values_equal(a: &RuntimeValue, b: &RuntimeValue) -> bool {
    match (a, b) {
        (RuntimeValue::Int(x), RuntimeValue::Int(y)) => x == y,
        (RuntimeValue::Float(x), RuntimeValue::Float(y)) => x == y,
        (RuntimeValue::String(x), RuntimeValue::String(y)) => x == y,
        (RuntimeValue::Boolean(x), RuntimeValue::Boolean(y)) => x == y,
        (RuntimeValue::Nil, RuntimeValue::Nil) => true,
        _ => false,
    }
}

fn method_is_in(obj: &RuntimeValue, args: &[RuntimeValue]) -> RuntimeValue {
    let Some(RuntimeValue::Array(list_ptr)) = args.first() else {
        return error!("Method '.is_in()' requires an <array> argument.");
    };

    let list = list_ptr.borrow();
    let found = list.iter().any(|item| values_equal(obj, item));
    RuntimeValue::Boolean(found)
}

fn method_contains(obj: &RuntimeValue, args: &[RuntimeValue]) -> RuntimeValue {
    let Some(target) = args.first() else {
        return error!("Method '.contains()' requires an argument.");
    };

    match obj {
        RuntimeValue::String(source_str) => {
            if let RuntimeValue::String(search_target) = target {
                RuntimeValue::Boolean(source_str.contains(search_target))
            } else {
                error!("Method '.contains()' on a <string> requires a <string> target argument.")
            }
        }
        RuntimeValue::Array(list_ptr) => {
            let list = list_ptr.borrow();
            let found = list.iter().any(|item| values_equal(item, target));
            RuntimeValue::Boolean(found)
        }
        _ => error!("Method '.contains()' can only be called on <string> or <array> types."),
    }
}
