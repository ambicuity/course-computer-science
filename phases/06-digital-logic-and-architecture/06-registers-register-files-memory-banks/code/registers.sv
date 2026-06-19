module register #(
  parameter WIDTH = 32
) (
  input  logic              clk,
  input  logic              rst,
  input  logic              en,
  input  logic [WIDTH-1:0]  d,
  output logic [WIDTH-1:0]  q
);
  always_ff @(posedge clk) begin
    if (rst)
      q <= '0;
    else if (en)
      q <= d;
  end
endmodule


module register_file (
  input  logic        clk,
  input  logic        rst,
  input  logic        we,
  input  logic [4:0]  rs1_addr,
  input  logic [4:0]  rs2_addr,
  input  logic [4:0]  rd_addr,
  input  logic [31:0] rd_data,
  output logic [31:0] rs1_data,
  output logic [31:0] rs2_data
);

  logic [31:0] regs [1:31];

  assign rs1_data = (rs1_addr == 5'd0) ? 32'b0 : regs[rs1_addr];
  assign rs2_data = (rs2_addr == 5'd0) ? 32'b0 : regs[rs2_addr];

  always_ff @(posedge clk) begin
    if (rst) begin
      for (int i = 1; i < 32; i++)
        regs[i] <= 32'b0;
    end else if (we && rd_addr != 5'd0) begin
      regs[rd_addr] <= rd_data;
    end
  end

endmodule


module tb_registers;
  logic        clk, rst, we;
  logic [4:0]  rs1_addr, rs2_addr, rd_addr;
  logic [31:0] rd_data, rs1_data, rs2_data;

  register_file dut (
    .clk      (clk),
    .rst      (rst),
    .we       (we),
    .rs1_addr (rs1_addr),
    .rs2_addr (rs2_addr),
    .rd_addr  (rd_addr),
    .rd_data  (rd_data),
    .rs1_data (rs1_data),
    .rs2_data (rs2_data)
  );

  initial clk = 0;
  always #5 clk = ~clk;

  task automatic write_reg(input [4:0] addr, input [31:0] data);
    @(negedge clk);
    we      = 1;
    rd_addr = addr;
    rd_data = data;
    @(posedge clk);  // write commits on this edge
    #1;              // allow outputs to settle
    we = 0;
  endtask

  task automatic read_reg(input [4:0] addr1, input [4:0] addr2);
    rs1_addr = addr1;
    rs2_addr = addr2;
    #1;  // async read — outputs settle after address changes
  endtask

  initial begin
    $dumpfile("registers.vcd");
    $dumpvars(0, tb_registers);

    // Initialize
    rst = 1; we = 0;
    rs1_addr = 0; rs2_addr = 0;
    rd_addr = 0; rd_data = 0;
    @(posedge clk); #1;
    rst = 0;

    // Test 1: Write and read back
    $display("--- Test 1: Write and read back ---");
    write_reg(5'd1, 32'hDEAD_BEEF);
    read_reg(5'd1, 5'd0);
    assert(rs1_data == 32'hDEAD_BEEF)
      else $fatal(1, "FAIL: x1 should be DEADBEEF, got %h", rs1_data);
    $display("PASS: x1 = %h", rs1_data);

    // Test 2: Write x0 — should be discarded
    $display("--- Test 2: x0 hardwired to zero ---");
    write_reg(5'd0, 32'hFFFF_FFFF);
    read_reg(5'd0, 5'd0);
    assert(rs1_data == 32'h0)
      else $fatal(1, "FAIL: x0 should read 0, got %h", rs1_data);
    $display("PASS: x0 always reads 0");

    // Test 3: Read two registers simultaneously
    $display("--- Test 3: Dual-port read ---");
    write_reg(5'd2, 32'h1234_5678);
    write_reg(5'd3, 32'hAAAA_BBBB);
    read_reg(5'd2, 5'd3);
    assert(rs1_data == 32'h1234_5678)
      else $fatal(1, "FAIL: rs1 expected 12345678, got %h", rs1_data);
    assert(rs2_data == 32'hAAAA_BBBB)
      else $fatal(1, "FAIL: rs2 expected AAAABBBB, got %h", rs2_data);
    $display("PASS: rs1 = %h, rs2 = %h", rs1_data, rs2_data);

    // Test 4: Write-enable gating — we=0 should not write
    $display("--- Test 4: Write-enable gating ---");
    read_reg(5'd1, 5'd0);
    assert(rs1_data == 32'hDEAD_BEEF)
      else $fatal(1, "FAIL: x1 should still be DEADBEEF before gated write");
    @(negedge clk);
    we = 0; rd_addr = 5'd1; rd_data = 32'h0000_0000;
    @(posedge clk); #1;
    read_reg(5'd1, 5'd0);
    assert(rs1_data == 32'hDEAD_BEEF)
      else $fatal(1, "FAIL: x1 should not have changed, got %h", rs1_data);
    $display("PASS: x1 unchanged when we=0");

    // Test 5: Reset clears all registers
    $display("--- Test 5: Reset ---");
    rst = 1;
    @(posedge clk); #1;
    rst = 0;
    read_reg(5'd1, 5'd2);
    assert(rs1_data == 32'h0)
      else $fatal(1, "FAIL: x1 should be 0 after reset, got %h", rs1_data);
    assert(rs2_data == 32'h0)
      else $fatal(1, "FAIL: x2 should be 0 after reset, got %h", rs2_data);
    $display("PASS: all registers cleared after reset");

    $display("=== All tests passed ===");
    $finish;
  end
endmodule
