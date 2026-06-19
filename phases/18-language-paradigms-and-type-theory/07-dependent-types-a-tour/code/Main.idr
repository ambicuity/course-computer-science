module Main

%default total

data Nat = Z | S Nat

data Vect : Nat -> Type -> Type where
  Nil  : Vect Z a
  (::) : a -> Vect n a -> Vect (S n) a

headV : Vect (S n) a -> a
headV (x :: _) = x

example : Int
example = headV (1 :: 2 :: Nil)

main : IO ()
main = putStrLn "dependent type demo"
