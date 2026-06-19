//! Generics, traits, monomorphization vs trait objects.
//! Build: rustc -O main.rs -o m && ./m

use std::ops::Add;

// ── 1. Generic function ───────────────────────────────────────────

fn swap<T>(a: &mut T, b: &mut T) {
    std::mem::swap(a, b);
}

// ── 2. Trait with generic bound ──────────────────────────────────

trait Summable: Sized {
    fn zero() -> Self;
    fn plus(self, other: Self) -> Self;
}

impl Summable for i32 {
    fn zero() -> Self { 0 }
    fn plus(self, other: Self) -> Self { self + other }
}

impl Summable for f64 {
    fn zero() -> Self { 0.0 }
    fn plus(self, other: Self) -> Self { self + other }
}

fn sum<T: Summable + Copy>(xs: &[T]) -> T {
    let mut acc = T::zero();
    for &x in xs { acc = acc.plus(x); }
    acc
}

// ── 3. Trait + trait object (heterogeneous collection) ──────────

trait Shape {
    fn area(&self) -> f64;
    fn name(&self) -> &'static str;
}

struct Circle { r: f64 }
struct Square { s: f64 }
struct Triangle { base: f64, height: f64 }

impl Shape for Circle   { fn area(&self) -> f64 { std::f64::consts::PI * self.r * self.r } fn name(&self) -> &'static str { "Circle" } }
impl Shape for Square   { fn area(&self) -> f64 { self.s * self.s }                       fn name(&self) -> &'static str { "Square" } }
impl Shape for Triangle { fn area(&self) -> f64 { 0.5 * self.base * self.height }         fn name(&self) -> &'static str { "Triangle" } }

// Static dispatch (monomorphized at each call site)
fn area_static<S: Shape>(s: &S) -> f64 {
    s.area()
}

// Dynamic dispatch (vtable lookup at runtime)
fn area_dyn(s: &dyn Shape) -> f64 {
    s.area()
}

// ── 4. Operator overloading via std::ops::Add ───────────────────

#[derive(Debug, Clone, Copy)]
struct Vec2 { x: f64, y: f64 }

impl Add for Vec2 {
    type Output = Vec2;
    fn add(self, other: Vec2) -> Vec2 {
        Vec2 { x: self.x + other.x, y: self.y + other.y }
    }
}

// ── Demo ──────────────────────────────────────────────────────────

fn main() {
    println!("== Generic swap ==");
    let (mut a, mut b) = (1, 2);
    swap(&mut a, &mut b);
    println!("  i32: a={}, b={}", a, b);

    let (mut s1, mut s2) = (String::from("hello"), String::from("world"));
    swap(&mut s1, &mut s2);
    println!("  String: s1={:?}, s2={:?}", s1, s2);

    println!("\n== Generic sum with trait bound ==");
    println!("  sum::<i32>(&[1, 2, 3, 4, 5]) = {}", sum(&[1, 2, 3, 4, 5]));
    println!("  sum::<f64>(&[1.5, 2.5, 3.0]) = {}", sum(&[1.5_f64, 2.5, 3.0]));

    println!("\n== Static dispatch (monomorphization) ==");
    let c = Circle { r: 1.0 };
    let s = Square { s: 2.0 };
    println!("  area_static(&Circle{{r:1.0}}) = {:.4}", area_static(&c));
    println!("  area_static(&Square{{s:2.0}}) = {:.4}", area_static(&s));

    println!("\n== Trait object (heterogeneous Vec) ==");
    let shapes: Vec<Box<dyn Shape>> = vec![
        Box::new(Circle { r: 1.0 }),
        Box::new(Square { s: 2.0 }),
        Box::new(Triangle { base: 3.0, height: 4.0 }),
    ];
    let mut total = 0.0;
    for shape in &shapes {
        let a = area_dyn(shape.as_ref());
        println!("  {} area = {:.4}", shape.name(), a);
        total += a;
    }
    println!("  total area = {:.4}", total);

    println!("\n== Operator overloading: Vec2 + Vec2 ==");
    let v1 = Vec2 { x: 1.0, y: 2.0 };
    let v2 = Vec2 { x: 3.0, y: 4.0 };
    let v3 = v1 + v2;
    println!("  {:?} + {:?} = {:?}", v1, v2, v3);
}
