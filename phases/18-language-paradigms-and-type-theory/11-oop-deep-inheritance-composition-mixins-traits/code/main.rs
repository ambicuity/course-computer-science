trait Formatter {
    fn format(&self, s: &str) -> String;
}

struct Upper;

impl Formatter for Upper {
    fn format(&self, s: &str) -> String {
        s.to_uppercase()
    }
}

fn run_service(f: &dyn Formatter, input: &str) -> String {
    f.format(input)
}

fn main() {
    let u = Upper;
    println!("{}", run_service(&u, "trait composition"));
}
