-- Main.hs — persistent BST and list in Haskell.
-- In Haskell, ALL data is persistent by default. There's no "mutate".

module Main where

data BST = Leaf | Node Int BST BST deriving Show

insert :: Int -> BST -> BST
insert k Leaf = Node k Leaf Leaf
insert k t@(Node v l r)
  | k < v     = Node v (insert k l) r
  | k > v     = Node v l (insert k r)
  | otherwise = t

contains :: Int -> BST -> Bool
contains _ Leaf = False
contains k (Node v l r)
  | k < v     = contains k l
  | k > v     = contains k r
  | otherwise = True

count :: BST -> Int
count Leaf = 0
count (Node _ l r) = 1 + count l + count r

main :: IO ()
main = do
  let t1 = foldr insert Leaf [0, 10, 20, 30, 40, 50, 60, 70]
  let t2 = insert 25 t1
  putStrLn $ "t1 nodes: " ++ show (count t1)
  putStrLn $ "t2 nodes: " ++ show (count t2)
  putStrLn $ "t1 contains 25: " ++ show (contains 25 t1)
  putStrLn $ "t2 contains 25: " ++ show (contains 25 t2)
  putStrLn "Both trees coexist; t1 unchanged after producing t2"
