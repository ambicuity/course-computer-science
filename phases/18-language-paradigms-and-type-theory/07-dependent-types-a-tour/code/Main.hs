{-# LANGUAGE DataKinds, GADTs, KindSignatures #-}
module Main where

data Nat = Z | S Nat

data Vect (n :: Nat) a where
  VNil :: Vect 'Z a
  VCons :: a -> Vect n a -> Vect ('S n) a

headV :: Vect ('S n) a -> a
headV (VCons x _) = x

main :: IO ()
main = print (headV (VCons 1 (VCons 2 VNil)))
