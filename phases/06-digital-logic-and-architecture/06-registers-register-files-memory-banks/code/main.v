module reg_file_2x8(
  input clk,
  input we,
  input [0:0] waddr,
  input [7:0] wdata,
  input [0:0] raddr0,
  input [0:0] raddr1,
  output [7:0] rdata0,
  output [7:0] rdata1
);
  reg [7:0] regs [0:1];

  always @(posedge clk) begin
    if (we) regs[waddr] <= wdata;
  end

  assign rdata0 = regs[raddr0];
  assign rdata1 = regs[raddr1];
endmodule
