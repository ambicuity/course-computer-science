# OOP Deep — Inheritance, Composition, Mixins, Traits

> Reuse behavior intentionally; inheritance is one tool, not default architecture.

**Type:** Learn
**Languages:** TypeScript, Rust
**Prerequisites:** Phase 18 lessons 01-10
**Time:** ~60 minutes

## Learning Objectives

- Compare inheritance and composition tradeoffs concretely.
- Understand mixin-style and trait-style reuse.
- Choose extension mechanisms that minimize coupling.
- Identify fragile-base-class risks early.

## The Problem

A team builds a game. The class hierarchy starts clean: `Entity → Character → Player`. Then someone adds `FlyingCharacter`, `SwimmingCharacter`, and `FlyingSwimmingCharacter`. The hierarchy explodes into a diamond. Changing `Character` breaks three subclasses. Adding a new ability (e.g., climbing) means touching every branch.

This is the fragile base class problem: changes to a parent class cascade unpredictably to children. Deep inheritance trees create hidden coupling. The parent's implementation details become the child's assumptions. Refactoring becomes risky because the blast radius is unbounded.

Composition fixes this by assembling behavior from small, independent components. Instead of inheriting `Flying` from a base class, you give a character a `FlyingBehavior` component. Want flying and swimming? Attach both. Want to remove flying at runtime? Remove the component. No hierarchy changes, no fragile base class.

## The Concept

### Inheritance: hierarchical reuse

```
        Animal
       /      \
    Dog      Cat
   /    \
Husky  Poodle
```

Inheritance models "is-a" relationships. A `Dog` is an `Animal`. The child inherits the parent's interface and implementation.

```typescript
class Animal {
  move() { console.log("moving"); }
}

class Dog extends Animal {
  bark() { console.log("woof"); }
}

class Husky extends Dog {
  pull() { console.log("pulling sled"); }
}
```

**Strengths**: simple for stable hierarchies, polymorphism via subtype, familiar to most developers.

**Weaknesses**: deep trees create coupling, the fragile base class problem, "is-a" is subjective and can change, multiple inheritance creates diamond problems.

### Composition: assemble collaborators

```
  Character ──has-a──→ MovementBehavior
       │──has-a──→ AttackBehavior
       └──has-a──→ RenderBehavior
```

Composition models "has-a" relationships. A character has behaviors, not is-a behavior.

```typescript
interface MovementBehavior {
  move(): void;
}

class WalkBehavior implements MovementBehavior {
  move() { console.log("walking"); }
}

class FlyBehavior implements MovementBehavior {
  move() { console.log("flying"); }
}

class Character {
  constructor(private movement: MovementBehavior) {}
  act() { this.movement.move(); }
}

// Swap behaviors at runtime
const walker = new Character(new WalkBehavior());
const flyer = new Character(new FlyBehavior());
```

**Strengths**: flexible, no deep coupling, behaviors are independently testable, can change at runtime.

**Weaknesses**: more boilerplate, no implicit polymorphism, requires explicit wiring.

### Mixins: composable behavior units

Mixins add behavior to a class without inheritance:

```typescript
// TypeScript mixin pattern
type Constructor<T = {}> = new (...args: any[]) => T;

function Serializable<T extends Constructor>(Base: T) {
  return class extends Base {
    serialize() { return JSON.stringify(this); }
  };
}

function Loggable<T extends Constructor>(Base: T) {
  return class extends Base {
    log() { console.log(this); }
  };
}

class User {
  constructor(public name: string) {}
}

// Compose behaviors
const SuperUser = Serializable(Loggable(User));
const u = new SuperUser("Alice");
u.serialize();  // works
u.log();        // works
```

### Traits: contracts with optional defaults

Traits are like interfaces but can carry implementation:

```rust
trait Drawable {
    fn draw(&self);
    fn bounding_box(&self) -> Rect;  // required
    fn color(&self) -> Color {       // default implementation
        Color::Black
    }
}

trait Clickable {
    fn on_click(&self, pos: Point);
}

// Combine traits
struct Button {
    label: String,
    rect: Rect,
}

impl Drawable for Button {
    fn draw(&self) { /* ... */ }
    fn bounding_box(&self) -> Rect { self.rect }
}

impl Clickable for Button {
    fn on_click(&self, _pos: Point) { /* ... */ }
}
```

