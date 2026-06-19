// LLVM IR Examples — hand-written IR and a simple pass
//
// Part 1: Hand-written LLVM IR for fibonacci, factorial, gcd
//   Saved as separate .ll files for use with clang/opt/llc.
//
// Part 2: A simple LLVM pass that counts instructions per opcode.
//   Build against an LLVM installation with:
//     clang++ -shared -o libInstCount.so inst_counter.cpp \
//       $(llvm-config --cxxflags --ldflags --libs)
//
// Part 3: Shell scripts (compile.sh, run_passes.sh, gen_asm.sh)

#include <iostream>
#include <map>
#include <string>

// -------------------------------------------------------
// Part 1: Write .ll files with hand-crafted LLVM IR
// -------------------------------------------------------

void write_fib_ll() {
    const char* src = R"(
; Fibonacci — recursive, written by hand
define i32 @fib(i32 %n) {
entry:
  %cmp = icmp sle i32 %n, 1
  br i1 %cmp, label %base, label %recur

base:
  ret i32 %n

recur:
  %n1 = sub i32 %n, 1
  %n2 = sub i32 %n, 2
  %r1 = call i32 @fib(i32 %n1)
  %r2 = call i32 @fib(i32 %n2)
  %result = add i32 %r1, %r2
  ret i32 %result
}

; Entry point — compute fib(10) and return it
define i32 @main() {
entry:
  %result = call i32 @fib(i32 10)
  ret i32 %result
}
)";
    FILE* f = fopen("fib.ll", "w");
    if (f) { fputs(src, f); fclose(f); }
    std::cout << "Wrote fib.ll\n";
}

void write_factorial_ll() {
    const char* src = R"(
; Factorial — iterative, written by hand
define i32 @factorial(i32 %n) {
entry:
  br label %loop

loop:
  %i = phi i32 [ 1, %entry ], [ %i_next, %loop ]
  %acc = phi i32 [ 1, %entry ], [ %acc_next, %loop ]
  %acc_next = mul i32 %acc, %i
  %i_next = add i32 %i, 1
  %done = icmp sgt i32 %i_next, %n
  br i1 %done, label %exit, label %loop

exit:
  ret i32 %acc_next
}

define i32 @main() {
entry:
  %result = call i32 @factorial(i32 7)
  ret i32 %result
}
)";
    FILE* f = fopen("factorial.ll", "w");
    if (f) { fputs(src, f); fclose(f); }
    std::cout << "Wrote factorial.ll\n";
}

void write_gcd_ll() {
    const char* src = R"(
; GCD — Euclidean algorithm, written by hand
define i32 @gcd(i32 %a, i32 %b) {
entry:
  %is_zero = icmp eq i32 %b, 0
  br i1 %is_zero, label %done, label %recur

recur:
  %rem = srem i32 %a, %b
  %result = call i32 @gcd(i32 %b, i32 %rem)
  ret i32 %result

done:
  ret i32 %a
}

define i32 @main() {
entry:
  %result = call i32 @gcd(i32 48, i32 18)
  ret i32 %result
}
)";
    FILE* f = fopen("gcd.ll", "w");
    if (f) { fputs(src, f); fclose(f); }
    std::cout << "Wrote gcd.ll\n";
}

// -------------------------------------------------------
// Part 2: Simple LLVM pass — instruction counter
//
// If compiled against LLVM headers, this pass walks every
// basic block in a function and tallies each opcode.
// When built as a shared library, load with:
//   opt -load-pass-plugin=./libInstCount.so -passes=inst-counter foo.ll
// -------------------------------------------------------

#ifdef USE_LLVM_PASS

#include "llvm/IR/PassManager.h"
#include "llvm/IR/Function.h"
#include "llvm/IR/Instructions.h"
#include "llvm/Passes/PassBuilder.h"
#include "llvm/Passes/PassPlugin.h"

namespace {

struct InstCounterPass : llvm::PassInfoMixin<InstCounterPass> {
    llvm::PreservedAnalyses run(llvm::Function &F,
                                 llvm::FunctionAnalysisManager &) {
        std::map<std::string, int> counts;
        for (auto &BB : F) {
            for (auto &I : BB) {
                counts[I.getOpcodeName()]++;
            }
        }
        llvm::errs() << "=== InstCounter: " << F.getName() << " ===\n";
        for (auto &[name, n] : counts) {
            llvm::errs() << "  " << name << ": " << n << "\n";
        }
        return llvm::PreservedAnalyses::all();
    }
};

} // anonymous namespace

extern "C" LLVM_ATTRIBUTE_WEAK ::llvm::PassPluginLibraryInfo
llvmGetPassPluginInfo() {
    return {LLVM_PLUGIN_API_VERSION, "InstCounter", "0.1",
            [](llvm::PassBuilder &PB) {
                PB.registerPipelineParsingCallback(
                    [](llvm::StringRef Name,
                       llvm::FunctionPassManager &FPM,
                       llvm::ArrayRef<llvm::PassBuilder::PipelineElement>) {
                        if (Name == "inst-counter") {
                            FPM.addPass(InstCounterPass());
                            return true;
                        }
                        return false;
                    });
            }};
}

#else // Standalone demonstration without LLVM headers

