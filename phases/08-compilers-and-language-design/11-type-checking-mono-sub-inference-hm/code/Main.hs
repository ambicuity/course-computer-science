-- Type Checking — Mono, Sub, Inference (HM)
-- Phase 08 — Compilers & Programming Language Design
--
-- Hindley-Milner type inference (Algorithm W) on a small lambda calculus.

module Main where

import Data.List (nub)
import qualified Data.Map as Map
import qualified Data.Set as Set

-- ── Types ──────────────────────────────────────────────────────────

data Type
  = TVar String
  | TInt
  | TBool
  | TFun Type Type
  deriving (Eq, Show)

data Scheme = Scheme [String] Type
  deriving (Show)

-- ── Substitution ──────────────────────────────────────────────────

type Subst = Map.Map String Type

class Types a where
  apply :: Subst -> a -> a
  ftv   :: a -> Set.Set String

instance Types Type where
  apply s (TVar n)    = case Map.lookup n s of
                          Just t  -> t
                          Nothing -> TVar n
  apply _ TInt        = TInt
  apply _ TBool       = TBool
  apply s (TFun a b)  = TFun (apply s a) (apply s b)

  ftv (TVar n)    = Set.singleton n
  ftv TInt        = Set.empty
  ftv TBool       = Set.empty
  ftv (TFun a b)  = ftv a `Set.union` ftv b

instance Types Scheme where
  apply s (Scheme vars t) = Scheme vars (apply (foldr Map.delete s vars) t)
  ftv (Scheme vars t)     = ftv t `Set.difference` Set.fromList vars

instance Types a => Types [a] where
  apply s = map (apply s)
  ftv     = Set.unions . map ftv

-- ── Type Environment ──────────────────────────────────────────────

type TypeEnv = Map.Map String Scheme

instance Types TypeEnv where
  apply s env = Map.map (apply s) env
  ftv env     = ftv (Map.elems env)

nullSubst :: Subst
nullSubst = Map.empty

composeSubst :: Subst -> Subst -> Subst
composeSubst s1 s2 = Map.union (Map.map (apply s1) s2) s1

-- ── Expression AST ────────────────────────────────────────────────

data Expr
  = EVar String
  | EInt Int
  | EBool Bool
  | ELam String Expr
  | EApp Expr Expr
  | ELet String Expr Expr
  deriving (Show)

-- ── Algorithm W ───────────────────────────────────────────────────

type Infer a = (Int, Either String (Subst, Type, a))

-- Fresh type variable generator
freshTVar :: State -> (Type, State)
freshTVar n = (TVar ("t" ++ show n), n + 1)

type State = Int

