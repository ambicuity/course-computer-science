// RISC-V ALU — 32-bit, all RV32I operations + flags
// Drop this module into any single-cycle or pipelined CPU datapath.

localparam ALU_ADD  = 4'b0000;
localparam ALU_SUB  = 4'b1000;
localparam ALU_AND  = 4'b0111;
localparam ALU_OR   = 4'b0110;
localparam ALU_XOR  = 4'b0100;
localparam ALU_SLT  = 4'b0010;
localparam ALU_SLTU = 4'b0011;
localparam ALU_SLL  = 4'b0001;
localparam ALU_SRL  = 4'b0101;
localparam ALU_SRA  = 4'b1101;

module alu (
    input  logic [31:0] a,
    input  logic [31:0] b,
    input  logic [3:0]  alu_op,
    output logic [31:0] result,
    output logic        zero,
    output logic        carry,
    output logic        overflow
);

    logic [32:0] sum_ext;  // extended to 33 bits to capture carry

    always_comb begin
        carry    = 1'b0;
        overflow = 1'b0;
        result   = 32'b0;

        case (alu_op)
            ALU_ADD: begin
                sum_ext  = {1'b0, a} + {1'b0, b};
                result   = sum_ext[31:0];
                carry    = sum_ext[32];
                overflow = (a[31] == b[31]) && (result[31] != a[31]);
            end
            ALU_SUB: begin
                sum_ext  = {1'b0, a} - {1'b0, b};
                result   = sum_ext[31:0];
                carry    = sum_ext[32];  // borrow: 0 means borrow occurred
                overflow = (a[31] != b[31]) && (result[31] != a[31]);
            end
            ALU_AND:  result = a & b;
            ALU_OR:   result = a | b;
            ALU_XOR:  result = a ^ b;
            ALU_SLT:  result = ($signed(a) < $signed(b)) ? 32'd1 : 32'd0;
            ALU_SLTU: result = (a < b) ? 32'd1 : 32'd0;
            ALU_SLL:  result = a << b[4:0];
            ALU_SRL:  result = a >> b[4:0];
            ALU_SRA:  result = $signed(a) >>> b[4:0];
            default:  result = 32'b0;
        endcase
    end

    assign zero = (result == 32'b0);

endmodule


// ─────────────────────────────────────────────────────────────
// Testbench — exhaustive tests for every ALU operation
// ─────────────────────────────────────────────────────────────
module tb_alu;

    logic [31:0] a, b, result;
    logic [3:0]  alu_op;
    logic        zero, carry, overflow;

    alu dut (
        .a(a), .b(b), .alu_op(alu_op),
        .result(result), .zero(zero),
        .carry(carry), .overflow(overflow)
    );

    integer pass_count = 0;
    integer fail_count = 0;

    task check(input string name, input logic [31:0] exp_result,
               input logic exp_zero, input logic exp_carry, input logic exp_overflow);
        if (result === exp_result && zero === exp_zero &&
            carry === exp_carry && overflow === exp_overflow) begin
            pass_count++;
        end else begin
            $display("FAIL [%s]: a=%h b=%h op=%b", name, a, b, alu_op);
            $display("  result: got=%h exp=%h | zero: got=%b exp=%b | carry: got=%b exp=%b | overflow: got=%b exp=%b",
                     result, exp_result, zero, exp_zero, carry, exp_carry, overflow, exp_overflow);
            fail_count++;
        end
    endtask

    initial begin
        $display("=== ALU Testbench ===");

        // ── ADD ──────────────────────────────────────────────
        alu_op = ALU_ADD;

        // Basic addition
        a = 32'd10; b = 32'd20; #1;
        check("ADD basic", 32'd30, 0, 0, 0);

        // Zero result
        a = 32'd0; b = 32'd0; #1;
        check("ADD zero", 32'd0, 1, 0, 0);

        // Unsigned overflow (carry)
        a = 32'hFFFFFFFF; b = 32'd1; #1;
        check("ADD overflow carry", 32'd0, 1, 1, 0);

        // Signed overflow: positive + positive = negative
        a = 32'h7FFFFFFF; b = 32'd1; #1;
        check("ADD signed overflow", 32'h80000000, 0, 0, 1);

        // Negative + negative = positive (overflow)
        a = 32'h80000000; b = 32'h80000000; #1;
        check("ADD neg+neg overflow", 32'd0, 1, 1, 1);

        // Max positive + 0
        a = 32'h7FFFFFFF; b = 32'd0; #1;
        check("ADD max+0", 32'h7FFFFFFF, 0, 0, 0);

        // ── SUB ──────────────────────────────────────────────
        alu_op = ALU_SUB;

        // Basic subtraction
        a = 32'd30; b = 32'd10; #1;
        check("SUB basic", 32'd20, 0, 1, 0);

        // Equal operands → zero
        a = 32'd42; b = 32'd42; #1;
        check("SUB equal", 32'd0, 1, 1, 0);

        // Underflow: 0 - 1 = -1 (unsigned wraps)
        a = 32'd0; b = 32'd1; #1;
        check("SUB underflow", 32'hFFFFFFFF, 0, 0, 0);

        // Signed overflow: 0x80000000 - 1
        a = 32'h80000000; b = 32'd1; #1;
        check("SUB signed overflow", 32'h7FFFFFFF, 0, 1, 1);

        // Max - max
        a = 32'hFFFFFFFF; b = 32'hFFFFFFFF; #1;
        check("SUB max-max", 32'd0, 1, 1, 0);

        // ── AND ──────────────────────────────────────────────
        alu_op = ALU_AND;

        a = 32'hFF00FF00; b = 32'h0F0F0F0F; #1;
        check("AND basic", 32'h0F000F00, 0, 0, 0);

        a = 32'hFFFFFFFF; b = 32'd0; #1;
        check("AND zero mask", 32'd0, 1, 0, 0);

        a = 32'hFFFFFFFF; b = 32'hFFFFFFFF; #1;
        check("AND all ones", 32'hFFFFFFFF, 0, 0, 0);

        // ── OR ───────────────────────────────────────────────
        alu_op = ALU_OR;

        a = 32'hFF00FF00; b = 32'h0F0F0F0F; #1;
        check("OR basic", 32'hFF0FFF0F, 0, 0, 0);

        a = 32'd0; b = 32'd0; #1;
        check("OR zero", 32'd0, 1, 0, 0);

        a = 32'h0000FFFF; b = 32'hFFFF0000; #1;
        check("OR combine halves", 32'hFFFFFFFF, 0, 0, 0);

        // ── XOR ──────────────────────────────────────────────
        alu_op = ALU_XOR;

        a = 32'hFF00FF00; b = 32'h0F0F0F0F; #1;
        check("XOR basic", 32'hF00FF00F, 0, 0, 0);

        a = 32'hA5A5A5A5; b = 32'hA5A5A5A5; #1;
        check("XOR self", 32'd0, 1, 0, 0);

        a = 32'hFFFFFFFF; b = 32'd0; #1;
        check("XOR with zero", 32'hFFFFFFFF, 0, 0, 0);

        // ── SLT (signed) ────────────────────────────────────
        alu_op = ALU_SLT;

        a = 32'd5; b = 32'd10; #1;
        check("SLT 5<10", 32'd1, 0, 0, 0);

        a = 32'd10; b = 32'd5; #1;
        check("SLT 10<5", 32'd0, 1, 0, 0);

        // Signed: -1 < 1
        a = 32'hFFFFFFFF; b = 32'd1; #1;
        check("SLT -1<1 (signed)", 32'd1, 0, 0, 0);

        // Edge: 0x80000000 is the most negative
        a = 32'h80000000; b = 32'h7FFFFFFF; #1;
        check("SLT min<max", 32'd1, 0, 0, 0);

        // ── SLTU (unsigned) ─────────────────────────────────
        alu_op = ALU_SLTU;

        a = 32'd5; b = 32'd10; #1;
        check("SLTU 5<10", 32'd1, 0, 0, 0);

        // Unsigned: 0xFFFFFFFF is the max unsigned, not -1
        a = 32'hFFFFFFFF; b = 32'd1; #1;
        check("SLTU 0xFFFFFFFF < 1 (unsigned)", 32'd0, 1, 0, 0);

        a = 32'd0; b = 32'hFFFFFFFF; #1;
        check("SLTU 0 < max", 32'd1, 0, 0, 0);

        // ── SLL (shift left logical) ────────────────────────
        alu_op = ALU_SLL;

        a = 32'h00000001; b = 32'd4; #1;
        check("SLL 1<<4", 32'h00000010, 0, 0, 0);

        a = 32'hFFFFFFFF; b = 32'd0; #1;
        check("SLL no shift", 32'hFFFFFFFF, 0, 0, 0);

        a = 32'h00000001; b = 32'd31; #1;
        check("SLL 1<<31", 32'h80000000, 0, 0, 0);

        a = 32'h80000000; b = 32'd1; #1;
        check("SLL bit falls off", 32'd0, 1, 0, 0);

        // ── SRL (shift right logical) ───────────────────────
        alu_op = ALU_SRL;

        a = 32'h80000000; b = 32'd4; #1;
        check("SRL basic", 32'h08000000, 0, 0, 0);

        a = 32'h80000000; b = 32'd31; #1;
        check("SRL by 31", 32'd1, 0, 0, 0);

        a = 32'h00000001; b = 32'd1; #1;
        check("SRL 1>>1", 32'd0, 1, 0, 0);

        // ── SRA (shift right arithmetic) ─────────────────────
        alu_op = ALU_SRA;

        a = 32'h80000000; b = 32'd4; #1;
        check("SRA sign extend", 32'hF8000000, 0, 0, 0);

        a = 32'h7FFFFFFF; b = 32'd4; #1;
        check("SRA positive", 32'h07FFFFFF, 0, 0, 0);

        a = 32'h80000000; b = 32'd31; #1;
        check("SRA by 31", 32'hFFFFFFFF, 0, 0, 0);

        a = 32'hFFFFFFFF; b = 32'd0; #1;
        check("SRA no shift", 32'hFFFFFFFF, 0, 0, 0);

        // ── Default / unknown opcode ─────────────────────────
        alu_op = 4'b1111;
        a = 32'hDEADBEEF; b = 32'hCAFEBABE; #1;
        check("default opcode", 32'd0, 1, 0, 0);

        // ── Summary ──────────────────────────────────────────
        $display("=== Results: %0d passed, %0d failed ===", pass_count, fail_count);
        if (fail_count == 0)
            $display("ALL TESTS PASSED");
        else
            $display("SOME TESTS FAILED");
        $finish;
    end

endmodule