struct InstructionCounter {
    std::map<std::string, int> counts;
    void record(const std::string& op) { counts[op]++; }
    void dump(const std::string& func_name) {
        std::cout << "=== " << func_name << " ===\n";
        for (auto& [op, n] : counts) {
            std::cout << "  " << op << ": " << n << "\n";
        }
    }
};

void demonstrate_counter() {
    // Simulate counting instructions from fib function
    InstructionCounter counter;
    counter.record("icmp");
    counter.record("br");
    counter.record("ret");
    counter.record("sub");
    counter.record("call");
    counter.record("call");
    counter.record("add");
    counter.record("ret");
    counter.dump("fib");
}

#endif

// -------------------------------------------------------
// Part 3: Shell script generation
// -------------------------------------------------------

void write_compile_script() {
    const char* src = R"(#!/bin/bash
# compile.sh — compile hand-written IR and verify
set -e

echo "=== Compiling fib.ll ==="
clang fib.ll -o fib_bin
./fib_bin
echo "fib(10) = $?"

echo ""
echo "=== Compiling factorial.ll ==="
clang factorial.ll -o factorial_bin
./factorial_bin
echo "factorial(7) = $?"

echo ""
echo "=== Compiling gcd.ll ==="
clang gcd.ll -o gcd_bin
./gcd_bin
echo "gcd(48, 18) = $?"
)";
    FILE* f = fopen("compile.sh", "w");
    if (f) { fputs(src, f); fclose(f); }
    system("chmod +x compile.sh");
    std::cout << "Wrote compile.sh\n";
}

void write_run_passes_script() {
    const char* src = R"(#!/bin/bash
# run_passes.sh — run optimization passes on IR
set -e

echo "=== Before passes (fib.ll) ==="
cat fib.ll

echo ""
echo "=== After mem2reg ==="
opt -S -passes=mem2reg fib.ll -o fib_mem2reg.ll 2>/dev/null || \
  opt -S -mem2reg fib.ll -o fib_mem2reg.ll
cat fib_mem2reg.ll

echo ""
echo "=== After instcombine + simplifycfg ==="
opt -S -passes=instcombine,simplifycfg fib_mem2reg.ll -o fib_opt.ll 2>/dev/null || \
  opt -S -instcombine -simplifycfg fib_mem2reg.ll -o fib_opt.ll
cat fib_opt.ll

echo ""
echo "=== Generate C from IR (using clang) and compare ==="
echo "(compile the .ll and run to verify correctness)"
clang fib_opt.ll -o fib_opt_bin
./fib_opt_bin
echo "fib(10) = $?"
)";
    FILE* f = fopen("run_passes.sh", "w");
    if (f) { fputs(src, f); fclose(f); }
    system("chmod +x run_passes.sh");
    std::cout << "Wrote run_passes.sh\n";
}

void write_gen_asm_script() {
    const char* src = R"(#!/bin/bash
# gen_asm.sh — generate assembly from IR for different targets
set -e

echo "=== RISC-V 64 assembly for fib.ll ==="
llc -mtriple=riscv64 -O2 fib.ll -o fib_rv64.s 2>/dev/null && \
  head -30 fib_rv64.s || echo "(llc riscv64 target not available)"

echo ""
echo "=== x86-64 assembly for fib.ll ==="
llc -mtriple=x86_64 -O2 fib.ll -o fib_x86.s 2>/dev/null && \
  head -30 fib_x86.s || echo "(llc x86_64 target not available)"

echo ""
echo "=== AArch64 assembly for gcd.ll ==="
llc -mtriple=aarch64 -O2 gcd.ll -o gcd_aarch64.s 2>/dev/null && \
  head -30 gcd_aarch64.s || echo "(llc aarch64 target not available)"

echo ""
echo "=== Emit C source from clang IR comparison ==="
echo "int fib_c(int n) { return n <= 1 ? n : fib_c(n-1) + fib_c(n-2); }" > fib.c
clang -S -emit-llvm -O0 fib.c -o fib_clang_O0.ll
clang -S -emit-llvm -O2 fib.c -o fib_clang_O2.ll
echo "Generated fib_clang_O0.ll and fib_clang_O2.ll"
)";
    FILE* f = fopen("gen_asm.sh", "w");
    if (f) { fputs(src, f); fclose(f); }
    system("chmod +x gen_asm.sh");
    std::cout << "Wrote gen_asm.sh\n";
}

// -------------------------------------------------------
// Main
// -------------------------------------------------------

int main() {
    std::cout << "LLVM IR Examples & Tools\n";
    std::cout << "========================\n\n";

    write_fib_ll();
    write_factorial_ll();
    write_gcd_ll();
    write_compile_script();
    write_run_passes_script();
    write_gen_asm_script();

#ifndef USE_LLVM_PASS
    std::cout << "\n--- Standalone instruction counter demo ---\n";
    demonstrate_counter();
#endif

    std::cout << "\nUsage:\n";
    std::cout << "  1. bash compile.sh        — compile IR and verify output\n";
    std::cout << "  2. bash run_passes.sh     — run opt passes and compare\n";
    std::cout << "  3. bash gen_asm.sh        — generate assembly for targets\n";
    std::cout << "  4. Build with -DUSE_LLVM_PASS to compile the LLVM plugin pass\n";
    return 0;
}
