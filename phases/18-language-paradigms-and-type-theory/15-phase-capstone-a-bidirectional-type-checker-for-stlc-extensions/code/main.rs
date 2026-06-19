use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Ty { Bool, Arr(Box<Ty>, Box<Ty>) }

#[derive(Clone, Debug)]
enum Tm {
    V(&'static str),
    Lam(&'static str, Box<Tm>),
    App(Box<Tm>, Box<Tm>),
    Ann(Box<Tm>, Ty),
    True,
}

type Ctx = HashMap<&'static str, Ty>;

fn infer(ctx: &Ctx, tm: &Tm) -> Result<Ty, String> {
    match tm {
        Tm::V(x) => ctx.get(x).cloned().ok_or_else(|| format!("unbound: {}", x)),
        Tm::True => Ok(Ty::Bool),
        Tm::Ann(t, ty) => { check(ctx, t, ty)?; Ok(ty.clone()) }
        Tm::App(f, a) => match infer(ctx, f)? {
            Ty::Arr(i, o) => { check(ctx, a, &i)?; Ok(*o) }
            _ => Err("apply non-function".into()),
        },
        Tm::Lam(_, _) => Err("lambda needs expected type".into()),
    }
}

fn check(ctx: &Ctx, tm: &Tm, ty: &Ty) -> Result<(), String> {
    match (tm, ty) {
        (Tm::Lam(x, body), Ty::Arr(i, o)) => {
            let mut c = ctx.clone(); c.insert(x, (*i.clone()).clone());
            check(&c, body, o)
        }
        _ => {
            let it = infer(ctx, tm)?;
            if &it == ty { Ok(()) } else { Err(format!("expected {:?}, got {:?}", ty, it)) }
        }
    }
}

fn main() {
    let id = Tm::Ann(Box::new(Tm::Lam("x", Box::new(Tm::V("x")))), Ty::Arr(Box::new(Ty::Bool), Box::new(Ty::Bool)));
    let app = Tm::App(Box::new(id), Box::new(Tm::True));
    println!("{:?}", infer(&Ctx::new(), &app));
}
