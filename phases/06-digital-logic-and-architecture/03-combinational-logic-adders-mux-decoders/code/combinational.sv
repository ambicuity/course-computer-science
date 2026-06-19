// Combinational Logic — Adders, Mux, Decoders
// SystemVerilog module library for Phase 06, Lesson 03

// ============================================================
// Half Adder
// ============================================================
module half_adder (
    input  logic a, b,
    output logic sum, cout
);
    assign sum  = a ^ b;
    assign cout = a & b;
endmodule

// ============================================================
// Full Adder (built from two half adders)
// ============================================================
module full_adder (
    input  logic a, b, cin,
    output logic sum, cout
);
    logic s1, c1, c2;
    half_adder ha0 (.a(a),    .b(b),   .sum(s1),  .cout(c1));
    half_adder ha1 (.a(s1),   .b(cin), .sum(sum), .cout(c2));
    assign cout = c1 | c2;
endmodule

// ============================================================
// 4-bit Ripple-Carry Adder
// ============================================================
module ripple_carry_adder_4bit (
    input  logic [3:0] a, b,
    input  logic       cin,
    output logic [3:0] sum,
    output logic       cout
);
    logic [3:1] c;
    full_adder fa0 (.a(a[0]), .b(b[0]), .cin(cin), .sum(sum[0]), .cout(c[1]));
    full_adder fa1 (.a(a[1]), .b(b[1]), .cin(c[1]), .sum(sum[1]), .cout(c[2]));
    full_adder fa2 (.a(a[2]), .b(b[2]), .cin(c[2]), .sum(sum[2]), .cout(c[3]));
    full_adder fa3 (.a(a[3]), .b(b[3]), .cin(c[3]), .sum(sum[3]), .cout(cout));
endmodule

// ============================================================
// 4-bit Carry-Lookahead Adder
// ============================================================
module cla_adder_4bit (
    input  logic [3:0] a, b,
    input  logic       cin,
    output logic [3:0] sum,
    output logic       cout
);
    logic [3:0] G, P, c;

    assign G = a & b;
    assign P = a ^ b;

    assign c[0] = cin;
    assign c[1] = G[0] | (P[0] & c[0]);
    assign c[2] = G[1] | (P[1] & G[0]) | (P[1] & P[0] & c[0]);
    assign c[3] = G[2] | (P[2] & G[1]) | (P[2] & P[1] & G[0]) | (P[2] & P[1] & P[0] & c[0]);
    assign cout = G[3] | (P[3] & c[3]);

    assign sum = P ^ c;
endmodule

// ============================================================
// 2:1 Multiplexer
// ============================================================
module mux_2to1 (
    input  logic a, b, sel,
    output logic y
);
    assign y = sel ? b : a;
endmodule

// ============================================================
// 4:1 Multiplexer (built from 2:1 muxes)
// ============================================================
module mux_4to1 (
    input  logic       a, b, c, d,
    input  logic [1:0] sel,
    output logic       y
);
    logic lo, hi;
    mux_2to1 mlo  (.a(a), .b(b), .sel(sel[0]), .y(lo));
    mux_2to1 mhi  (.a(c), .b(d), .sel(sel[0]), .y(hi));
    mux_2to1 mout (.a(lo), .b(hi), .sel(sel[1]), .y(y));
endmodule

