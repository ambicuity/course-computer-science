// =============================================================================
// 5-Stage Pipeline with Forwarding and Hazard Detection
// =============================================================================

// --- Control signals bundle (ID/EX pipeline register fields) ---------------
typedef struct packed {
    logic       reg_write;    // write to register file
    logic       mem_to_reg;   // result from memory (load)
    logic       mem_read;     // read data memory
    logic       mem_write;    // write data memory
    logic       alu_src;      // ALU source: 0=reg, 1=imm
    logic       branch;       // is branch instruction
    logic [1:0] alu_op;       // ALU operation encoding
    logic       is_load;      // specifically a load (for hazard detection)
} ctrl_t;

// --- Opcodes (subset) ------------------------------------------------------
localparam logic [6:0]
    OP_RTYPE   = 7'b0110011,
    OP_ITYPE   = 7'b0010011,
    OP_LOAD    = 7'b0000011,
    OP_STORE   = 7'b0100011,
    OP_BRANCH  = 7'b1100011;

// =============================================================================
// Hazard Detection Unit
// =============================================================================
module hazard_detection (
    input  logic [4:0] id_ex_rd,
    input  logic       id_ex_mem_read,
    input  logic [4:0] if_id_rs1,
    input  logic [4:0] if_id_rs2,
    output logic       stall           // 1 = insert bubble
);
    // Load-use hazard: EX stage is a load, and the next instruction (in ID)
    // reads the register being loaded.  Forwarding cannot help because the
    // load result is not available until the end of MEM.
    always_comb begin
        stall = 1'b0;
        if (id_ex_mem_read &&
            (id_ex_rd != 5'd0) &&
            ((id_ex_rd == if_id_rs1) || (id_ex_rd == if_id_rs2))) begin
            stall = 1'b1;
        end
    end
endmodule

// =============================================================================
// Forwarding Unit
// =============================================================================
module forwarding_unit (
    input  logic [4:0] ex_mem_rd,
    input  logic       ex_mem_reg_write,
    input  logic [4:0] mem_wb_rd,
    input  logic       mem_wb_reg_write,
    input  logic [4:0] id_ex_rs1,
    input  logic [4:0] id_ex_rs2,
    output logic [1:0] forward_a,      // 00=reg, 01=MEM/WB, 10=EX/MEM
    output logic [1:0] forward_b
);
    always_comb begin
        // Default: read from register file
        forward_a = 2'b00;
        forward_b = 2'b00;

        // --- EX hazard (higher priority: result is fresher) ---------------
        if (ex_mem_reg_write && ex_mem_rd != 5'd0 &&
            ex_mem_rd == id_ex_rs1)
            forward_a = 2'b10;

        if (ex_mem_reg_write && ex_mem_rd != 5'd0 &&
            ex_mem_rd == id_ex_rs2)
            forward_b = 2'b10;

        // --- MEM hazard (only if EX hazard didn't match) ------------------
        if (mem_wb_reg_write && mem_wb_rd != 5'd0 &&
            !(ex_mem_reg_write && ex_mem_rd != 5'd0 && ex_mem_rd == id_ex_rs1) &&
            mem_wb_rd == id_ex_rs1)
            forward_a = 2'b01;

        if (mem_wb_reg_write && mem_wb_rd != 5'd0 &&
            !(ex_mem_reg_write && ex_mem_rd != 5'd0 && ex_mem_rd == id_ex_rs2) &&
            mem_wb_rd == id_ex_rs2)
            forward_b = 2'b01;
    end
endmodule

// =============================================================================
// ALU (simple: ADD, SUB, AND, OR)
// =============================================================================
module alu (
    input  logic [31:0] a,
    input  logic [31:0] b,
    input  logic [1:0]  alu_op,
    output logic [31:0] result,
    output logic        zero
);
    always_comb begin
        case (alu_op)
            2'b00: result = a + b;    // ADD (also load/store address)
            2'b01: result = a - b;    // SUB
            2'b10: result = a & b;    // AND
            2'b11: result = a | b;    // OR
        endcase
    end
    assign zero = (result == 32'd0);
endmodule

// =============================================================================
// Register File (read combinational, write on clock edge)
// =============================================================================
module regfile (
    input  logic        clk,
    input  logic [4:0]  rs1_addr,
    input  logic [4:0]  rs2_addr,
    input  logic [4:0]  rd_addr,
    input  logic [31:0] rd_data,
    input  logic        rd_write,
    output logic [31:0] rs1_data,
    output logic [31:0] rs2_data
);
    logic [31:0] regs [0:31];

    // Read (combinational)
    assign rs1_data = (rs1_addr == 5'd0) ? 32'd0 : regs[rs1_addr];
    assign rs2_data = (rs2_addr == 5'd0) ? 32'd0 : regs[rs2_addr];

    // Write (clock edge)
    always_ff @(posedge clk) begin
        if (rd_write && rd_addr != 5'd0)
            regs[rd_addr] <= rd_data;
    end
endmodule

// =============================================================================
// Pipeline CPU Top Module
// =============================================================================
module pipeline_cpu (
    input  logic        clk,
    input  logic        rst_n,
    output logic [31:0] debug_pc
);
    // --- Instruction memory (synchronous ROM) -----------------------------
    logic [31:0] instr_mem [0:255];
    initial $readmemh("program.hex", instr_mem);

    // --- Data memory (synchronous RAM) ------------------------------------
    logic [31:0] data_mem [0:255];
    logic [31:0] mem_rdata;
    always_ff @(posedge clk) begin
        if (mem_write_en && mem_addr[31:2] < 256)
            data_mem[mem_addr[31:2]] <= mem_wdata;
        mem_rdata <= data_mem[mem_addr[31:2]];
    end

    // =========================================================================
    // Pipeline registers
    // =========================================================================
    // IF/ID
    logic [31:0] ifid_instr, ifid_pc4;
    // ID/EX
    logic [31:0] idex_pc4, idex_rs1, idex_rs2, idex_imm;
    logic [4:0]  idex_rs1_addr, idex_rs2_addr, idex_rd;
    ctrl_t       idex_ctrl;
    // EX/MEM
    logic [31:0] exmem_alu, exmem_rs2;
    logic [4:0]  exmem_rd;
    ctrl_t       exmem_ctrl;
    // MEM/WB
    logic [31:0] memwb_alu, memwb_mem;
    logic [4:0]  memwb_rd;
    logic        memwb_reg_write, memwb_mem_to_reg;

    // =========================================================================
    // Wires
    // =========================================================================
    logic [31:0] pc, pc_next, pc_plus4;
    logic [31:0] if_instr;
    logic [31:0] id_rs1_data, id_rs2_data, id_imm;
    logic [4:0]  id_rs1, id_rs2, id_rd;
    logic [6:0]  id_opcode;
    ctrl_t       id_ctrl;

    logic [31:0] ex_alu_a, ex_alu_b, ex_alu_in2, ex_alu_result;
    logic        ex_zero;
    logic [1:0]  fwd_a, fwd_b;

    logic        stall, flush;
    logic        branch_taken;
    logic [31:0] branch_target;

    logic [31:0] wb_result;
    logic [31:0] mem_addr, mem_wdata;
    logic        mem_write_en;

    assign debug_pc = pc;

    // =========================================================================
    // IF Stage
    // =========================================================================
    assign pc_plus4 = pc + 32'd4;
    assign if_instr = (pc[31:2] < 256) ? instr_mem[pc[31:2]] : 32'h00000013; // NOP if out of range

    // Next PC logic
    always_comb begin
        if (branch_taken)
            pc_next = branch_target;
        else
            pc_next = pc_plus4;
    end

    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n)
            pc <= 32'd0;
        else if (!stall)
            pc <= pc_next;
    end

    // IF/ID register
    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n || flush) begin
            ifid_instr <= 32'h00000013; // NOP
            ifid_pc4   <= 32'd0;
        end else if (!stall) begin
            ifid_instr <= if_instr;
            ifid_pc4   <= pc_plus4;
        end
    end

    // =========================================================================
    // ID Stage — Decode
    // =========================================================================
    assign id_opcode = ifid_instr[6:0];
    assign id_rd     = ifid_instr[11:7];
    assign id_rs1    = ifid_instr[19:15];
    assign id_rs2    = ifid_instr[24:20];

    // Immediate generation (handles I, S, B types)
    always_comb begin
        case (id_opcode)
            OP_LOAD:   id_imm = {{20{ifid_instr[31]}}, ifid_instr[31:20]};
            OP_STORE:  id_imm = {{20{ifid_instr[31]}}, ifid_instr[31:25], ifid_instr[11:7]};
            OP_BRANCH: id_imm = {{19{ifid_instr[31]}}, ifid_instr[31], ifid_instr[7],
                                 ifid_instr[30:25], ifid_instr[11:8], 1'b0};
            default:   id_imm = {{20{ifid_instr[31]}}, ifid_instr[31:20]}; // I-type ALU
        endcase
    end

    // Control signals
    always_comb begin
        id_ctrl = '{default: 0};
        case (id_opcode)
            OP_RTYPE: begin
                id_ctrl.reg_write = 1'b1;
                id_ctrl.alu_op    = 2'b00; // decoded further by funct
            end
            OP_ITYPE: begin
                id_ctrl.reg_write = 1'b1;
                id_ctrl.alu_src   = 1'b1;
                id_ctrl.alu_op    = 2'b00;
            end
            OP_LOAD: begin
                id_ctrl.reg_write  = 1'b1;
                id_ctrl.mem_to_reg = 1'b1;
                id_ctrl.mem_read   = 1'b1;
                id_ctrl.alu_src    = 1'b1;
                id_ctrl.is_load    = 1'b1;
            end
            OP_STORE: begin
                id_ctrl.mem_write = 1'b1;
                id_ctrl.alu_src   = 1'b1;
            end
            OP_BRANCH: begin
                id_ctrl.branch = 1'b1;
                id_ctrl.alu_op = 2'b01; // SUB for comparison
            end
            default: ;
        endcase
    end

    // Register file
    regfile u_regfile (
        .clk       (clk),
        .rs1_addr  (id_rs1),
        .rs2_addr  (id_rs2),
        .rd_addr   (memwb_rd),
        .rd_data   (wb_result),
        .rd_write  (memwb_reg_write),
        .rs1_data  (id_rs1_data),
        .rs2_data  (id_rs2_data)
    );

    // Hazard detection
    hazard_detection u_hazard (
        .id_ex_rd        (idex_rd),
        .id_ex_mem_read  (idex_ctrl.mem_read),
        .if_id_rs1       (id_rs1),
        .if_id_rs2       (id_rs2),
        .stall           (stall)
    );

    assign flush = branch_taken;

    // ID/EX register
    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n || stall || flush) begin
            idex_ctrl  <= '{default: 0};
            idex_pc4   <= 32'd0;
            idex_rs1   <= 32'd0;
            idex_rs2   <= 32'd0;
            idex_imm   <= 32'd0;
            idex_rs1_addr <= 5'd0;
            idex_rs2_addr <= 5'd0;
            idex_rd    <= 5'd0;
        end else begin
            idex_ctrl     <= id_ctrl;
            idex_pc4      <= ifid_pc4;
            idex_rs1      <= id_rs1_data;
            idex_rs2      <= id_rs2_data;
            idex_imm      <= id_imm;
            idex_rs1_addr <= id_rs1;
            idex_rs2_addr <= id_rs2;
            idex_rd       <= id_rd;
        end
    end

    // =========================================================================
    // EX Stage
    // =========================================================================
    forwarding_unit u_fwd (
        .ex_mem_rd         (exmem_rd),
        .ex_mem_reg_write  (exmem_ctrl.reg_write),
        .mem_wb_rd         (memwb_rd),
        .mem_wb_reg_write  (memwb_reg_write),
        .id_ex_rs1         (idex_rs1_addr),
        .id_ex_rs2         (idex_rs2_addr),
        .forward_a         (fwd_a),
        .forward_b         (fwd_b)
    );

    // Forwarding muxes
    always_comb begin
        case (fwd_a)
            2'b10:   ex_alu_a = exmem_alu;
            2'b01:   ex_alu_a = wb_result;
            default: ex_alu_a = idex_rs1;
        endcase
        case (fwd_b)
            2'b10:   ex_alu_in2 = exmem_alu;
            2'b01:   ex_alu_in2 = wb_result;
            default: ex_alu_in2 = idex_rs2;
        endcase
    end

    assign ex_alu_b = idex_ctrl.alu_src ? idex_imm : ex_alu_in2;

    alu u_alu (
        .a       (ex_alu_a),
        .b       (ex_alu_b),
        .alu_op  (idex_ctrl.alu_op),
        .result  (ex_alu_result),
        .zero    (ex_zero)
    );

    // Branch resolution (simple: taken if zero == 1 for BEQ)
    assign branch_taken  = idex_ctrl.branch && ex_zero;
    assign branch_target = idex_pc4 - 32'd4 + idex_imm; // PC-relative (note: PC was +4 in IF)

    // EX/MEM register
    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            exmem_ctrl <= '{default: 0};
            exmem_alu  <= 32'd0;
            exmem_rs2  <= 32'd0;
            exmem_rd   <= 5'd0;
        end else begin
            exmem_ctrl <= idex_ctrl;
            exmem_alu  <= ex_alu_result;
            exmem_rs2  <= ex_alu_in2;
            exmem_rd   <= idex_rd;
        end
    end

    // =========================================================================
    // MEM Stage
    // =========================================================================
    assign mem_addr      = exmem_alu;
    assign mem_wdata     = exmem_rs2;
    assign mem_write_en  = exmem_ctrl.mem_write;

    // MEM/WB register
    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            memwb_reg_write  <= 1'b0;
            memwb_mem_to_reg <= 1'b0;
            memwb_alu        <= 32'd0;
            memwb_mem        <= 32'd0;
            memwb_rd         <= 5'd0;
        end else begin
            memwb_reg_write  <= exmem_ctrl.reg_write;
            memwb_mem_to_reg <= exmem_ctrl.mem_to_reg;
            memwb_alu        <= exmem_alu;
            memwb_mem        <= mem_rdata;
            memwb_rd         <= exmem_rd;
        end
    end

    // =========================================================================
    // WB Stage
    // =========================================================================
    assign wb_result = memwb_mem_to_reg ? memwb_mem : memwb_alu;

endmodule

// =============================================================================
// Testbench
// =============================================================================
module tb_pipeline;
    logic clk, rst_n;
    logic [31:0] debug_pc;

    pipeline_cpu uut (.*);

    // 10 ns clock
    always #5 clk = ~clk;

    // Hand-assembled test program (hex):
    //   ADD x1, x2, x3     → 003100B3  (no hazard)
    //   SUB x4, x1, x5     → 40508233  (RAW on x1 — forwarded from EX/MEM)
    //   AND x6, x1, x7     → 0070F333  (RAW on x1 — forwarded from MEM/WB)
    //   OR  x8, x9, x10    → 00A4E433  (no hazard)
    //   LW  x11, 0(x12)    → 00060583  (load)
    //   ADD x13, x11, x14  → 00E586B3  (load-use — requires stall)
    initial begin
        $readmemh("program.hex", uut.instr_mem);
    end

    // Monitor pipeline activity
    always @(posedge clk) begin
        $display("PC=%03d  IF=%h  ID=%h  stall=%b flush=%b",
                 debug_pc, uut.if_instr, uut.ifid_instr, uut.stall, uut.flush);
    end

    initial begin
        clk = 0; rst_n = 0;
        #12 rst_n = 1;
        #200;
        $display("=== Simulation complete ===");
        $finish;
    end
endmodule
