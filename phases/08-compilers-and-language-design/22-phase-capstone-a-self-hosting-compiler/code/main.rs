// main.rs — Compiler driver for pal
//
// Ties together all compilation stages:
//   source → lexer → parser → AST → typecheck → IR → optimize → codegen → .s
//
// Usage:
//   pal compile source.pal              → compile to executable
//   pal --verbose source.pal            → show all stages
//   pal --ir-only source.pal            → output IR only

mod lexer;
mod parser;
mod ast;
mod typecheck;
mod ir;
mod optimize;
mod codegen;

use std::env;
use std::fs;
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: pal [--verbose|--ir-only] <source.pal>");
        std::process::exit(1);
    }

    let mut verbose = false;
    let mut ir_only = false;
    let mut source_file = String::new();

    for arg in &args[1..] {
        match arg.as_str() {
            "--verbose" => verbose = true,
            "--ir-only" => ir_only = true,
            _ => source_file = arg.clone(),
        }
    }

    if source_file.is_empty() {
        eprintln!("Error: no source file specified");
        std::process::exit(1);
    }

    let source = fs::read_to_string(&source_file).unwrap_or_else(|e| {
        eprintln!("Error reading '{}': {}", source_file, e);
        std::process::exit(1);
    });

    if verbose {
        println!("=== Source ===");
        println!("{}", source);
    }

    // Stage 1: Parse
    let mut parser = parser::Parser::new(&source);
    let program = parser.parse_program();

    if verbose {
        println!("\n=== AST ===");
        println!("{:#?}", program);
    }

    // Stage 2: Type check
    let mut checker = typecheck::TypeChecker::new();
    checker.check_program(&program).unwrap_or_else(|e| {
        eprintln!("Type error: {}", e.message);
        std::process::exit(1);
    });

    if verbose {
        println!("\n=== Type check passed ===");
    }

    // Stage 3: IR generation
    let mut ir_gen = ir::IrGenerator::new();
    let instructions = ir_gen.generate_program(&program);

    if verbose {
        println!("\n=== IR (before optimization) ===");
        for (i, instr) in instructions.iter().enumerate() {
            println!("  {}: {:?}", i, instr);
        }
    }

    // Stage 4: Optimize
    let optimized = optimize::optimize(instructions);

    if verbose {
        println!("\n=== IR (after optimization) ===");
        for (i, instr) in optimized.iter().enumerate() {
            println!("  {}: {:?}", i, instr);
        }
    }

    if ir_only {
        for instr in &optimized {
            println!("{:?}", instr);
        }
        return;
    }

    // Stage 5: Code generation
    let asm = codegen::CodeGenerator::generate(&optimized);

    if verbose {
        println!("\n=== RISC-V Assembly ===");
        println!("{}", asm);
    }

    // Write assembly file
    let base_name = source_file.trim_end_matches(".pal");
    let asm_file = format!("{}.s", base_name);
    fs::write(&asm_file, &asm).unwrap_or_else(|e| {
        eprintln!("Error writing '{}': {}", asm_file, e);
        std::process::exit(1);
    });

    println!("Wrote {}", asm_file);

    // Assemble and link (if RISC-V toolchain available)
    let obj_file = format!("{}.o", base_name);
    let as_result = Command::new("riscv64-unknown-elf-as")
        .args(&["-o", &obj_file, &asm_file])
        .status();

    match as_result {
        Ok(status) if status.success() => {
            let ld_result = Command::new("riscv64-unknown-elf-ld")
                .args(&["-o", base_name, &obj_file])
                .status();

            match ld_result {
                Ok(s) if s.success() => {
                    println!("Compiled: {}", base_name);
                }
                _ => {
                    println!("Linked (but check your RISC-V linker): {}", base_name);
                }
            }
        }
        _ => {
            println!("Assembly written to {}. RISC-V toolchain not found — run manually.", asm_file);
        }
    }
}

// Example pal program for testing:
#[allow(dead_code)]
const HELLO_PAL: &str = r#"
program hello;

function fib(n: int): int;
begin
  if n <= 1 then
    fib := n
  else
    fib := fib(n - 1) + fib(n - 2)
end;

begin
  print(fib(10))
end.
"#;