**Trait vs interface**: traits can have default implementations and can be implemented for types you don't own. Interfaces in Java/TypeScript require the type to opt in.

### The comparison

| Mechanism | Coupling | Flexibility | Polymorphism | Runtime change |
|-----------|----------|------------|--------------|----------------|
| Inheritance | High | Low | Subtype | No |
| Composition | Low | High | Interface | Yes |
| Mixins | Medium | Medium | Structural | No |
| Traits | Low | High | Structural | No |

### SOLID principles (revisited)

| Principle | Inheritance risk | Composition solution |
|-----------|-----------------|---------------------|
| Single Responsibility | Base class accumulates behaviors | Each component has one job |
| Open/Closed | Modifying base breaks children | Add new components without changing existing |
| Liskov Substitution | Subclasses may violate expectations | Components satisfy contracts independently |
| Interface Segregation | Fat base class forces unused methods | Small, focused interfaces per component |
| Dependency Inversion | Children depend on parent implementation | Components depend on abstractions |

## Build It

### Step 1: TypeScript composition-first

```typescript
// Define small, focused interfaces
interface Renderer {
  render(): string;
}

interface Serializer {
  serialize(): string;
}

// Compose behaviors into a class
class Widget implements Renderer, Serializer {
  constructor(
    private name: string,
    private value: number
  ) {}

  render(): string {
    return `${this.name}: ${this.value}`;
  }

  serialize(): string {
    return JSON.stringify({ name: this.name, value: this.value });
  }
}

// Add new behavior without modifying Widget
function withLogging<T extends Renderer>(widget: T): T & { log: () => void } {
  return {
    ...widget,
    log() { console.log(this.render()); }
  };
}

const w = withLogging(new Widget("temp", 42));
w.log();  // "temp: 42"
```

### Step 2: Rust trait composition

```rust
trait Summary {
    fn summarize(&self) -> String;

    // Default implementation
    fn preview(&self) -> String {
        let s = self.summarize();
        if s.len() > 100 {
            format!("{}...", &s[..100])
        } else {
            s
        }
    }
}

trait Formattable {
    fn format(&self) -> String;
}

struct Article {
    title: String,
    content: String,
}

impl Summary for Article {
    fn summarize(&self) -> String {
        format!("{}: {}", self.title, self.content)
    }
}

impl Formattable for Article {
    fn format(&self) -> String {
        format!("# {}\n\n{}", self.title, self.content)
    }
}

// Generic function requiring both traits
fn display<T: Summary + Formattable>(item: &T) {
    println!("{}", item.format());
    println!("Summary: {}", item.summarize());
}
```

### Step 3: Compare extension friction

```typescript
// Adding a new capability with inheritance: modify the tree
// Adding a new capability with composition: add a new interface/behavior

// Before: Widget renders and serializes
// After: Widget also validates

// Inheritance: create ValidatingWidget extends Widget
// Composition: add a Validator interface and compose it in

interface Validator {
  validate(): boolean;
}

class ValidatedWidget extends Widget implements Validator {
  validate(): boolean {
    return this.value > 0;
  }
}
// Or: just add validation as a standalone function
function validateWidget(w: Widget): boolean {
  return true; // validation logic
}
```

## Use It

Most large codebases mix patterns:

- **React**: composition via components and hooks. Inheritance is discouraged.
- **Rust standard library**: traits everywhere (`Display`, `Debug`, `Clone`, `Iterator`).
- **Go**: interfaces for capability contracts, composition via embedding.
- **Java**: inheritance for domain taxonomies, interfaces for capability contracts.
- **Game engines**: Entity-Component-System (ECS) is pure composition.

The trend in modern API design is toward composition and interfaces. Inheritance remains useful for stable, well-understood hierarchies (e.g., AST node types in a compiler).

