use std::io::{self, Write};

fn classify(statement: &str) -> &'static str {
    let s = statement.to_lowercase();
    if s.contains("for all") || s.contains("forall") {
        "quantified proposition"
    } else if s.contains("if") && s.contains("then") {
        "implication"
    } else if s.contains("and") || s.contains("or") {
        "compound proposition"
    } else {
        "atomic proposition"
    }
}

fn main() -> io::Result<()> {
    print!("enter statement: ");
    io::stdout().flush()?;

    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    let k = classify(buf.trim());
    println!("classification: {k}");
    Ok(())
}
