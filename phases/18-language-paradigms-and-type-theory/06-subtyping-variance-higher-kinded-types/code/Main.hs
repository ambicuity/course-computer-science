module Main where

class FunctorLike f where
  fmapLike :: (a -> b) -> f a -> f b

instance FunctorLike [] where
  fmapLike = map

main :: IO ()
main = print (fmapLike (+1) [1,2,3])
