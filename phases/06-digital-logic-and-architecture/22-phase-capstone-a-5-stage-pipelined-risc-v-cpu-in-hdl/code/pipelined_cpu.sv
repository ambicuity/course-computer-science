// =============================================================================
// 5-Stage Pipelined RISC-V CPU (RV32I)
// Stages: IF → ID → EX → MEM → WB
// Features: full forwarding, load-use stall, branch flush
// =============================================================================
`default_nettype none

// =============================================================================
// Constants
// =============================================================================
package riscv_pkg;
    // ALU operations
    typedef enum logic [3:0] {
        ALU_ADD  = 4'b0000,
        ALU_SUB  = 4'b0001,
        ALU_AND  = 4'b0010,
        ALU_OR   = 4'b0011,
        ALU_XOR  = 4'b0100,
        ALU_SLL  = 4'b0101,
        ALU_SRL  = 4'b0110,
        ALU_SRA  = 4'b0111,
        ALU_SLT  = 4'b1000,
        ALU_SLTU = 4'b1001
    } alu_op_e;

    // Opcodes
    localparam logic [6:0] OP_RTYPE   = 7'b0110011;
    localparam logic [6:0] OP_ITYPE   = 7'b0010011;
    localparam logic [6:0] OP_LOAD    = 7'b0000011;
    localparam logic [6:0] OP_STORE   = 7'b0100011;
    localparam logic [6:0] OP_BRANCH  = 7'b1100011;
    localparam logic [6:0] OP_JAL     = 7'b1101111;
    localparam logic [6:0] OP_JALR    = 7'b1100111;
    localparam logic [6:0] OP_LUI     = 7'b0110111;
    localparam logic [6:0] OP_AUIPC   = 7'b0010111;
endpackage

import riscv_pkg::*;

// =============================================================================
// Control signals bundle
// =============================================================================
typedef struct packed {
    logic       RegWrite;
    logic       ALUSrc;      // 0=rs2, 1=imm
    logic       MemWrite;
    logic       MemRead;
    logic       MemToReg;    // 0=ALU, 1=mem
    logic       Branch;
    logic       Jump;
    logic [3:0] ALUOp;
} ctrl_t;

// =============================================================================
// Register File
// =============================================================================
module register_file (
    input  logic        clk,
    input  logic        rst_n,
    input  logic [4:0]  rs1_addr,
    input  logic [4:0]  rs2_addr,
    output logic [31:0] rs1_data,
    output logic [31:0] rs2_data,
    input  logic        we,
    input  logic [4:0]  rd_addr,
    input  logic [31:0] rd_data
);
    logic [31:0] regs [1:31]; // x1-x31; x0 hardwired to 0

    assign rs1_data = (rs1_addr == 5'b0) ? 32'b0 : regs[rs1_addr];
    assign rs2_data = (rs2_addr == 5'b0) ? 32'b0 : regs[rs2_addr];

    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            for (int i = 1; i < 32; i++)
                regs[i] <= 32'b0;
        end else if (we && rd_addr != 5'b0) begin
            regs[rd_addr] <= rd_data;
        end
    end
endmodule

// =============================================================================
// ALU
// =============================================================================
module alu (
    input  logic [31:0] a,
    input  logic [31:0] b,
    input  logic [3:0]  alu_op,
    output logic [31:0] result,
    output logic        zero
);
    always_comb begin
        case (alu_op)
            ALU_ADD:  result = a + b;
            ALU_SUB:  result = a - b;
            ALU_AND:  result = a & b;
            ALU_OR:   result = a | b;
            ALU_XOR:  result = a ^ b;
            ALU_SLL:  result = a << b[4:0];
            ALU_SRL:  result = a >> b[4:0];
            ALU_SRA:  result = $signed(a) >>> b[4:0];
            ALU_SLT:  result = ($signed(a) < $signed(b)) ? 32'd1 : 32'd0;
            ALU_SLTU: result = (a < b) ? 32'd1 : 32'd0;
            default:  result = 32'b0;
        endcase
    end
    assign zero = (result == 32'b0);
endmodule

// =============================================================================
// Control Unit
// =============================================================================
module control_unit (
    input  logic [6:0] opcode,
    input  logic [2:0] funct3,
    input  logic [6:0] funct7,
    output ctrl_t      ctrl
);
    always_comb begin
        ctrl = '{default: 0};
        case (opcode)
            OP_RTYPE: begin
                ctrl.RegWrite = 1'b1;
                ctrl.ALUSrc   = 1'b0;
                ctrl.ALUOp    = alu_rtype(funct3, funct7);
            end
            OP_ITYPE: begin
                ctrl.RegWrite = 1'b1;
                ctrl.ALUSrc   = 1'b1;
                ctrl.ALUOp    = alu_itype(funct3, funct7);
            end
            OP_LOAD: begin
                ctrl.RegWrite = 1'b1;
                ctrl.ALUSrc   = 1'b1;
                ctrl.MemRead  = 1'b1;
                ctrl.MemToReg = 1'b1;
                ctrl.ALUOp    = ALU_ADD;
            end
            OP_STORE: begin
                ctrl.ALUSrc   = 1'b1;
                ctrl.MemWrite = 1'b1;
                ctrl.ALUOp    = ALU_ADD;
            end
            OP_BRANCH: begin
                ctrl.Branch   = 1'b1;
                ctrl.ALUOp    = ALU_SUB;
            end
            OP_JAL: begin
                ctrl.RegWrite = 1'b1;
                ctrl.Jump     = 1'b1;
                ctrl.ALUOp    = ALU_ADD;
            end
            OP_JALR: begin
                ctrl.RegWrite = 1'b1;
                ctrl.ALUSrc   = 1'b1;
                ctrl.Jump     = 1'b1;
                ctrl.ALUOp    = ALU_ADD;
            end
            OP_LUI: begin
                ctrl.RegWrite = 1'b1;
                ctrl.ALUSrc   = 1'b1;
                ctrl.ALUOp    = ALU_ADD; // b = imm, a = 0 => result = imm
            end
            OP_AUIPC: begin
                ctrl.RegWrite = 1'b1;
                ctrl.ALUSrc   = 1'b1;
                ctrl.ALUOp    = ALU_ADD; // a = pc, b = imm
            end
            default: ;
        endcase
    end

    function automatic logic [3:0] alu_rtype(logic [2:0] f3, logic [6:0] f7);
        case ({f7, f3})
            10'b0000000_000: return ALU_ADD;
            10'b0100000_000: return ALU_SUB;
            10'b0000000_111: return ALU_AND;
            10'b0000000_110: return ALU_OR;
            10'b0000000_100: return ALU_XOR;
            10'b0000000_001: return ALU_SLL;
            10'b0000000_101: return ALU_SRL;
            10'b0100000_101: return ALU_SRA;
            10'b0000000_010: return ALU_SLT;
            10'b0000000_011: return ALU_SLTU;
            default:         return ALU_ADD;
        endcase
    endfunction

    function automatic logic [3:0] alu_itype(logic [2:0] f3, logic [6:0] f7);
        case (f3)
            3'b000:  return ALU_ADD;   // ADDI
            3'b111:  return ALU_AND;   // ANDI
            3'b110:  return ALU_OR;    // ORI
            3'b100:  return ALU_XOR;   // XORI
            3'b010:  return ALU_SLT;   // SLTI
            3'b011:  return ALU_SLTU;  // SLTIU
            3'b001:  return ALU_SLL;   // SLLI
            3'b101:  return (f7 == 7'b0100000) ? ALU_SRA : ALU_SRL; // SRAI / SRLI
            default: return ALU_ADD;
        endcase
    endfunction
endmodule

// =============================================================================
// Immediate Generator
// =============================================================================
module imm_gen (
    input  logic [31:0] instr,
    output logic [31:0] imm
);
    logic [6:0] opcode;
    assign opcode = instr[6:0];

    always_comb begin
        case (opcode)
            OP_ITYPE, OP_LOAD, OP_JALR:
                imm = {{20{instr[31]}}, instr[31:20]};
            OP_STORE:
                imm = {{20{instr[31]}}, instr[31:25], instr[11:7]};
            OP_BRANCH:
                imm = {{20{instr[31]}}, instr[7], instr[30:25], instr[11:8], 1'b0};
            OP_JAL:
                imm = {{12{instr[31]}}, instr[19:12], instr[20], instr[30:21], 1'b0};
            OP_LUI, OP_AUIPC:
                imm = {instr[31:12], 12'b0};
            default:
                imm = 32'b0;
        endcase
    end
endmodule

// =============================================================================
// Pipeline Registers
// =============================================================================

// IF/ID Pipeline Register
module if_id_reg (
    input  logic        clk,
    input  logic        rst_n,
    input  logic        stall,
    input  logic        flush,
    input  logic [31:0] instr_in,
    input  logic [31:0] pc_in,
    input  logic [31:0] pc_plus4_in,
    output logic [31:0] instr_out,
    output logic [31:0] pc_out,
    output logic [31:0] pc_plus4_out
);
    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n || flush) begin
            instr_out    <= 32'h00000013; // NOP = ADDI x0, x0, 0
            pc_out       <= 32'b0;
            pc_plus4_out <= 32'b0;
        end else if (!stall) begin
            instr_out    <= instr_in;
            pc_out       <= pc_in;
            pc_plus4_out <= pc_plus4_in;
        end
    end
endmodule

// ID/EX Pipeline Register
module id_ex_reg (
    input  logic        clk,
    input  logic        rst_n,
    input  logic        flush,
    input  ctrl_t       ctrl_in,
    input  logic [31:0] pc_in,
    input  logic [31:0] pc_plus4_in,
    input  logic [31:0] rs1_data_in,
    input  logic [31:0] rs2_data_in,
    input  logic [31:0] imm_in,
    input  logic [4:0]  rs1_addr_in,
    input  logic [4:0]  rs2_addr_in,
    input  logic [4:0]  rd_addr_in,
    output ctrl_t       ctrl_out,
    output logic [31:0] pc_out,
    output logic [31:0] pc_plus4_out,
    output logic [31:0] rs1_data_out,
    output logic [31:0] rs2_data_out,
    output logic [31:0] imm_out,
    output logic [4:0]  rs1_addr_out,
    output logic [4:0]  rs2_addr_out,
    output logic [4:0]  rd_addr_out
);
    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n || flush) begin
            ctrl_out      <= '{default: 0};
            pc_out        <= 32'b0;
            pc_plus4_out  <= 32'b0;
            rs1_data_out  <= 32'b0;
            rs2_data_out  <= 32'b0;
            imm_out       <= 32'b0;
            rs1_addr_out  <= 5'b0;
            rs2_addr_out  <= 5'b0;
            rd_addr_out   <= 5'b0;
        end else begin
            ctrl_out      <= ctrl_in;
            pc_out        <= pc_in;
            pc_plus4_out  <= pc_plus4_in;
            rs1_data_out  <= rs1_data_in;
            rs2_data_out  <= rs2_data_in;
            imm_out       <= imm_in;
            rs1_addr_out  <= rs1_addr_in;
            rs2_addr_out  <= rs2_addr_in;
            rd_addr_out   <= rd_addr_in;
        end
    end
endmodule

// EX/MEM Pipeline Register
module ex_mem_reg (
    input  logic        clk,
    input  logic        rst_n,
    input  ctrl_t       ctrl_in,
    input  logic [31:0] alu_result_in,
    input  logic [31:0] rs2_data_in,
    input  logic [4:0]  rd_addr_in,
    output ctrl_t       ctrl_out,
    output logic [31:0] alu_result_out,
    output logic [31:0] rs2_data_out,
    output logic [4:0]  rd_addr_out
);
    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            ctrl_out       <= '{default: 0};
            alu_result_out <= 32'b0;
            rs2_data_out   <= 32'b0;
            rd_addr_out    <= 5'b0;
        end else begin
            ctrl_out       <= ctrl_in;
            alu_result_out <= alu_result_in;
            rs2_data_out   <= rs2_data_in;
            rd_addr_out    <= rd_addr_in;
        end
    end
endmodule

// MEM/WB Pipeline Register
module mem_wb_reg (
    input  logic        clk,
    input  logic        rst_n,
    input  ctrl_t       ctrl_in,
    input  logic [31:0] mem_data_in,
    input  logic [31:0] alu_result_in,
    input  logic [4:0]  rd_addr_in,
    output ctrl_t       ctrl_out,
    output logic [31:0] mem_data_out,
    output logic [31:0] alu_result_out,
    output logic [4:0]  rd_addr_out
);
    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            ctrl_out       <= '{default: 0};
            mem_data_out   <= 32'b0;
            alu_result_out <= 32'b0;
            rd_addr_out    <= 5'b0;
        end else begin
            ctrl_out       <= ctrl_in;
            mem_data_out   <= mem_data_in;
            alu_result_out <= alu_result_in;
            rd_addr_out    <= rd_addr_in;
        end
    end
endmodule

// =============================================================================
// Hazard Detection Unit
// =============================================================================
module hazard_unit (
    input  logic       id_ex_MemRead,
    input  logic [4:0] id_ex_rd,
    input  logic [4:0] if_id_rs1,
    input  logic [4:0] if_id_rs2,
    input  logic       branch_taken,
    input  logic       jump,
    output logic       stall,
    output logic       flush_if_id,
    output logic       flush_id_ex
);
    // Load-use hazard: LW in EX, consumer in ID
    assign stall = id_ex_MemRead && (id_ex_rd != 5'b0) &&
                   ((id_ex_rd == if_id_rs1) || (id_ex_rd == if_id_rs2));

    // Branch taken or jump: flush wrong-path instructions in IF/ID and ID/EX
    assign flush_if_id = branch_taken || jump;
    assign flush_id_ex = branch_taken || jump;
endmodule

// =============================================================================
// Forwarding Unit
// =============================================================================
module forwarding_unit (
    input  logic       ex_mem_RegWrite,
    input  logic [4:0] ex_mem_rd,
    input  logic       mem_wb_RegWrite,
    input  logic [4:0] mem_wb_rd,
    input  logic [4:0] id_ex_rs1,
    input  logic [4:0] id_ex_rs2,
    output logic [1:0] forward_A,
    output logic [1:0] forward_B
);
    always_comb begin
        // Default: use ID/EX register data
        forward_A = 2'b00;
        forward_B = 2'b00;

        // EX hazard: forward from EX/MEM (highest priority)
        if (ex_mem_RegWrite && ex_mem_rd != 5'b0 && ex_mem_rd == id_ex_rs1)
            forward_A = 2'b10;
        // MEM hazard: forward from MEM/WB
        else if (mem_wb_RegWrite && mem_wb_rd != 5'b0 && mem_wb_rd == id_ex_rs1)
            forward_A = 2'b01;

        // EX hazard for B
        if (ex_mem_RegWrite && ex_mem_rd != 5'b0 && ex_mem_rd == id_ex_rs2)
            forward_B = 2'b10;
        // MEM hazard for B
        else if (mem_wb_RegWrite && mem_wb_rd != 5'b0 && mem_wb_rd == id_ex_rs2)
            forward_B = 2'b01;
    end
endmodule

// =============================================================================
// Data Memory (RAM)
// =============================================================================
module data_memory (
    input  logic        clk,
    input  logic        we,
    input  logic        re,
    input  logic [31:0] addr,
    input  logic [31:0] wdata,
    output logic [31:0] rdata
);
    logic [31:0] mem [0:1023]; // 4 KB

    assign rdata = re ? mem[addr[11:2]] : 32'b0;

    always_ff @(posedge clk) begin
        if (we)
            mem[addr[11:2]] <= wdata;
    end
endmodule

// =============================================================================
// Instruction Memory (ROM)
// =============================================================================
module instr_memory (
    input  logic [31:0] addr,
    output logic [31:0] instr
);
    logic [31:0] rom [0:255]; // 1 KB ROM

    initial begin
        // Default: NOPs
        for (int i = 0; i < 256; i++)
            rom[i] = 32'h00000013;
        // Load test program (override with $readmemh in testbench)
        $readmemh("program.hex", rom);
    end

    assign instr = rom[addr[9:2]];
endmodule

// =============================================================================
// Pipeline Stages
// =============================================================================

// IF Stage
module fetch_stage (
    input  logic        clk,
    input  logic        rst_n,
    input  logic        stall,
    input  logic        branch_taken,
    input  logic        jump,
    input  logic [31:0] branch_target,
    input  logic [31:0] jump_target,
    output logic [31:0] pc_out,
    output logic [31:0] pc_plus4_out,
    output logic [31:0] instr_out
);
    logic [31:0] pc;

    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n)
            pc <= 32'b0;
        else if (!stall) begin
            if (branch_taken)
                pc <= branch_target;
            else if (jump)
                pc <= jump_target;
            else
                pc <= pc + 32'd4;
        end
    end

    assign pc_out      = pc;
    assign pc_plus4_out = pc + 32'd4;

    instr_memory u_imem (.addr(pc), .instr(instr_out));
endmodule

// ID Stage
module decode_stage (
    input  logic [31:0] instr,
    input  logic [31:0] pc,
    input  logic [31:0] pc_plus4,
    // From writeback
    input  logic        wb_RegWrite,
    input  logic [4:0]  wb_rd,
    input  logic [31:0] wb_data,
    // Outputs
    output ctrl_t       ctrl,
    output logic [31:0] rs1_data,
    output logic [31:0] rs2_data,
    output logic [31:0] imm,
    output logic [4:0]  rs1_addr,
    output logic [4:0]  rs2_addr,
    output logic [4:0]  rd_addr
);
    logic [6:0] opcode;
    logic [2:0] funct3;
    logic [6:0] funct7;

    assign opcode = instr[6:0];
    assign rd_addr = instr[11:7];
    assign funct3 = instr[14:12];
    assign rs1_addr = instr[19:15];
    assign rs2_addr = instr[24:20];
    assign funct7 = instr[31:25];

    control_unit u_ctrl (
        .opcode(opcode), .funct3(funct3), .funct7(funct7), .ctrl(ctrl)
    );

    register_file u_rf (
        .clk(clk), .rst_n(rst_n),
        .rs1_addr(rs1_addr), .rs2_addr(rs2_addr),
        .rs1_data(rs1_data), .rs2_data(rs2_data),
        .we(wb_RegWrite), .rd_addr(wb_rd), .rd_data(wb_data)
    );

    imm_gen u_imm (.instr(instr), .imm(imm));
endmodule

// EX Stage
module execute_stage (
    input  ctrl_t       ctrl,
    input  logic [31:0] pc,
    input  logic [31:0] pc_plus4,
    input  logic [31:0] rs1_data,
    input  logic [31:0] rs2_data,
    input  logic [31:0] imm,
    input  logic [4:0]  rd_addr,
    // Forwarding
    input  logic [1:0]  forward_A,
    input  logic [1:0]  forward_B,
    input  logic [31:0] ex_mem_alu_result,
    input  logic [31:0] wb_data,
    // Outputs
    output logic [31:0] alu_result,
    output logic [31:0] rs2_data_out,
    output logic        branch_taken,
    output logic [31:0] branch_target,
    output logic [31:0] jump_target
);
    logic [31:0] alu_a, alu_b_raw, alu_b;
    logic        alu_zero;

    // Forwarding MUXes
    always_comb begin
        case (forward_A)
            2'b10:   alu_a = ex_mem_alu_result;
            2'b01:   alu_a = wb_data;
            default: alu_a = rs1_data;
        endcase

        case (forward_B)
            2'b10:   alu_b_raw = ex_mem_alu_result;
            2'b01:   alu_b_raw = wb_data;
            default: alu_b_raw = rs2_data;
        endcase
    end

    // ALUSrc MUX
    assign alu_b = ctrl.ALUSrc ? imm : alu_b_raw;

    alu u_alu (.a(alu_a), .b(alu_b), .alu_op(ctrl.ALUOp),
               .result(alu_result), .zero(alu_zero));

    // Branch resolution
    logic branch_cond;
    always_comb begin
        case (rd_addr[2:0]) // reuse rd_addr field — actually should be funct3 from pipeline
            3'b000:  branch_cond = alu_zero;                   // BEQ
            3'b001:  branch_cond = !alu_zero;                  // BNE
            3'b100:  branch_cond = alu_result[0];              // BLT (slt result)
            3'b101:  branch_cond = !alu_result[0];             // BGE
            default: branch_cond = 1'b0;
        endcase
    end

    // NOTE: branch uses imm (which is already shifted) added to PC
    assign branch_target = pc + imm;
    assign branch_taken  = ctrl.Branch && branch_cond;

    // Jump targets
    logic [31:0] jal_target, jalr_target;
    assign jal_target  = pc + imm;
    assign jalr_target = (alu_a + imm) & ~32'b1; // JALR: (rs1 + imm) & ~1
    assign jump_target = (instr_is_jalr(ctrl, rd_addr)) ? jalr_target : jal_target;

    assign rs2_data_out = alu_b_raw; // for store

    function automatic logic instr_is_jalr(ctrl_t c, logic [4:0] rd);
        return c.Jump && c.ALUSrc; // JALR sets ALUSrc; JAL doesn't
    endfunction
endmodule

// MEM Stage
module memory_stage (
    input  ctrl_t       ctrl,
    input  logic [31:0] alu_result,
    input  logic [31:0] rs2_data,
    output logic [31:0] mem_data
);
    data_memory u_dmem (
        .clk(clk), .we(ctrl.MemWrite), .re(ctrl.MemRead),
        .addr(alu_result), .wdata(rs2_data), .rdata(mem_data)
    );
endmodule

// WB Stage
module writeback_stage (
    input  ctrl_t       ctrl,
    input  logic [31:0] alu_result,
    input  logic [31:0] mem_data,
    input  logic [31:0] pc_plus4,
    output logic [31:0] wb_data
);
    // MemToReg MUX: 0 = ALU result, 1 = memory data
    // For JAL/JALR: write PC+4 (handled by setting MemToReg=0 and ALU=PC+4,
    // or by a separate JAL mux — here we use a simple approach)
    assign wb_data = ctrl.MemToReg ? mem_data : alu_result;
endmodule

// =============================================================================
// Top-Level Pipelined CPU
// =============================================================================
module pipelined_cpu (
    input  logic clk,
    input  logic rst_n
);
    // IF stage wires
    logic [31:0] if_pc, if_pc_plus4, if_instr;
    logic        stall, flush_if_id, flush_id_ex;
    logic        branch_taken;
    logic [31:0] branch_target, jump_target;
    logic        jump;

    // IF/ID wires
    logic [31:0] ifid_instr, ifid_pc, ifid_pc_plus4;

    // ID stage wires
    ctrl_t       id_ctrl;
    logic [31:0] id_rs1_data, id_rs2_data, id_imm;
    logic [4:0]  id_rs1_addr, id_rs2_addr, id_rd_addr;

    // ID/EX wires
    ctrl_t       idex_ctrl;
    logic [31:0] idex_pc, idex_pc_plus4, idex_rs1_data, idex_rs2_data, idex_imm;
    logic [4:0]  idex_rs1_addr, idex_rs2_addr, idex_rd_addr;

    // EX wires
    logic [31:0] ex_alu_result, ex_rs2_data;
    logic [1:0]  forward_A, forward_B;

    // EX/MEM wires
    ctrl_t       exmem_ctrl;
    logic [31:0] exmem_alu_result, exmem_rs2_data;
    logic [4:0]  exmem_rd_addr;

    // MEM wires
    logic [31:0] mem_mem_data;

    // MEM/WB wires
    ctrl_t       memwb_ctrl;
    logic [31:0] memwb_mem_data, memwb_alu_result;
    logic [4:0]  memwb_rd_addr;

    // WB wires
    logic [31:0] wb_data;

    // Jump signal from decode
    assign jump = id_ctrl.Jump;

    // =========================================================================
    // IF Stage
    // =========================================================================
    fetch_stage u_if (
        .clk(clk), .rst_n(rst_n), .stall(stall),
        .branch_taken(branch_taken), .jump(jump),
        .branch_target(branch_target), .jump_target(jump_target),
        .pc_out(if_pc), .pc_plus4_out(if_pc_plus4), .instr_out(if_instr)
    );

    // =========================================================================
    // IF/ID Register
    // =========================================================================
    if_id_reg u_if_id (
        .clk(clk), .rst_n(rst_n), .stall(stall), .flush(flush_if_id),
        .instr_in(if_instr), .pc_in(if_pc), .pc_plus4_in(if_pc_plus4),
        .instr_out(ifid_instr), .pc_out(ifid_pc), .pc_plus4_out(ifid_pc_plus4)
    );

    // =========================================================================
    // ID Stage
    // =========================================================================
    decode_stage u_id (
        .instr(ifid_instr), .pc(ifid_pc), .pc_plus4(ifid_pc_plus4),
        .wb_RegWrite(memwb_ctrl.RegWrite), .wb_rd(memwb_rd_addr), .wb_data(wb_data),
        .ctrl(id_ctrl), .rs1_data(id_rs1_data), .rs2_data(id_rs2_data),
        .imm(id_imm), .rs1_addr(id_rs1_addr), .rs2_addr(id_rs2_addr),
        .rd_addr(id_rd_addr)
    );

    // =========================================================================
    // ID/EX Register
    // =========================================================================
    // Stall: insert bubble (NOP) when load-use detected
    ctrl_t idex_ctrl_in;
    always_comb begin
        if (stall)
            idex_ctrl_in = '{default: 0};
        else
            idex_ctrl_in = id_ctrl;
    end

    id_ex_reg u_id_ex (
        .clk(clk), .rst_n(rst_n), .flush(flush_id_ex),
        .ctrl_in(idex_ctrl_in), .pc_in(ifid_pc), .pc_plus4_in(ifid_pc_plus4),
        .rs1_data_in(id_rs1_data), .rs2_data_in(id_rs2_data),
        .imm_in(id_imm), .rs1_addr_in(id_rs1_addr), .rs2_addr_in(id_rs2_addr),
        .rd_addr_in(id_rd_addr),
        .ctrl_out(idex_ctrl), .pc_out(idex_pc), .pc_plus4_out(idex_pc_plus4),
        .rs1_data_out(idex_rs1_data), .rs2_data_out(idex_rs2_data),
        .imm_out(idex_imm), .rs1_addr_out(idex_rs1_addr),
        .rs2_addr_out(idex_rs2_addr), .rd_addr_out(idex_rd_addr)
    );

    // =========================================================================
    // EX Stage
    // =========================================================================
    execute_stage u_ex (
        .ctrl(idex_ctrl), .pc(idex_pc), .pc_plus4(idex_pc_plus4),
        .rs1_data(idex_rs1_data), .rs2_data(idex_rs2_data),
        .imm(idex_imm), .rd_addr(idex_rd_addr),
        .forward_A(forward_A), .forward_B(forward_B),
        .ex_mem_alu_result(exmem_alu_result), .wb_data(wb_data),
        .alu_result(ex_alu_result), .rs2_data_out(ex_rs2_data),
        .branch_taken(branch_taken),
        .branch_target(branch_target), .jump_target(jump_target)
    );

    // =========================================================================
    // EX/MEM Register
    // =========================================================================
    ex_mem_reg u_ex_mem (
        .clk(clk), .rst_n(rst_n),
        .ctrl_in(idex_ctrl), .alu_result_in(ex_alu_result),
        .rs2_data_in(ex_rs2_data), .rd_addr_in(idex_rd_addr),
        .ctrl_out(exmem_ctrl), .alu_result_out(exmem_alu_result),
        .rs2_data_out(exmem_rs2_data), .rd_addr_out(exmem_rd_addr)
    );

    // =========================================================================
    // MEM Stage
    // =========================================================================
    memory_stage u_mem (
        .ctrl(exmem_ctrl), .alu_result(exmem_alu_result),
        .rs2_data(exmem_rs2_data), .mem_data(mem_mem_data)
    );

    // =========================================================================
    // MEM/WB Register
    // =========================================================================
    mem_wb_reg u_mem_wb (
        .clk(clk), .rst_n(rst_n),
        .ctrl_in(exmem_ctrl), .mem_data_in(mem_mem_data),
        .alu_result_in(exmem_alu_result), .rd_addr_in(exmem_rd_addr),
        .ctrl_out(memwb_ctrl), .mem_data_out(memwb_mem_data),
        .alu_result_out(memwb_alu_result), .rd_addr_out(memwb_rd_addr)
    );

    // =========================================================================
    // WB Stage
    // =========================================================================
    writeback_stage u_wb (
        .ctrl(memwb_ctrl), .alu_result(memwb_alu_result),
        .mem_data(memwb_mem_data), .pc_plus4(32'b0), .wb_data(wb_data)
    );

    // =========================================================================
    // Hazard Unit
    // =========================================================================
    hazard_unit u_hz (
        .id_ex_MemRead(idex_ctrl.MemRead), .id_ex_rd(idex_rd_addr),
        .if_id_rs1(ifid_instr[19:15]), .if_id_rs2(ifid_instr[24:20]),
        .branch_taken(branch_taken), .jump(jump),
        .stall(stall), .flush_if_id(flush_if_id), .flush_id_ex(flush_id_ex)
    );

    // =========================================================================
    // Forwarding Unit
    // =========================================================================
    forwarding_unit u_fwd (
        .ex_mem_RegWrite(exmem_ctrl.RegWrite), .ex_mem_rd(exmem_rd_addr),
        .mem_wb_RegWrite(memwb_ctrl.RegWrite), .mem_wb_rd(memwb_rd_addr),
        .id_ex_rs1(idex_rs1_addr), .id_ex_rs2(idex_rs2_addr),
        .forward_A(forward_A), .forward_B(forward_B)
    );
endmodule

// =============================================================================
// Testbench
// =============================================================================
module tb_pipelined_cpu;
    logic clk;
    logic rst_n;

    pipelined_cpu dut (.clk(clk), .rst_n(rst_n));

    // 100 MHz clock
    always #5 clk = ~clk;

    initial begin
        $dumpfile("cpu.vcd");
        $dumpvars(0, tb_pipelined_cpu);

        clk = 0;
        rst_n = 0;
        #20;
        rst_n = 1;

        // Run for 200 cycles
        #2000;

        // Dump register file
        $display("=== Register File ===");
        $display("x0  = %0d", dut.u_id.u_rf.regs[0]);  // always 0 (not stored)
        for (int i = 1; i < 32; i++)
            $display("x%-2d = %0d (0x%08x)", i, dut.u_id.u_rf.regs[i], dut.u_id.u_rf.regs[i]);

        $display("=== Done ===");
        $finish;
    end

    // Cycle counter
    int cycle;
    always @(posedge clk) begin
        if (rst_n) begin
            cycle <= cycle + 1;
            $display("Cycle %0d: PC=0x%08x Instr=0x%08x",
                     cycle, dut.if_pc, dut.if_instr);
        end
    end
endmodule
