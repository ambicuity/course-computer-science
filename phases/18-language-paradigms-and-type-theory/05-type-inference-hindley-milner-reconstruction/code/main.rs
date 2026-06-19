#[derive(Clone, Debug, PartialEq, Eq)]
enum Ty {
    Int,
    Bool,
    Var(&'static str),
    Arr(Box<Ty>, Box<Ty>),
}

fn unify_simple(a: &Ty, b: &Ty) -> Result<(), String> {
    match (a, b) {
        (Ty::Int, Ty::Int) | (Ty::Bool, Ty::Bool) => Ok(()),
        (Ty::Var(_), _) | (_, Ty::Var(_)) => Ok(()),
        (Ty::Arr(a1, a2), Ty::Arr(b1, b2)) => {
            unify_simple(a1, b1)?;
            unify_simple(a2, b2)
        }
        _ => Err(format!("cannot unify {:?} with {:?}", a, b)),
    }
}

fn main() {
    let ok = Ty::Arr(Box::new(Ty::Int), Box::new(Ty::Bool));
    let bad = Ty::Arr(Box::new(Ty::Bool), Box::new(Ty::Int));
    println!("ok={:?}", unify_simple(&ok, &ok));
    println!("bad={:?}", unify_simple(&ok, &bad));
}
