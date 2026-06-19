module Main where

import qualified Data.Map as M

data Ty = TBool | TArr Ty Ty deriving (Eq, Show)
data Tm = TV String | TLam String Tm | TApp Tm Tm | TAnn Tm Ty | TTrue deriving (Eq, Show)
type Ctx = M.Map String Ty

infer :: Ctx -> Tm -> Either String Ty
infer g (TV x) = maybe (Left $ "unbound: " ++ x) Right (M.lookup x g)
infer _ TTrue = Right TBool
infer g (TAnn t ty) = do check g t ty; Right ty
infer g (TApp f a) = do
  tf <- infer g f
  case tf of
    TArr i o -> do check g a i; Right o
    _ -> Left "apply non-function"
infer _ (TLam _ _) = Left "lambda needs expected type"

check :: Ctx -> Tm -> Ty -> Either String ()
check g (TLam x body) (TArr i o) = check (M.insert x i g) body o
check g t ty = do
  it <- infer g t
  if it == ty then Right () else Left $ "expected " ++ show ty ++ ", got " ++ show it

main :: IO ()
main = do
  let idAnn = TAnn (TLam "x" (TV "x")) (TArr TBool TBool)
  print (infer M.empty idAnn)
  print (infer M.empty (TApp idAnn TTrue))
