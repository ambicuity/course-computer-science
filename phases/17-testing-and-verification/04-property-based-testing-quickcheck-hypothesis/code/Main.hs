module Main where

import Data.List (sort)
import Test.QuickCheck

buggySort :: [Int] -> [Int]
buggySort xs =
  case sort xs of
    (a:b:rest) | a == b -> b : rest
    ys -> ys

prop_sortedness :: [Int] -> Bool
prop_sortedness xs =
  let ys = buggySort xs
   in and $ zipWith (<=) ys (drop 1 ys)

prop_idempotent :: [Int] -> Bool
prop_idempotent xs = buggySort (buggySort xs) == buggySort xs

prop_lengthPreserved :: [Int] -> Bool
prop_lengthPreserved xs = length (buggySort xs) == length xs

main :: IO ()
main = do
  quickCheck prop_sortedness
  quickCheck prop_idempotent
  quickCheck prop_lengthPreserved