## Read the Source

- *Design Patterns* (GoF) — the original patterns book, heavy on composition.
- *Effective Java* (Bloch) — Item 18: "Favor composition over inheritance."
- *Rust Programming Language*, Chapter 10: Traits.
- [Refactoring Guru: Composition Over Inheritance](https://refactoring.guru/design-patterns/composition-over-inheritance).

## Ship It

- `code/main.ts`: composition and mixin-style sample.
- `code/main.rs`: trait-based behavior composition.
- `outputs/README.md`: reuse-pattern decision checklist.

## Quiz

**Q1 (Pre).** What is the fragile base class problem?

- A) Base classes are always slow.
- B) Changes to a parent class can break child classes in unexpected ways.
- C) Base classes can't be modified.
- D) Child classes can't override parent methods.

**Answer: B.** When a child class depends on the parent's implementation details (not just its interface), changing the parent can break the child. The deeper the hierarchy, the more fragile it becomes. Composition avoids this by not having parent-child implementation coupling.

**Q2 (Pre).** How do traits differ from inheritance?

- A) Traits are slower.
- B) Traits define capability contracts that can be implemented by unrelated types, without creating a hierarchy.
- C) Traits require subclassing.
- D) Traits can't have default implementations.

**Answer: B.** A trait like `Drawable` can be implemented by `Circle`, `Button`, and `Document`, none of which share a common ancestor. Traits are about capability ("can draw"), not taxonomy ("is a shape"). Rust traits can have default implementations.

**Q3 (Post).** When is inheritance the right choice?

- A) Always.
- B) When the hierarchy models a genuine, stable "is-a" relationship and won't need runtime flexibility.
- C) Never.
- D) Only for UI components.

**Answer: B.** Inheritance works well for stable domain taxonomies (AST nodes, exception hierarchies, collection types). The key test: does the "is-a" relationship hold in all future scenarios? If the hierarchy might change, prefer composition.

**Q4 (Post).** What's the key advantage of composition over inheritance?

- A) It's faster.
- B) It decouples behaviors so you can change, add, or remove them independently.
- C) It requires less code.
- D) It's the only way to achieve polymorphism.

**Answer: B.** Composition lets you swap behaviors at runtime, test them in isolation, and add new behaviors without modifying existing classes. The coupling is at the interface level, not the implementation level. This makes refactoring safer and the system more flexible.

**Q5 (Post).** How do mixins relate to composition and inheritance?

- A) They're identical to inheritance.
- B) They're a form of composition that adds behavior to a class without creating a deep hierarchy.
- C) They replace interfaces.
- D) They only work in JavaScript.

**Answer: B.** Mixins compose behavior by adding methods to a class without establishing an "is-a" parent-child relationship. TypeScript's mixin pattern uses higher-order functions to add capabilities. They sit between inheritance (hierarchical) and composition (explicit wiring) in terms of coupling.

## Exercises

1. **Easy.** Refactor a deep class tree branch (3+ levels) into composed collaborators. Show the before and after.
2. **Medium.** Add a new behavior (e.g., caching) to an existing class using a trait/mixin without modifying the original class.
3. **Hard.** Document the compatibility guarantees for each extension point in a codebase you maintain. Which are stable? Which are fragile?

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Inheritance | "reuse" | Behavioral extension via subtype hierarchy |
| Composition | "has-a" | Reuse by assembling collaborating objects/components |
| Trait | "interface+default" | Capability contract optionally with shared behavior implementation |
| Mixin | "behavior module" | Composable behavior unit added to a type without hierarchy |
| Fragile base class | "parent broke my code" | Changes to a parent class cascade unexpectedly to children |

## Further Reading

- [Refactoring Guru: Composition Over Inheritance](https://refactoring.guru/design-patterns/composition-over-inheritance)
- [Rust Traits](https://doc.rust-lang.org/book/ch10-02-traits.html)
- *Effective Java* (Bloch), Item 18
- [Mixins in TypeScript](https://www.typescriptlang.org/docs/handbook/mixins.html)
