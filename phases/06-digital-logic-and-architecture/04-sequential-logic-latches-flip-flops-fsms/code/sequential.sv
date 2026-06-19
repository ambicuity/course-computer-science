// Sequential Logic — Latches, Flip-Flops, FSMs
// All modules for Phase 06, Lesson 04

// ---------------------------------------------------------------------------
// SR Latch — two cross-coupled NOR gates
// ---------------------------------------------------------------------------
module sr_latch (
    input  logic s, r,
    output logic q, qbar
);
    assign q    = ~(r | qbar);
    assign qbar = ~(s | q);
endmodule

// ---------------------------------------------------------------------------
// D Latch — level-sensitive, Q follows D when en=1
// ---------------------------------------------------------------------------
module d_latch (
    input  logic d, en,
    output logic q
);
    always_latch begin
        if (en) q <= d;
    end
endmodule

// ---------------------------------------------------------------------------
// D Flip-Flop — positive-edge triggered, the fundamental building block
// ---------------------------------------------------------------------------
module dff (
    input  logic       d, clk,
    output logic       q
);
    always_ff @(posedge clk) begin
        q <= d;
    end
endmodule

// ---------------------------------------------------------------------------
// JK Flip-Flop — toggles when J=K=1, eliminates SR forbidden state
// ---------------------------------------------------------------------------
module jk_ff (
    input  logic       j, k, clk,
    output logic       q
);
    always_ff @(posedge clk) begin
        case ({j, k})
            2'b00: q <= q;
            2'b01: q <= 1'b0;
            2'b10: q <= 1'b1;
            2'b11: q <= ~q;
        endcase
    end
endmodule

// ---------------------------------------------------------------------------
// 4-Bit Up Counter — synchronous reset, built from DFFs
// ---------------------------------------------------------------------------
module counter_4bit (
    input  logic       clk, rst,
    output logic [3:0] count
);
    always_ff @(posedge clk) begin
        if (rst) count <= 4'd0;
        else     count <= count + 4'd1;
    end
endmodule

// ---------------------------------------------------------------------------
// Traffic Light Controller — Moore FSM, 4 states
// NS_GREEN → NS_YELLOW → EW_GREEN → EW_YELLOW → NS_GREEN ...
// ---------------------------------------------------------------------------
module traffic_light_fsm (
    input  logic clk, rst,
    output logic ns_green, ns_yellow, ew_green, ew_yellow
);
    typedef enum logic [1:0] {
        NS_GREEN  = 2'b00,
        NS_YELLOW = 2'b01,
        EW_GREEN  = 2'b10,
        EW_YELLOW = 2'b11
    } state_t;

    state_t state, next_state;

    always_ff @(posedge clk) begin
        if (rst) state <= NS_GREEN;
        else     state <= next_state;
    end

    always_comb begin
        case (state)
            NS_GREEN:  next_state = NS_YELLOW;
            NS_YELLOW: next_state = EW_GREEN;
            EW_GREEN:  next_state = EW_YELLOW;
            EW_YELLOW: next_state = NS_GREEN;
            default:   next_state = NS_GREEN;
        endcase
    end

    always_comb begin
        ns_green  = (state == NS_GREEN);
        ns_yellow = (state == NS_YELLOW);
        ew_green  = (state == EW_GREEN);
        ew_yellow = (state == EW_YELLOW);
    end
endmodule

// ---------------------------------------------------------------------------
// Sequence Detector "1011" — Mealy FSM with overlap
// Detects the serial bit pattern 1-0-1-1 on din, one bit per clock.
// ---------------------------------------------------------------------------
module sequence_detector_1011 (
    input  logic clk, rst, din,
    output logic detected
);
    typedef enum logic [1:0] {
        S0 = 2'b00, S1 = 2'b01, S2 = 2'b10, S3 = 2'b11
    } state_t;

    state_t state, next_state;

    always_ff @(posedge clk) begin
        if (rst) state <= S0;
        else     state <= next_state;
    end

    always_comb begin
        detected   = 1'b0;
        next_state = S0;
        case (state)
            S0: begin
                if (din) next_state = S1;
                else     next_state = S0;
            end
            S1: begin
                if (din) next_state = S1;
                else     next_state = S2;
            end
            S2: begin
                if (din) next_state = S3;
                else     next_state = S0;
            end
            S3: begin
                if (din) begin
                    detected   = 1'b1;
                    next_state = S1;
                end else begin
                    next_state = S2;
                end
            end
        endcase
    end
endmodule

