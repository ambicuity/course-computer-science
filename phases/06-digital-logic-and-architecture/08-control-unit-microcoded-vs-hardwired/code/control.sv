// Control Unit — Microcoded vs Hardwired
// SystemVerilog module library for Phase 06, Lesson 08

// ============================================================
// RISC-V Opcode Definitions
// ============================================================
localparam R_TYPE = 7'b0110011;
localparam I_TYPE = 7'b0010011;
localparam LOAD   = 7'b0000011;
localparam STORE  = 7'b0100011;
localparam BRANCH = 7'b1100011;
localparam JAL    = 7'b1101111;
localparam JALR   = 7'b1100111;

// ============================================================
// Hardwired Control Unit
// Pure combinational logic: opcode → control signals
// ============================================================
module control_unit_hardwired (
  input  logic [6:0] opcode,
  output logic       reg_write,
  output logic       alu_src,
  output logic       mem_read,
  output logic       mem_write,
  output logic       branch,
  output logic       mem_to_reg,
  output logic [1:0] alu_op
);
  always_comb begin
    // Defaults — deassert everything
    reg_write  = 0;
    alu_src    = 0;
    mem_read   = 0;
    mem_write  = 0;
    branch     = 0;
    mem_to_reg = 0;
    alu_op     = 2'b00;

    case (opcode)
      R_TYPE: begin // ADD, SUB, AND, OR, SLT, etc.
        reg_write = 1;
        alu_op    = 2'b10;
      end
      I_TYPE: begin // ADDI, SLTI, XORI, ORI, ANDI
        reg_write = 1;
        alu_src   = 1;
        alu_op    = 2'b11;
      end
      LOAD: begin // LW, LB, LH, LBU, LHU
        reg_write  = 1;
        alu_src    = 1;
        mem_read   = 1;
        mem_to_reg = 1;
      end
      STORE: begin // SW, SB, SH
        alu_src   = 1;
        mem_write = 1;
      end
      BRANCH: begin // BEQ, BNE, BLT, BGE, BLTU, BGEU
        branch = 1;
        alu_op = 2'b01;
      end
      JAL: begin // Jump and Link
        reg_write = 1;
      end
      JALR: begin // Jump and Link Register
        reg_write = 1;
        alu_src   = 1;
      end
      default: ; // NOP — all signals stay 0
    endcase
  end
endmodule

// ============================================================
// Microcoded Control Unit
// ROM-based: control_word = microcode_rom[opcode]
// ============================================================
module control_unit_microcoded (
  input  logic [6:0] opcode,
  output logic [7:0] control_word
);
  // Control word layout (MSB→LSB):
  //   [7]    = RegWrite
  //   [6]    = ALUSrc
  //   [5]    = MemRead
  //   [4]    = MemWrite
  //   [3]    = Branch
  //   [2]    = MemToReg
  //   [1:0]  = ALUOp

  logic [7:0] microcode_rom [0:127];

  initial begin
    // Clear all entries to NOP
    for (int i = 0; i < 128; i++)
      microcode_rom[i] = 8'h00;

    // Populate instruction microcode
    microcode_rom[R_TYPE] = 8'b1_0_0_0_0_0_10; // R-type
    microcode_rom[I_TYPE] = 8'b1_1_0_0_0_0_11; // I-type
    microcode_rom[LOAD]   = 8'b1_1_1_0_0_1_00; // Load
    microcode_rom[STORE]  = 8'b0_1_0_1_0_0_00; // Store
    microcode_rom[BRANCH] = 8'b0_0_0_0_1_0_01; // Branch
    microcode_rom[JAL]    = 8'b1_0_0_0_0_0_00; // JAL
    microcode_rom[JALR]   = 8'b1_1_0_0_0_0_00; // JALR
  end

  assign control_word = microcode_rom[opcode[6:0]];
endmodule

// ============================================================
// Testbench — Verify both control units produce identical output
// ============================================================
module tb_control;
  logic [6:0] opcode;
  logic       hw_reg_write, hw_alu_src, hw_mem_read, hw_mem_write;
  logic       hw_branch, hw_mem_to_reg;
  logic [1:0] hw_alu_op;
  logic [7:0] mc_control_word;

  control_unit_hardwired dut_hw (
    .opcode    (opcode),
    .reg_write (hw_reg_write),
    .alu_src   (hw_alu_src),
    .mem_read  (hw_mem_read),
    .mem_write (hw_mem_write),
    .branch    (hw_branch),
    .mem_to_reg(hw_mem_to_reg),
    .alu_op    (hw_alu_op)
  );

  control_unit_microcoded dut_mc (
    .opcode      (opcode),
    .control_word(mc_control_word)
  );

  // Pack hardwired outputs into the same 8-bit format for comparison
  logic [7:0] hw_packed;
  assign hw_packed = {hw_reg_write, hw_alu_src, hw_mem_read, hw_mem_write,
                      hw_branch, hw_mem_to_reg, hw_alu_op};

  int errors = 0;

  task automatic check(input [6:0] opc, input string name);
    opcode = opc;
    #1;
    if (hw_packed !== mc_control_word) begin
      $display("FAIL  opcode=%b (%s)  hw=%b  mc=%b",
               opc, name, hw_packed, mc_control_word);
      errors++;
    end else begin
      $display("PASS  opcode=%b (%s)  control=%b",
               opc, name, hw_packed);
    end
  endtask

  initial begin
    $display("=== Control Unit Equivalence Test ===");

    check(R_TYPE, "R-type");
    check(I_TYPE, "I-type");
    check(LOAD,   "Load");
    check(STORE,  "Store");
    check(BRANCH, "Branch");
    check(JAL,    "JAL");
    check(JALR,   "JALR");

    // Also test an unrecognized opcode — both should output NOP (0x00)
    check(7'b1111111, "unknown");
    check(7'b0000000, "zero");

    $display("=======================================");
    if (errors == 0)
      $display("ALL TESTS PASSED — hardwired and microcoded agree.");
    else
      $display("%0d TEST(S) FAILED.", errors);

    $finish;
  end
endmodule
