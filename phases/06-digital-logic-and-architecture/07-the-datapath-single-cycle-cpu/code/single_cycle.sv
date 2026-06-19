// The Datapath — Single-Cycle CPU
// SystemVerilog single-cycle RISC-V CPU for Phase 06, Lesson 07

// ============================================================
// Instruction Memory (ROM)
// ============================================================
module imem #(
    parameter WORDS = 64
)(
    input  logic [31:0] addr,
    output logic [31:0] rdata
);
    logic [31:0] mem [0:WORDS-1];

    // Word-addressed: divide byte address by 4
    assign rdata = mem[addr[31:2]];
endmodule

// ============================================================
// Data Memory (RAM)
// ============================================================
module dmem #(
    parameter WORDS = 256
)(
    input  logic        clk,
    input  logic [31:0] addr,
    input  logic [31:0] wdata,
    input  logic        wr_en,
    input  logic        rd_en,
    output logic [31:0] rdata
);
    logic [31:0] mem [0:WORDS-1];

    // Read: combinational
    assign rdata = rd_en ? mem[addr[31:2]] : 32'h0;

    // Write: clocked
    always_ff @(posedge clk) begin
        if (wr_en) mem[addr[31:2]] <= wdata;
    end
endmodule

// ============================================================
// Register File (32 x 32-bit, x0 hardwired to 0)
// ============================================================
module reg_file (
    input  logic        clk,
    input  logic        rst,
    input  logic [4:0]  rs1, rs2, rd,
    input  logic [31:0] rd_data,
    input  logic        wr_en,
    output logic [31:0] rs1_data,
    output logic [31:0] rs2_data
);
    logic [31:0] regs [1:31]; // x1–x31; x0 is hardwired

    assign rs1_data = (rs1 == 5'd0) ? 32'h0 : regs[rs1];
    assign rs2_data = (rs2 == 5'd0) ? 32'h0 : regs[rs2];

    always_ff @(posedge clk) begin
        if (rst) begin
            for (int i = 1; i < 32; i++) regs[i] <= 32'h0;
        end else if (wr_en && rd != 5'd0) begin
            regs[rd] <= rd_data;
        end
    end
endmodule

// ============================================================
// ALU (supports add, sub, and, or, slt)
// ============================================================
module alu_main (
    input  logic [31:0] a, b,
    input  logic [3:0]  alu_ctrl,
    output logic [31:0] result,
    output logic        zero
);
    always_comb begin
        case (alu_ctrl)
            4'b0010: result = a + b;          // add
            4'b0110: result = a - b;          // sub
            4'b0000: result = a & b;          // and
            4'b0001: result = a | b;          // or
            4'b0111: result = ($signed(a) < $signed(b)) ? 32'd1 : 32'd0; // slt
            default: result = 32'hxxxxxxxx;
        endcase
    end
    assign zero = (result == 32'h0);
endmodule

// ============================================================
// Immediate Generator
// ============================================================
module imm_gen (
    input  logic [31:0] instr,
    output logic [31:0] imm
);
    logic [6:0] opcode;
    assign opcode = instr[6:0];

    always_comb begin
        case (opcode)
            7'b0000011,     // I-type (lw)
            7'b0010011:     // I-type (addi)
                imm = {{20{instr[31]}}, instr[31:20]};
            7'b0100011:     // S-type (sw)
                imm = {{20{instr[31]}}, instr[31:25], instr[11:7]};
            7'b1100011:     // B-type (beq)
                imm = {{20{instr[31]}}, instr[7], instr[30:25], instr[11:8], 1'b0};
            default:
                imm = 32'h0;
        endcase
    end
endmodule

// ============================================================
// Control Unit
// ============================================================
module control_unit (
    input  logic [6:0] opcode,
    output logic       ALUSrc,
    output logic       MemToReg,
    output logic       RegWrite,
    output logic       MemRead,
    output logic       MemWrite,
    output logic       Branch,
    output logic [1:0] ALUOp
);
    always_comb begin
        {ALUSrc, MemToReg, RegWrite, MemRead, MemWrite, Branch, ALUOp} = 7'b0;
        case (opcode)
            7'b0110011: begin // R-type (add, sub, and, or, slt)
                ALUSrc   = 0;
                MemToReg = 0;
                RegWrite = 1;
                MemRead  = 0;
                MemWrite = 0;
                Branch   = 0;
                ALUOp    = 2'b10;
            end
            7'b0000011: begin // I-type (lw)
                ALUSrc   = 1;
                MemToReg = 1;
                RegWrite = 1;
                MemRead  = 1;
                MemWrite = 0;
                Branch   = 0;
                ALUOp    = 2'b00;
            end
            7'b0010011: begin // I-type (addi)
                ALUSrc   = 1;
                MemToReg = 0;
                RegWrite = 1;
                MemRead  = 0;
                MemWrite = 0;
                Branch   = 0;
                ALUOp    = 2'b00;
            end
            7'b0100011: begin // S-type (sw)
                ALUSrc   = 1;
                MemToReg = 0; // don't care
                RegWrite = 0;
                MemRead  = 0;
                MemWrite = 1;
                Branch   = 0;
                ALUOp    = 2'b00;
            end
            7'b1100011: begin // B-type (beq)
                ALUSrc   = 0;
                MemToReg = 0; // don't care
                RegWrite = 0;
                MemRead  = 0;
                MemWrite = 0;
                Branch   = 1;
                ALUOp    = 2'b01;
            end
            default: ;
        endcase
    end
endmodule

// ============================================================
// ALU Control
// ============================================================
module alu_control (
    input  logic [1:0] ALUOp,
    input  logic [2:0] funct3,
    input  logic [6:0] funct7,
    output logic [3:0] alu_ctrl
);
    always_comb begin
        case (ALUOp)
            2'b00: alu_ctrl = 4'b0010; // add (lw, sw, addi)
            2'b01: alu_ctrl = 4'b0110; // sub (beq)
            2'b10: begin               // R-type
                case ({funct7, funct3})
                    10'b0000000_000: alu_ctrl = 4'b0010; // add
                    10'b0100000_000: alu_ctrl = 4'b0110; // sub
                    10'b0000000_111: alu_ctrl = 4'b0000; // and
                    10'b0000000_110: alu_ctrl = 4'b0001; // or
                    10'b0000000_010: alu_ctrl = 4'b0111; // slt
                    default:         alu_ctrl = 4'bxxxx;
                endcase
            end
            default: alu_ctrl = 4'bxxxx;
        endcase
    end
endmodule

// ============================================================
// Single-Cycle CPU Top Module
// ============================================================
module single_cycle_cpu #(
    parameter IMEM_WORDS = 64,
    parameter DMEM_WORDS = 256
)(
    input  logic clk,
    input  logic rst,
    output logic [31:0] pc_out,
    output logic [31:0] instr_out
);
    // PC register
    logic [31:0] pc, pc_next, pc_plus4;
    always_ff @(posedge clk) begin
        if (rst) pc <= 32'h0;
        else     pc <= pc_next;
    end
    assign pc_plus4 = pc + 4;
    assign pc_out   = pc;

    // Instruction memory
    logic [31:0] instr;
    imem #(.WORDS(IMEM_WORDS)) rom (.addr(pc), .rdata(instr));
    assign instr_out = instr;

    // Instruction decode
    logic [6:0]  opcode = instr[6:0];
    logic [4:0]  rd     = instr[11:7];
    logic [2:0]  funct3 = instr[14:12];
    logic [4:0]  rs1    = instr[19:15];
    logic [4:0]  rs2    = instr[24:20];
    logic [6:0]  funct7 = instr[31:25];

    // Immediate generator
    logic [31:0] imm;
    imm_gen gen (.instr(instr), .imm(imm));

    // Control unit
    logic ALUSrc, MemToReg, RegWrite, MemRead, MemWrite, Branch;
    logic [1:0] ALUOp;
    control_unit ctrl (
        .opcode(opcode),
        .ALUSrc(ALUSrc), .MemToReg(MemToReg), .RegWrite(RegWrite),
        .MemRead(MemRead), .MemWrite(MemWrite), .Branch(Branch),
        .ALUOp(ALUOp)
    );

    // Register file
    logic [31:0] rs1_data, rs2_data, write_data;
    reg_file rf (
        .clk(clk), .rst(rst),
        .rs1(rs1), .rs2(rs2), .rd(rd),
        .rd_data(write_data), .wr_en(RegWrite),
        .rs1_data(rs1_data), .rs2_data(rs2_data)
    );

    // ALU control
    logic [3:0]  alu_ctrl;
    alu_control ac (
        .ALUOp(ALUOp), .funct3(funct3), .funct7(funct7),
        .alu_ctrl(alu_ctrl)
    );

    // ALU
    logic [31:0] alu_b, alu_result;
    logic        alu_zero;
    assign alu_b = ALUSrc ? imm : rs2_data;
    alu_main alu (
        .a(rs1_data), .b(alu_b), .alu_ctrl(alu_ctrl),
        .result(alu_result), .zero(alu_zero)
    );

    // Data memory
    logic [31:0] dmem_rdata;
    dmem #(.WORDS(DMEM_WORDS)) ram (
        .clk(clk), .addr(alu_result),
        .wdata(rs2_data), .wr_en(MemWrite),
        .rd_en(MemRead), .rdata(dmem_rdata)
    );

    // Write-back mux
    assign write_data = MemToReg ? dmem_rdata : alu_result;

    // Next-PC mux
    logic branch_taken;
    assign branch_taken = Branch & alu_zero;
    assign pc_next = branch_taken ? (pc + imm) : pc_plus4;
endmodule

// ============================================================
// Testbench
// ============================================================
module tb_single_cycle;
    logic clk, rst;
    logic [31:0] pc_out, instr_out;

    single_cycle_cpu #(.IMEM_WORDS(64), .DMEM_WORDS(256))
        dut (.clk(clk), .rst(rst), .pc_out(pc_out), .instr_out(instr_out));

    // Clock: 10ns period
    always #5 clk = ~clk;

    // RISC-V instruction encoding helpers
    // R-type:  {funct7, rs2, rs1, funct3, rd, opcode}
    function automatic logic [31:0] R(logic [6:0] f7, logic [4:0] rs2, rs1,
                                       logic [2:0] f3, logic [4:0] rd);
        return {f7, rs2, rs1, f3, rd, 7'b0110011};
    endfunction
    // I-type:  {imm[11:0], rs1, funct3, rd, opcode}
    function automatic logic [31:0] I(logic [11:0] imm, logic [4:0] rs1,
                                       logic [2:0] f3, logic [4:0] rd,
                                       logic [6:0] opc);
        return {imm, rs1, f3, rd, opc};
    endfunction
    // S-type:  {imm[11:5], rs2, rs1, funct3, imm[4:0], opcode}
    function automatic logic [31:0] S(logic [11:0] imm, logic [4:0] rs2, rs1,
                                       logic [2:0] f3);
        return {imm[11:5], rs2, rs1, f3, imm[4:0], 7'b0100011};
    endfunction
    // B-type:  {imm[12|10:5], rs2, rs1, funct3, imm[4:1|11], opcode}
    function automatic logic [31:0] B(logic [12:0] imm, logic [4:0] rs2, rs1,
                                       logic [2:0] f3);
        return {imm[12], imm[10:5], rs2, rs1, f3, imm[4:1], imm[11], 7'b1100011};
    endfunction

    initial begin
        $display("=== Single-Cycle CPU Testbench ===");
        clk = 0;
        rst = 1;

        // Program:
        //   addi x1, x0, 5      ; x1 = 5
        //   addi x2, x0, 3      ; x2 = 3
        //   add  x3, x1, x2     ; x3 = 8
        //   sub  x4, x1, x2     ; x4 = 2
        //   and  x5, x1, x2     ; x5 = 1
        //   or   x6, x1, x2     ; x6 = 7
        //   slt  x7, x2, x1     ; x7 = 1 (3 < 5)
        //   sw   x3, 0(x0)      ; MEM[0] = 8
        //   lw   x8, 0(x0)      ; x8 = 8
        //   beq  x1, x1, +8     ; branch taken, skip next
        //   addi x9, x0, 99     ; skipped
        //   addi x10, x0, 42    ; x10 = 42 (branch target)

        // Load program into instruction memory
        dut.rom.mem[0]  = I(12'd5,  5'd0, 3'b000, 5'd1,  7'b0010011);  // addi x1,x0,5
        dut.rom.mem[1]  = I(12'd3,  5'd0, 3'b000, 5'd2,  7'b0010011);  // addi x2,x0,3
        dut.rom.mem[2]  = R(7'b0000000, 5'd2, 5'd1, 3'b000, 5'd3);     // add  x3,x1,x2
        dut.rom.mem[3]  = R(7'b0100000, 5'd2, 5'd1, 3'b000, 5'd4);     // sub  x4,x1,x2
        dut.rom.mem[4]  = R(7'b0000000, 5'd2, 5'd1, 3'b111, 5'd5);     // and  x5,x1,x2
        dut.rom.mem[5]  = R(7'b0000000, 5'd2, 5'd1, 3'b110, 5'd6);     // or   x6,x1,x2
        dut.rom.mem[6]  = R(7'b0000000, 5'd2, 5'd1, 3'b010, 5'd7);     // slt  x7,x2,x1
        dut.rom.mem[7]  = S(12'd0,  5'd3, 5'd0, 3'b010);                // sw   x3,0(x0)
        dut.rom.mem[8]  = I(12'd0,  5'd0, 3'b010, 5'd8,  7'b0000011);  // lw   x8,0(x0)
        // beq x1,x1,+8 → offset = 8 bytes, skip 2 instructions (mem[11] is target)
        dut.rom.mem[9]  = B(13'd8,  5'd1, 5'd1, 3'b000);                // beq  x1,x1,+8
        dut.rom.mem[10] = I(12'd99, 5'd0, 3'b000, 5'd9,  7'b0010011);  // addi x9,x0,99 (skipped)
        dut.rom.mem[11] = I(12'd42, 5'd0, 3'b000, 5'd10, 7'b0010011);  // addi x10,x0,42

        // Clear remaining ROM
        for (int i = 12; i < 64; i++) dut.rom.mem[i] = 32'h0;

        // Reset: 2 cycles
        @(posedge clk); @(posedge clk);
        rst = 0;

        // Run for 12 instruction cycles
        repeat (12) @(posedge clk);
        #1; // let combinational logic settle

        // Verify register file
        $display("x1  = %0d (expect 5)",  dut.rf.regs[1]);
        $display("x2  = %0d (expect 3)",  dut.rf.regs[2]);
        $display("x3  = %0d (expect 8)",  dut.rf.regs[3]);
        $display("x4  = %0d (expect 2)",  dut.rf.regs[4]);
        $display("x5  = %0d (expect 1)",  dut.rf.regs[5]);
        $display("x6  = %0d (expect 7)",  dut.rf.regs[6]);
        $display("x7  = %0d (expect 1)",  dut.rf.regs[7]);
        $display("x8  = %0d (expect 8)",  dut.rf.regs[8]);
        $display("x9  = %0d (expect 0 — skipped by branch)", dut.rf.regs[9]);
        $display("x10 = %0d (expect 42)", dut.rf.regs[10]);
        $display("MEM[0] = %0d (expect 8)", dut.ram.mem[0]);

        assert(dut.rf.regs[1]  == 32'd5)  else $error("x1 wrong");
        assert(dut.rf.regs[2]  == 32'd3)  else $error("x2 wrong");
        assert(dut.rf.regs[3]  == 32'd8)  else $error("x3 wrong: add");
        assert(dut.rf.regs[4]  == 32'd2)  else $error("x4 wrong: sub");
        assert(dut.rf.regs[5]  == 32'd1)  else $error("x5 wrong: and");
        assert(dut.rf.regs[6]  == 32'd7)  else $error("x6 wrong: or");
        assert(dut.rf.regs[7]  == 32'd1)  else $error("x7 wrong: slt");
        assert(dut.ram.mem[0]  == 32'd8)  else $error("sw failed");
        assert(dut.rf.regs[8]  == 32'd8)  else $error("x8 wrong: lw");
        assert(dut.rf.regs[9]  == 32'd0)  else $error("x9 should be 0 (skipped by beq)");
        assert(dut.rf.regs[10] == 32'd42) else $error("x10 wrong");

        $display("=== All tests passed ===");
        $finish;
    end
endmodule