// ---------------------------------------------------------------------------
// Testbench — exercises all modules
// ---------------------------------------------------------------------------
module tb_sequential;

    // SR latch
    logic s, r, q_sr, qbar_sr;
    sr_latch u_sr (.s(s), .r(r), .q(q_sr), .qbar(qbar_sr));

    // D latch
    logic d_dl, en_dl, q_dl;
    d_latch u_dl (.d(d_dl), .en(en_dl), .q(q_dl));

    // D flip-flop
    logic d_dff, clk, q_dff;
    dff u_dff (.d(d_dff), .clk(clk), .q(q_dff));

    // JK flip-flop
    logic j_jk, k_jk, q_jk;
    jk_ff u_jk (.j(j_jk), .k(k_jk), .clk(clk), .q(q_jk));

    // 4-bit counter
    logic rst, rst_counter;
    logic [3:0] count;
    counter_4bit u_cnt (.clk(clk), .rst(rst_counter), .count(count));

    // Traffic light
    logic ns_g, ns_y, ew_g, ew_y;
    traffic_light_fsm u_tl (.clk(clk), .rst(rst), .ns_green(ns_g), .ns_yellow(ns_y),
                             .ew_green(ew_g), .ew_yellow(ew_y));

    // Sequence detector
    logic din, detected;
    sequence_detector_1011 u_sd (.clk(clk), .rst(rst), .din(din), .detected(detected));

    // Clock generation — 10 ns period
    initial clk = 0;
    always #5 clk = ~clk;

    // Main stimulus
    initial begin
        $dumpfile("sequential.vcd");
        $dumpvars(0, tb_sequential);

        // -------------------------------------------------------------------
        // 1. SR Latch tests
        // -------------------------------------------------------------------
        $display("=== SR Latch ===");
        s = 0; r = 0; #1; $display("Hold:  q=%b qbar=%b", q_sr, qbar_sr);
        s = 1; r = 0; #1; $display("Set:   q=%b qbar=%b", q_sr, qbar_sr);
        s = 0; r = 0; #1; $display("Hold:  q=%b qbar=%b", q_sr, qbar_sr);
        s = 0; r = 1; #1; $display("Reset: q=%b qbar=%b", q_sr, qbar_sr);
        s = 0; r = 0; #1; $display("Hold:  q=%b qbar=%b", q_sr, qbar_sr);

        // -------------------------------------------------------------------
        // 2. D Latch tests
        // -------------------------------------------------------------------
        $display("\n=== D Latch ===");
        en_dl = 0; d_dl = 1; #1; $display("EN=0 D=1: q=%b", q_dl);
        en_dl = 1; d_dl = 1; #1; $display("EN=1 D=1: q=%b", q_dl);
        d_dl = 0;          #1; $display("EN=1 D=0: q=%b", q_dl);
        en_dl = 0; d_dl = 1; #1; $display("EN=0 D=1: q=%b (should hold 0)", q_dl);

        // -------------------------------------------------------------------
        // 3. D Flip-Flop tests
        // -------------------------------------------------------------------
        $display("\n=== D Flip-Flop ===");
        rst = 1; rst_counter = 1; @(posedge clk); #1;
        rst = 0; rst_counter = 0;

        d_dff = 1; @(posedge clk); #1; $display("D=1 -> q=%b", q_dff);
        d_dff = 0; @(posedge clk); #1; $display("D=0 -> q=%b", q_dff);
        d_dff = 1; @(posedge clk); #1; $display("D=1 -> q=%b", q_dff);

        // -------------------------------------------------------------------
        // 4. JK Flip-Flop tests
        // -------------------------------------------------------------------
        $display("\n=== JK Flip-Flop ===");
        j_jk = 0; k_jk = 0; @(posedge clk); #1; $display("J=0 K=0: q=%b (hold)", q_jk);
        j_jk = 1; k_jk = 0; @(posedge clk); #1; $display("J=1 K=0: q=%b (set)", q_jk);
        j_jk = 0; k_jk = 0; @(posedge clk); #1; $display("J=0 K=0: q=%b (hold)", q_jk);
        j_jk = 0; k_jk = 1; @(posedge clk); #1; $display("J=0 K=1: q=%b (reset)", q_jk);
        j_jk = 1; k_jk = 1; @(posedge clk); #1; $display("J=1 K=1: q=%b (toggle)", q_jk);
        j_jk = 1; k_jk = 1; @(posedge clk); #1; $display("J=1 K=1: q=%b (toggle)", q_jk);

        // -------------------------------------------------------------------
        // 5. Counter tests
        // -------------------------------------------------------------------
        $display("\n=== 4-Bit Counter ===");
        rst_counter = 1; @(posedge clk); #1;
        rst_counter = 0;
        repeat (6) begin
            @(posedge clk); #1;
            $display("count = %0d", count);
        end

        // -------------------------------------------------------------------
        // 6. Traffic Light FSM
        // -------------------------------------------------------------------
        $display("\n=== Traffic Light FSM ===");
        rst = 1; @(posedge clk); #1;
        rst = 0;
        repeat (8) begin
            @(posedge clk); #1;
            $display("NS_G=%b NS_Y=%b EW_G=%b EW_Y=%b", ns_g, ns_y, ew_g, ew_y);
        end

        // -------------------------------------------------------------------
        // 7. Sequence Detector — feed "1011011"
        // -------------------------------------------------------------------
        $display("\n=== Sequence Detector 1011 ===");
        rst = 1; @(posedge clk); #1;
        rst = 0;

        // Feed: 1 0 1 1 0 1 1
        din = 1; @(posedge clk); #1; $display("din=1 det=%b", detected);
        din = 0; @(posedge clk); #1; $display("din=0 det=%b", detected);
        din = 1; @(posedge clk); #1; $display("din=1 det=%b", detected);
        din = 1; @(posedge clk); #1; $display("din=1 det=%b (expect 1)", detected);
        din = 0; @(posedge clk); #1; $display("din=0 det=%b", detected);
        din = 1; @(posedge clk); #1; $display("din=1 det=%b", detected);
        din = 1; @(posedge clk); #1; $display("din=1 det=%b (expect 1)", detected);

        $display("\nAll tests complete.");
        $finish;
    end

endmodule
