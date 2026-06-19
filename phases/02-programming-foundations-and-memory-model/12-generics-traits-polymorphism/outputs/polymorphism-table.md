# Polymorphism Across Languages

Three flavors of polymorphism, five languages.

## Parametric (one definition, many types)

| Language | Syntax | Dispatch |
|----------|--------|----------|
| Rust | `fn f<T>(x: T)` | Monomorphized at compile time |
| C++  | `template <typename T> void f(T x);` | Monomorphized at compile time |
| Java | `<T> void f(T x)` | Type-erased (object reference at runtime; primitives autoboxed) |
| Go (1.18+) | `func f[T any](x T)` | Monomorphized with some shared instantiations |
| Haskell | `f :: a -> a` | Type-erased at runtime; specialization optional |

## Ad-hoc (different code per type, dispatched by behavior)

| Language | Mechanism | Static or dynamic? |
|----------|-----------|--------------------|
| Rust | `trait` + `impl ... for ...` | Static (`<T: Trait>`) or dynamic (`dyn Trait`) — your choice |
| C++  | `concept` (C++20) + overload resolution; `virtual` for vtable | Templates static; virtual dynamic |
| Java | `interface` + `class implements` | Always dynamic (vtable) |
| Go   | `interface` (structurally satisfied) | Always dynamic |
| Haskell | `class` / `instance` (type class) | Dictionary-passing under the hood |

## Subtype (inheritance / class hierarchy)

| Language | Has it? | Notes |
|----------|---------|-------|
| Rust  | No | Use trait objects; composition over inheritance |
| C++   | Yes — `class Derived : public Base` | Multiple inheritance, virtual inheritance, etc. |
| Java  | Yes — single inheritance; multiple interfaces | Final classes possible |
| Go    | No (only struct embedding + interfaces) | "Promotion" of embedded fields/methods |
| Haskell | No (type classes do the job) | |

## When to use each

| Need | Use |
|------|-----|
| One algorithm, every primitive type | Generic / template |
| Add behavior to existing types (esp. types you don't own) | Trait / type class |
| Heterogeneous collection at runtime | Trait object / interface / virtual base |
| Specialization for performance | Generic + monomorphization |
| Stable ABI / plugin loading | Trait object / virtual |

## Cost summary

| Mechanism | Compile time | Runtime |
|-----------|--------------|---------|
| Generic (monomorphized) | High (per-type code) | Zero — inlines like hand-written |
| Trait object / virtual call | Low | One indirection (~2 cycles) |
| Java generics (erased) | Low | Boxing overhead for primitives |
