module Main where

data Ty = TInt | TBool | TV String | TArr Ty Ty deriving (Eq, Show)

unifySimple :: (Ty, Ty) -> Either String ()
unifySimple (TInt, TInt) = Right ()
unifySimple (TBool, TBool) = Right ()
unifySimple (TV _, _) = Right ()
unifySimple (_, TV _) = Right ()
unifySimple (TArr a b, TArr c d) = do
  unifySimple (a, c)
  unifySimple (b, d)
unifySimple (x, y) = Left $ "cannot unify " ++ show x ++ " with " ++ show y

main :: IO ()
main = do
  print (unifySimple (TArr TInt TBool, TArr TInt TBool))
  print (unifySimple (TArr TInt TBool, TArr TBool TInt))
