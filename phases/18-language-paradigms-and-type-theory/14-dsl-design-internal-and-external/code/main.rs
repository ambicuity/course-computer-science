#[derive(Clone, Debug)]
enum Expr {
    Lit(i64),
    Add(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
}

fn eval(e: &Expr) -> i64 {
    match e {
        Expr::Lit(n) => *n,
        Expr::Add(a, b) => eval(a) + eval(b),
        Expr::Mul(a, b) => eval(a) * eval(b),
    }
}

fn main() {
    let p = Expr::Mul(
        Box::new(Expr::Add(Box::new(Expr::Lit(2)), Box::new(Expr::Lit(3)))),
        Box::new(Expr::Lit(4)),
    );
    println!("{}", eval(&p));
}
