#[derive(Debug)]
enum TransferError {
    NonPositiveAmount,
    InsufficientFunds,
    InvariantViolation,
}

fn transfer(src: i64, dst: i64, amount: i64) -> Result<(i64, i64), TransferError> {
    if amount <= 0 {
        return Err(TransferError::NonPositiveAmount);
    }
    if src < amount {
        return Err(TransferError::InsufficientFunds);
    }

    let new_src = src - amount;
    let new_dst = dst + amount;

    if new_src < 0 || new_src + new_dst != src + dst {
        return Err(TransferError::InvariantViolation);
    }
    Ok((new_src, new_dst))
}

fn main() {
    println!("{:?}", transfer(100, 20, 30));
}
