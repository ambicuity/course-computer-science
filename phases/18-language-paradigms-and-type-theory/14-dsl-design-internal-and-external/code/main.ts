type Expr =
  | { tag: "lit"; n: number }
  | { tag: "add"; a: Expr; b: Expr }
  | { tag: "mul"; a: Expr; b: Expr };

function evalE(e: Expr): number {
  switch (e.tag) {
    case "lit":
      return e.n;
    case "add":
      return evalE(e.a) + evalE(e.b);
    case "mul":
      return evalE(e.a) * evalE(e.b);
  }
}

const program: Expr = { tag: "mul", a: { tag: "add", a: { tag: "lit", n: 2 }, b: { tag: "lit", n: 3 } }, b: { tag: "lit", n: 4 } };
console.log(evalE(program));
