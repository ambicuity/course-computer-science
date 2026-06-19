# Rust Ownership — Cheat Sheet

## The Three Rules

1. **One owner** per value at any time.
2. **Drop on scope exit** — destructor runs automatically.
3. **Borrow exclusively OR share** — never both at once.

## Move vs Copy

```rust
// Non-Copy types MOVE on assignment / function call
let s1 = String::from("hi");
let s2 = s1;            // s1 invalid now
                        // String, Vec<T>, Box<T>, File: move

// Copy types COPY on assignment
let x: i32 = 5;
let y = x;              // both valid
                        // i32, f64, bool, char, fixed arrays: Copy
```

## Borrows

| Form | Effect |
|------|--------|
| `&T`     | Shared borrow; many allowed; T cannot be mutated |
| `&mut T` | Exclusive borrow; only one; T can be mutated |

```rust
let mut v = vec![1, 2, 3];
let r1 = &v;            // shared OK
let r2 = &v;            // another shared OK
// let r3 = &mut v;     // ERROR: cannot borrow mutably while shared exist
println!("{:?} {:?}", r1, r2);    // last use of r1, r2

let r3 = &mut v;        // now OK — shared borrows ended (NLL)
r3.push(4);
```

## Function Signatures Spell Ownership

```rust
fn take(s: String)       { /* s owned by callee */ }
fn borrow(s: &String)    { /* shared read */ }
fn modify(s: &mut String){ /* exclusive write */ }
```

Idiom: take `&T` unless you need to consume or modify.

## Three Errors to Recognize

### 1. Use after move
```rust
let s = String::from("hi");
let _ = s;
println!("{}", s);   // ERROR: borrow of moved value
```
Fix: `let _ = s.clone();` if you need a copy.

### 2. Mutable + shared simultaneously
```rust
let mut v = vec![1, 2, 3];
let first = &v[0];
v.push(4);
println!("{}", first);   // ERROR
```
Fix: shrink scopes or copy `*first` before mutating.

### 3. Dangling reference
```rust
fn f() -> &String {
    let s = String::from("hi");
    &s     // ERROR: s would be dropped before return
}
```
Fix: return owned `String` instead.

## Clone vs Copy

- `Copy` is automatic, bitwise, only for cheap types.
- `Clone` is explicit (`x.clone()`) and may be expensive (heap copy).
- Default to `&T` borrows; clone only when needed.

## Drop = RAII

```rust
struct File { fd: i32 }
impl Drop for File {
    fn drop(&mut self) { close(self.fd); }
}
{
    let f = File::open("...")?;
    // ... use f ...
}   // f.drop() runs here automatically
```

No manual `close`/`free` in user code. Ever.

## Internal Mutability (when you really need shared + mutating)

| Type | Single-threaded | Multi-threaded |
|------|-----------------|-----------------|
| Runtime-checked single-write | `RefCell<T>` | `Mutex<T>`, `RwLock<T>` |
| Atomic primitives | `Cell<T>` for Copy types | `AtomicUsize`, etc. |
| Reference-counted shared | `Rc<T>` | `Arc<T>` |

Default to plain `&T` / `&mut T`. Reach for these when the borrow checker can't see your invariants.
