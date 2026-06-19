// gates.sv — Basic logic gate library + exhaustive testbench
// Phase 06, Lesson 02: Transistors → Logic Gates

module not_gate(input logic a, output logic y);
  assign y = ~a;
endmodule

module and_gate(input logic a, b, output logic y);
  assign y = a & b;
endmodule

module or_gate(input logic a, b, output logic y);
  assign y = a | b;
endmodule

module nand_gate(input logic a, b, output logic y);
  assign y = ~(a & b);
endmodule

module nor_gate(input logic a, b, output logic y);
  assign y = ~(a | b);
endmodule

module xor_gate(input logic a, b, output logic y);
  assign y = a ^ b;
endmodule

module xnor_gate(input logic a, b, output logic y);
  assign y = ~(a ^ b);
endmodule

// --- Testbench ---

module tb_gates;
  logic a, b;
  logic y_not, y_and, y_or, y_nand, y_nor, y_xor, y_xnor;

  not_gate  u_not  (.a(a),         .y(y_not));
  and_gate  u_and  (.a(a), .b(b),  .y(y_and));
  or_gate   u_or   (.a(a), .b(b),  .y(y_or));
  nand_gate u_nand (.a(a), .b(b),  .y(y_nand));
  nor_gate  u_nor  (.a(a), .b(b),  .y(y_nor));
  xor_gate  u_xor  (.a(a), .b(b),  .y(y_xor));
  xnor_gate u_xnor (.a(a), .b(b),  .y(y_xnor));

  // Expected values for each combination
  // a b | NOT AND OR NAND NOR XOR XNOR
  // 0 0 |  1   0   0   1    1   0    1
  // 0 1 |  1   0   1   1    0   1    0
  // 1 0 |  0   0   1   1    0   1    0
  // 1 1 |  0   1   1   0    0   0    1

  logic [6:0] expected [0:3];
  initial begin
    expected[0] = 7'b1_0_0_1_1_0_1;  // a=0 b=0
    expected[1] = 7'b1_0_1_1_0_1_0;  // a=0 b=1
    expected[2] = 7'b0_0_1_1_0_1_0;  // a=1 b=0
    expected[3] = 7'b0_1_1_0_0_0_1;  // a=1 b=1
  end

  int errors = 0;

  initial begin
    $display(" a b | NOT AND OR NAND NOR XOR XNOR | PASS?");
    $display(" ----+--------------------------------+-------");
    for (int i = 0; i < 4; i++) begin
      {a, b} = i[1:0];
      #1;
      logic [6:0] got = {y_not, y_and, y_or, y_nand, y_nor, y_xor, y_xnor};
      string status = (got == expected[i]) ? "  ok" : "FAIL";
      if (got != expected[i]) errors++;
      $display(" %b %b |  %b   %b  %b   %b   %b   %b    %b  | %s",
               a, b, y_not, y_and, y_or, y_nand, y_nor, y_xor, y_xnor, status);
    end
    $display("");
    if (errors == 0)
      $display("PASSED: All gate outputs match truth tables.");
    else
      $display("FAILED: %0d mismatch(es) found.", errors);
    $finish;
  end
endmodule
