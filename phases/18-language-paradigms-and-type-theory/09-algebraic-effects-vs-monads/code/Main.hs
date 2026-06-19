module Main where

import Control.Monad (when)

eval :: Int -> Int -> Either String (Int, [String])
eval x y = do
  when (y == 0) (Left "division by zero")
  let q = x `div` y
  Right (q, ["computed " ++ show x ++ " / " ++ show y])

main :: IO ()
main = do
  print (eval 10 2)
  print (eval 10 0)
