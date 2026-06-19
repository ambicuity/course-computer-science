module Main where

-- Conceptual note: Haskell value usage is unrestricted by default;
-- linear types extension can enforce single-use contracts.

useOnce :: a -> (a, String)
useOnce x = (x, "used once conceptually")

main :: IO ()
main = print (snd (useOnce 42))
