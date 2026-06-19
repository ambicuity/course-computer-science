module Main where

data T = V String | L String T | A T T deriving (Eq, Show)

subst :: T -> String -> T -> T
subst (V n) x v = if n == x then v else V n
subst (L p b) x v = if p == x then L p b else L p (subst b x v)
subst (A f a) x v = A (subst f x v) (subst a x v)

step :: T -> T
step (A (L p b) a) = subst b p a
step t = t

main :: IO ()
main = print (step (A (L "x" (V "x")) (V "y")))
