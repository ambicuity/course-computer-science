module Main where

import qualified Data.Map as M

data Type = TBool | TArr Type Type deriving (Eq, Show)
data Term = TVar String | TLam String Type Term | TApp Term Term | TTrue deriving (Eq, Show)
type Ctx = M.Map String Type

typeOf :: Ctx -> Term -> Either String Type
typeOf g (TVar x) = maybe (Left $ "unbound: " ++ x) Right (M.lookup x g)
typeOf g (TLam x ty body) = do
  rt <- typeOf (M.insert x ty g) body
  pure (TArr ty rt)
typeOf g (TApp f a) = do
  tf <- typeOf g f
  ta <- typeOf g a
  case tf of
    TArr i o | i == ta -> Right o
             | otherwise -> Left "argument type mismatch"
    _ -> Left "attempted to apply non-function"
typeOf _ TTrue = Right TBool

main :: IO ()
main = do
  let idBool = TLam "x" TBool (TVar "x")
  print (typeOf M.empty idBool)
  print (typeOf M.empty (TApp idBool TTrue))
