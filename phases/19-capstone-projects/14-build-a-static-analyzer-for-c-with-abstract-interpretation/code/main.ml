type interval = { lo : int; hi : int }

let top = { lo = min_int / 4; hi = max_int / 4 }
let constant v = { lo = v; hi = v }
let add_const i c = { lo = i.lo + c; hi = i.hi + c }
let join a b = { lo = min a.lo b.lo; hi = max a.hi b.hi }
let may_be_zero i = i.lo <= 0 && 0 <= i.hi

let () =
  let x0 = constant (-1) in
  let x1 = add_const x0 2 in
  let merged = join x1 { lo = 0; hi = 3 } in
  if may_be_zero merged then
    Printf.printf "warning: possible divide-by-zero, x in [%d, %d]\n" merged.lo merged.hi
  else
    Printf.printf "safe division, x in [%d, %d]\n" merged.lo merged.hi
