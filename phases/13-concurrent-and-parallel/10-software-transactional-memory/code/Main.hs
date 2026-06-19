{-
  Software Transactional Memory — Haskell Examples
  =================================================
  All examples use Control.Concurrent.STM (GHC's built-in implementation).
  Compile:  ghc Main.hs -o stm_examples
  Run:      ./stm_examples

  Covers:
    - TVar basics with atomically
    - retry for blocking
    - orElse for fallback
    - Bank account transfer (composable)
    - TChan producer-consumer
    - TMVar for one-shot synchronization
-}

module Main where

import Control.Concurrent (forkIO, threadDelay, myThreadId)
import Control.Concurrent.STM
import Control.Monad (forever, replicateM_, void)
import System.IO (hFlush, stdout)

-- ===================================================================
-- Example 1: Concurrent Counter (TVar basics)
-- ===================================================================

counterExample :: IO ()
counterExample = do
  putStrLn "\n=== Example 1: Concurrent Counter ==="
  counter <- newTVarIO (0 :: Int)
  let workers = 10
      iterations = 1000

  -- Fork workers, each incrementing the counter
  threads <- replicateM workers . forkIO $
    replicateM_ iterations . atomically $
      modifyTVar' counter (+1)

  putStrLn $ "Spawned " ++ show workers ++ " threads, each doing "
          ++ show iterations ++ " increments."
  threadDelay 500000

  result <- atomically $ readTVar counter
  putStrLn $ "Final counter value: " ++ show result
  putStrLn $ "Expected:            " ++ show (workers * iterations)

-- ===================================================================
-- Example 2: retry — Blocking until a condition is met
-- ===================================================================

retryExample :: IO ()
retryExample = do
  putStrLn "\n=== Example 2: retry — Blocking ==="
  flag <- newTVarIO False

  -- Fork a thread that sets flag to True after 1 second
  _ <- forkIO $ do
    threadDelay 1000000
    atomically $ writeTVar flag True
    putStrLn "  [worker] Flag set to True"

  -- Wait for flag to become True using retry
  putStrLn "  [main] Waiting for flag (via retry)..."
  atomically $ do
    val <- readTVar flag
    if val then return () else retry
  putStrLn "  [main] Done waiting!"

-- ===================================================================
-- Example 3: orElse — Fallback on retry
-- ===================================================================

orElseExample :: IO ()
orElseExample = do
  putStrLn "\n=== Example 3: orElse — Fallback ==="
  emptyChan <- newTChanIO
  fullChan <- newTChanIO

  -- Write something to fullChan
  atomically $ writeTChan fullChan "hello from fullChan"

  -- Try to read from emptyChan first; fall back to fullChan
  msg <- atomically $ readTChan emptyChan `orElse` readTChan fullChan
  putStrLn $ "  Read: " ++ show msg

-- ===================================================================
-- Example 4: Bank Account Transfer (Composable STM)
-- ===================================================================

type Account = TVar Int

createAccount :: Int -> IO Account
createAccount balance = newTVarIO balance

-- | Withdraw 'amount' from account. Blocks (retry) if insufficient funds.
withdraw :: Account -> Int -> STM ()
withdraw acc amount = do
  bal <- readTVar acc
  if bal < amount
    then retry
    else writeTVar acc (bal - amount)

-- | Deposit 'amount' into account.
deposit :: Account -> Int -> STM ()
deposit acc amount = modifyTVar' acc (+ amount)

-- | Transfer 'amount' from one account to another.
--   This COMPOSES withdraw and deposit atomically!
--   The entire transfer is one transaction — no lock ordering issues,
--   no deadlock, no intermediate state visible to other threads.
transfer :: Account -> Account -> Int -> STM ()
transfer fromAcc toAcc amount = do
  withdraw fromAcc amount
  deposit toAcc amount

-- | Read balance inside STM.
getBalance :: Account -> STM Int
getBalance = readTVar

-- | Print balance (in IO, but reads balance atomically).
printBalance :: String -> Account -> IO ()
printBalance label acc = do
  bal <- atomically $ getBalance acc
  putStrLn $ "  " ++ label ++ ": " ++ show bal

-- | Try to transfer; if insufficient funds in checking, try credit.
--   If both fail, return False (no-op).
tryPay :: Account -> Account -> Int -> STM Bool
tryPay checking credit amount =
      (withdraw checking amount >> return True)
  `orElse`
      (withdraw credit amount >> return True)
  `orElse`
      return False

bankTransferExample :: IO ()
bankTransferExample = do
  putStrLn "\n=== Example 4: Bank Account Transfer ==="

  alice <- createAccount 1000
  bob   <- createAccount 500

  putStrLn "Initial balances:"
  printBalance "Alice" alice
  printBalance "Bob"   bob

  -- Spawn 5 concurrent transfers of 100 each from Alice to Bob
  replicateM_ 5 . forkIO $ atomically $ transfer alice bob 100

  -- Give threads time to finish
  threadDelay 500000

  putStrLn "\nAfter 5x $100 transfers (Alice -> Bob):"
  aBal <- atomically $ getBalance alice
  bBal <- atomically $ getBalance bob
  printBalance "Alice" alice
  printBalance "Bob"   bob
  putStrLn $ "  Alice expected: 500, Bob expected: 1000"

-- ===================================================================
-- Example 5: tryPay with orElse fallback
-- ===================================================================

orElsePayExample :: IO ()
orElsePayExample = do
  putStrLn "\n=== Example 5: orElse Payment Fallback ==="

  checking <- createAccount 50
  credit   <- createAccount 5000

  putStrLn "Initial:"
  printBalance "Checking" checking
  printBalance "Credit"   credit

  -- Try to pay $100 — insufficient in checking, falls back to credit
  paid <- atomically $ tryPay checking credit 100
  putStrLn $ "  Payment of $100 succeeded? " ++ show paid

  putStrLn "\nAfter payment:"
  printBalance "Checking" checking
  printBalance "Credit"   credit

-- ===================================================================
-- Example 6: TChan — Producer-Consumer
-- ===================================================================

tchanExample :: IO ()
tchanExample = do
  putStrLn "\n=== Example 6: TChan Producer-Consumer ==="
  chan <- newTChanIO
  count <- newTVarIO (0 :: Int)
  let numItems = 10

  -- Producer: write items to channel
  _ <- forkIO $ do
    replicateM_ numItems $ do
      atomically $ do
        writeTChan chan "hello"
        modifyTVar' count (+1)
      threadDelay 50000
    putStrLn "  [producer] Done producing"

  -- Consumer: read items from channel
  _ <- forkIO $ do
    replicateM_ numItems $ do
      msg <- atomically $ readTChan chan
      threadDelay 100000  -- slower consumer
    cnt <- atomically $ readTVar count
    putStrLn $ "  [consumer] Done consuming. total: " ++ show cnt

  threadDelay 2000000
  cnt <- atomically $ readTVar count
  putStrLn $ "  Final count: " ++ show cnt

-- ===================================================================
-- Example 7: TMVar — One-shot synchronization
-- ===================================================================

tmvarExample :: IO ()
tmvarExample = do
  putStrLn "\n=== Example 7: TMVar ==="
  tmvar <- newTMVarIO (0 :: Int)

  _ <- forkIO $ do
    tid <- myThreadId
    putStrLn $ "  [" ++ show tid ++ "] Waiting for value..."
    val <- atomically $ takeTMVar tmvar
    putStrLn $ "  [" ++ show tid ++ "] Got value: " ++ show val
    atomically $ putTMVar tmvar (val * 2)

  _ <- forkIO $ do
    tid <- myThreadId
    threadDelay 500000
    putStrLn $ "  [" ++ show tid ++ "] Putting 42..."
    atomically $ putTMVar tmvar 42

  threadDelay 1000000
  result <- atomically $ takeTMVar tmvar
  putStrLn $ "  Final value: " ++ show result

-- ===================================================================
-- Example 8: Deadlock-free concurrent linked-list insert
-- ===================================================================

data Node a = Node
  { nodeValue :: a
  , nodeNext  :: TVar (Maybe (TVar (Node a)))
  }

newtype ConcurrentList a = ConcurrentList (TVar (Maybe (TVar (Node a))))

-- Not a full implementation — just demonstrates how STM enables safe
-- concurrent data structure traversal without lock ordering.

-- ===================================================================
-- Main: run all examples
-- ===================================================================

main :: IO ()
main = do
  putStrLn "Software Transactional Memory — Haskell Examples"
  putStrLn "================================================"
  hFlush stdout

  counterExample
  retryExample
  orElseExample
  bankTransferExample
  orElsePayExample
  tchanExample
  tmvarExample

  putStrLn "\nAll examples completed."
