module Main where

data Grade = Pass | Fail deriving (Eq, Show)

grade :: Int -> Grade
grade n = if n >= 60 then Pass else Fail

summary :: [Int] -> (Int, Int)
summary = foldr step (0, 0)
  where
    step s (p, f) = case grade s of
      Pass -> (p + 1, f)
      Fail -> (p, f + 1)

main :: IO ()
main = print (summary [90, 50, 70, 20, 88])