-- Instantiate a type scheme with fresh variables
instantiate :: Scheme -> State -> (Type, State)
instantiate (Scheme vars t) s =
  let (s', subst) = foldr (\v (st, m) ->
            let (tv, st') = freshTVar st
            in  (st', Map.insert v tv m))
          (s, Map.empty) vars
  in  (apply subst t, s')

-- Generalize: quantify over free type variables not in the environment
generalize :: TypeEnv -> Type -> Scheme
generalize env t =
  let vars = Set.toList (ftv t `Set.difference` ftv env)
  in  Scheme vars t

-- Unification
unify :: Type -> Type -> Either String Subst
unify TInt TInt       = Right Map.empty
unify TBool TBool     = Right Map.empty
unify (TFun a1 b1) (TFun a2 b2) = do
  s1 <- unify a1 a2
  s2 <- unify (apply s1 b1) (apply s1 b2)
  Right (composeSubst s2 s1)
unify (TVar n) t      = bindVar n t
unify t (TVar n)      = bindVar n t
unify t1 t2           = Left $ "cannot unify " ++ show t1 ++ " with " ++ show t2

bindVar :: String -> Type -> Either String Subst
bindVar n t
  | t == TVar n         = Right Map.empty
  | n `Set.member` ftv t = Left $ "occurs check fails: " ++ n ++ " in " ++ show t
  | otherwise           = Right (Map.singleton n t)

-- Algorithm W: infer type of expression
infer :: TypeEnv -> Expr -> State -> (Subst, Type, State)
infer env (EInt _)  s = (Map.empty, TInt, s)
infer env (EBool _) s = (Map.empty, TBool, s)

infer env (EVar x) s =
  case Map.lookup x env of
    Just scheme ->
      let (t, s') = instantiate scheme s
      in  (Map.empty, t, s')
    Nothing -> error $ "unbound variable: " ++ x

infer env (ELam x body) s =
  let (tv, s1) = freshTVar s
      env'     = Map.insert x (Scheme [] tv) env
      (s2, tbody, s2') = infer env' body s1
  in  (s2, TFun (apply s2 tv) tbody, s2')

infer env (EApp fn arg) s =
  let (s1, tfn, s1')  = infer env fn s
      (s2, targ, s2') = infer (apply s1 env) arg s1'
      (tv, s3)        = freshTVar s2'
      s3'             = case unify (apply s2 tfn) (TFun targ tv) of
                          Right su -> composeSubst su (composeSubst s2 s1)
                          Left e   -> error e
  in  (s3', apply s3' tv, s3)

infer env (ELet x def body) s =
  let (s1, tdef, s1') = infer env def s
      scheme          = generalize (apply s1 env) tdef
      env'            = Map.insert x scheme (apply s1 env)
      (s2, tbody, s2') = infer env' body s1'
  in  (composeSubst s2 s1, tbody, s2')

-- Convenience wrapper
typeInfer :: Expr -> Type
typeInfer expr =
  let (_, t, _) = infer Map.empty expr 0
  in  t

-- ── Pretty-print types ────────────────────────────────────────────

prettyType :: Type -> String
prettyType TInt       = "Int"
prettyType TBool      = "Bool"
prettyType (TVar n)   = n
prettyType (TFun a b) = parenArrow a ++ " -> " ++ prettyType b
  where
    parenArrow (TFun _ _) = "(" ++ prettyType a ++ ")"
    parenArrow _          = prettyType a

-- ── Demonstration ─────────────────────────────────────────────────

main :: IO ()
main = do
  putStrLn "=== Lesson 11: Type Checking — Hindley-Milner Inference ==="
  putStrLn ""

  -- 1. Simple integer
  let e1 = EInt 42
  putStrLn $ "  42 : " ++ prettyType (typeInfer e1)

  -- 2. Lambda: λx. x   (identity)
  let e2 = ELam "x" (EVar "x")
  putStrLn $ "  λx. x : " ++ prettyType (typeInfer e2)

  -- 3. Lambda: λx. λy. x   (const / K combinator)
  let e3 = ELam "x" (ELam "y" (EVar "x"))
  putStrLn $ "  λx. λy. x : " ++ prettyType (typeInfer e3)

  -- 4. Application: (λx. x) 42
  let e4 = EApp (ELam "x" (EVar "x")) (EInt 42)
  putStrLn $ "  (λx. x) 42 : " ++ prettyType (typeInfer e4)

  -- 5. Let-polymorphism: let id = λx. x in (id 1, ... simulate two uses)
  --    Since we don't have pairs, use id applied to different things sequentially
  let e5 = ELet "id" (ELam "x" (EVar "x"))
             (EApp (EVar "id") (EInt 1))
  putStrLn $ "  let id = λx. x in id 1 : " ++ prettyType (typeInfer e5)

  -- 6. Let-polymorphism with Bool
  let e6 = ELet "id" (ELam "x" (EVar "x"))
             (EApp (EVar "id") (EBool True))
  putStrLn $ "  let id = λx. x in id True : " ++ prettyType (typeInfer e6)

  -- 7. Nested let with inference
  let e7 = ELet "double" (ELam "f" (ELam "x" (EApp (EVar "f") (EApp (EVar "f") (EVar "x")))))
             (EApp (EApp (EVar "double") (ELam "n" (EVar "n"))) (EInt 5))
  putStrLn $ "  let double = λf. λx. f (f x) in double (λn. n) 5 : " ++ prettyType (typeInfer e7)

  putStrLn ""
  putStrLn "=== Done ==="
