module single_cycle_datapath(
  input [31:0] pc,
  input [31:0] imm,
  input [31:0] reg_a,
  input [31:0] reg_b,
  input alu_src,
  output [31:0] alu_in2,
  output [31:0] alu_out,
  output [31:0] pc_next
);
  assign alu_in2 = alu_src ? imm : reg_b;
  assign alu_out = reg_a + alu_in2;
  assign pc_next = pc + 32'd4;
endmodule