// ============================================================
// 2-to-4 Decoder with Enable
// ============================================================
module decoder_2to4 (
    input  logic [1:0] a,
    input  logic       en,
    output logic [3:0] y
);
    assign y = en ? (4'b0001 << a) : 4'b0000;
endmodule

// ============================================================
// 4-to-2 Priority Encoder
// ============================================================
module encoder_4to2 (
    input  logic [3:0] y,
    output logic [1:0] a,
    output logic       valid
);
    always_comb begin
        valid = 1;
        if (y[3])       a = 2'd3;
        else if (y[2])  a = 2'd2;
        else if (y[1])  a = 2'd1;
        else if (y[0])  a = 2'd0;
        else begin
            a = 2'd0;
            valid = 0;
        end
    end
endmodule

// ============================================================
// Testbench
// ============================================================
module tb_combinational;
    // --- Adder signals ---
    logic [3:0] a4, b4;
    logic       cin;
    logic [3:0] sum_rca, sum_cla;
    logic       cout_rca, cout_cla;

    ripple_carry_adder_4bit rca (.a(a4), .b(b4), .cin(cin), .sum(sum_rca), .cout(cout_rca));
    cla_adder_4bit          cla (.a(a4), .b(b4), .cin(cin), .sum(sum_cla), .cout(cout_cla));

    // --- Mux signals ---
    logic m_a, m_b, m_c, m_d, m_sel0, m_y2, m_y4;
    logic [1:0] m_sel1;
    mux_2to1 dut_mux2 (.a(m_a), .b(m_b), .sel(m_sel0), .y(m_y2));
    mux_4to1 dut_mux4 (.a(m_a), .b(m_b), .c(m_c), .d(m_d), .sel(m_sel1), .y(m_y4));

    // --- Decoder signals ---
    logic [1:0] dec_a;
    logic       dec_en;
    logic [3:0] dec_y;
    decoder_2to4 dut_dec (.a(dec_a), .en(dec_en), .y(dec_y));

    // --- Encoder signals ---
    logic [3:0] enc_y;
    logic [1:0] enc_a;
    logic       enc_valid;
    encoder_4to2 dut_enc (.y(enc_y), .a(enc_a), .valid(enc_valid));

    initial begin
        $display("=== Combinational Logic Testbench ===");

        // Test adders: a=5, b=3, cin=1 -> sum=9, cout=0
        a4 = 4'd5; b4 = 4'd3; cin = 1;
        #1;
        $display("Adder 5+3+1: RCA sum=%0d cout=%b | CLA sum=%0d cout=%b",
                 sum_rca, cout_rca, sum_cla, cout_cla);
        assert(sum_rca == 9 && !cout_rca) else $error("RCA failed 5+3+1");
        assert(sum_cla == 9 && !cout_cla) else $error("CLA failed 5+3+1");

        // Test adders: a=15, b=1, cin=0 -> sum=0, cout=1
        a4 = 4'd15; b4 = 4'd1; cin = 0;
        #1;
        $display("Adder 15+1+0: RCA sum=%0d cout=%b | CLA sum=%0d cout=%b",
                 sum_rca, cout_rca, sum_cla, cout_cla);
        assert(sum_rca == 0 && cout_rca) else $error("RCA failed 15+1");
        assert(sum_cla == 0 && cout_cla) else $error("CLA failed 15+1");

        // Exhaustive adder comparison
        for (int i = 0; i < 16; i++)
            for (int j = 0; j < 16; j++)
                for (int k = 0; k < 2; k++) begin
                    a4 = i; b4 = j; cin = k;
                    #1;
                    assert({cout_rca, sum_rca} == {cout_cla, sum_cla})
                        else $error("Mismatch: a=%0d b=%0d cin=%0d", i, j, k);
                end
        $display("Adder: all 512 combinations match.");

        // Test 2:1 mux
        m_a = 0; m_b = 1;
        m_sel0 = 0; #1; assert(m_y2 == 0);
        m_sel0 = 1; #1; assert(m_y2 == 1);
        $display("MUX 2:1 OK");

        // Test 4:1 mux
        m_a = 1; m_b = 0; m_c = 1; m_d = 0;
        m_sel1 = 2'b00; #1; assert(m_y4 == 1);
        m_sel1 = 2'b01; #1; assert(m_y4 == 0);
        m_sel1 = 2'b10; #1; assert(m_y4 == 1);
        m_sel1 = 2'b11; #1; assert(m_y4 == 0);
        $display("MUX 4:1 OK");

        // Test decoder
        dec_en = 0; dec_a = 2'b10; #1; assert(dec_y == 4'b0000);
        dec_en = 1;
        dec_a = 2'b00; #1; assert(dec_y == 4'b0001);
        dec_a = 2'b01; #1; assert(dec_y == 4'b0010);
        dec_a = 2'b10; #1; assert(dec_y == 4'b0100);
        dec_a = 2'b11; #1; assert(dec_y == 4'b1000);
        $display("Decoder 2:4 OK");

        // Test priority encoder
        enc_y = 4'b0000; #1; assert(enc_valid == 0);
        enc_y = 4'b0001; #1; assert(enc_a == 0 && enc_valid == 1);
        enc_y = 4'b0010; #1; assert(enc_a == 1 && enc_valid == 1);
        enc_y = 4'b0101; #1; assert(enc_a == 2 && enc_valid == 1);
        enc_y = 4'b1111; #1; assert(enc_a == 3 && enc_valid == 1);
        $display("Priority Encoder OK");

        $display("=== All tests passed ===");
        $finish;
    end
endmodule
