module Main where

polyId :: a -> a
polyId x = x

polyMap :: (a -> b) -> [a] -> [b]
polyMap _ [] = []
polyMap f (x:xs) = f x : polyMap f xs

showTwice :: Show a => a -> String
showTwice x = show x ++ " | " ++ show x

main :: IO ()
main = do
  print (polyId 42)
  print (polyMap (+1) [1,2,3])
  putStrLn (showTwice True)
