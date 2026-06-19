// =============================================================================
// Branch Predictor Modules — Bimodal, Gshare, Tournament
// =============================================================================

// =============================================================================
// 2-Bit Saturating Counter (used by all predictor tables)
// =============================================================================
module saturating_counter_2bit (
    input  logic       clk,
    input  logic       rst_n,
    input  logic       update_en,
    input  logic       outcome,    // 1 = taken, 0 = not-taken
    input  logic [1:0] state_in,
    output logic [1:0] state_out,
    output logic       predict     // 1 = predict taken (MSB == 1)
);
    logic [1:0] state;

    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n)
            state <= 2'b10; // weakly taken default
        else if (update_en) begin
            if (outcome && state != 2'b11)
                state <= state + 2'd1;
            else if (!outcome && state != 2'b00)
                state <= state - 2'd1;
        end
    end

    assign state_out = state;
    assign predict   = state[1]; // top bit: 11,10 → taken; 01,00 → not-taken
endmodule

// =============================================================================
// Bimodal (2-Bit) Predictor
// Table of N saturating counters indexed by PC bits.
// =============================================================================
module bimodal_predictor #(
    parameter TABLE_BITS = 10,               // 2^TABLE_BITS entries
    parameter TABLE_SIZE = 1 << TABLE_BITS
)(
    input  logic        clk,
    input  logic        rst_n,
    input  logic        update_en,
    input  logic [31:0] lookup_pc,
    input  logic [31:0] update_pc,
    input  logic        actual_outcome,
    output logic        prediction
);
    logic [1:0] table [0:TABLE_SIZE-1];
    logic [TABLE_BITS-1:0] lookup_idx, update_idx;

    assign lookup_idx = lookup_pc[TABLE_BITS+1:2]; // word-aligned
    assign update_idx = update_pc[TABLE_BITS+1:2];

    // Read (prediction is MSB of counter)
    assign prediction = table[lookup_idx][1];

    // Update
    integer i;
    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            for (i = 0; i < TABLE_SIZE; i = i + 1)
                table[i] <= 2'b10; // weakly taken
        end else if (update_en) begin
            if (actual_outcome && table[update_idx] != 2'b11)
                table[update_idx] <= table[update_idx] + 2'd1;
            else if (!actual_outcome && table[update_idx] != 2'b00)
                table[update_idx] <= table[update_idx] - 2'd1;
        end
    end
endmodule

// =============================================================================
// Gshare Predictor
// Index = PC[bits] XOR global_history
// =============================================================================
module gshare_predictor #(
    parameter TABLE_BITS = 10,
    parameter HIST_BITS  = 10,
    parameter TABLE_SIZE = 1 << TABLE_BITS
)(
    input  logic        clk,
    input  logic        rst_n,
    input  logic        update_en,
    input  logic [31:0] lookup_pc,
    input  logic [31:0] update_pc,
    input  logic        actual_outcome,
    output logic        prediction
);
    logic [1:0]         table [0:TABLE_SIZE-1];
    logic [HIST_BITS-1:0] global_history;

    logic [TABLE_BITS-1:0] lookup_idx, update_idx;

    assign lookup_idx = lookup_pc[TABLE_BITS+1:2] ^ global_history;
    assign update_idx = update_pc[TABLE_BITS+1:2] ^ global_history;

    assign prediction = table[lookup_idx][1];

    integer i;
    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            global_history <= {HIST_BITS{1'b0}};
            for (i = 0; i < TABLE_SIZE; i = i + 1)
                table[i] <= 2'b10;
        end else if (update_en) begin
            // Update counter
            if (actual_outcome && table[update_idx] != 2'b11)
                table[update_idx] <= table[update_idx] + 2'd1;
            else if (!actual_outcome && table[update_idx] != 2'b00)
                table[update_idx] <= table[update_idx] - 2'd1;
            // Shift global history
            global_history <= {global_history[HIST_BITS-2:0], actual_outcome};
        end
    end
endmodule

// =============================================================================
// Tournament Predictor
// Local predictor (per-branch history) + Global predictor (gshare)
// + Selector that chooses between them.
// =============================================================================
module tournament_predictor #(
    parameter TABLE_BITS = 10,
    parameter HIST_BITS  = 10,
    parameter TABLE_SIZE = 1 << TABLE_BITS
)(
    input  logic        clk,
    input  logic        rst_n,
    input  logic        update_en,
    input  logic [31:0] lookup_pc,
    input  logic [31:0] update_pc,
    input  logic        actual_outcome,
    output logic        prediction
);

    // --- Local predictor (per-branch 2-bit counter table) ------------------
    logic [1:0] local_table [0:TABLE_SIZE-1];
    logic [TABLE_BITS-1:0] local_idx, local_up_idx;
    logic local_pred;

    assign local_idx    = lookup_pc[TABLE_BITS+1:2];
    assign local_up_idx = update_pc[TABLE_BITS+1:2];
    assign local_pred   = local_table[local_idx][1];

    // --- Global predictor (gshare) -----------------------------------------
    logic [HIST_BITS-1:0] ghr; // global history register
    logic [TABLE_BITS-1:0] g_idx, g_up_idx;
    logic [1:0] global_table [0:TABLE_SIZE-1];
    logic global_pred;

    assign g_idx      = lookup_pc[TABLE_BITS+1:2] ^ ghr;
    assign g_up_idx   = update_pc[TABLE_BITS+1:2] ^ ghr;
    assign global_pred = global_table[g_idx][1];

    // --- Selector (2-bit counter per branch: 00=use global, 11=use local) --
    logic [1:0] selector [0:TABLE_SIZE-1];
    logic [TABLE_BITS-1:0] sel_idx, sel_up_idx;
    logic use_local;

    assign sel_idx    = lookup_pc[TABLE_BITS+1:2];
    assign sel_up_idx = update_pc[TABLE_BITS+1:2];
    assign use_local  = selector[sel_idx][1]; // MSB=1 → prefer local

    // Final prediction
    assign prediction = use_local ? local_pred : global_pred;

    // --- Update logic -------------------------------------------------------
    integer i;
    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            ghr <= {HIST_BITS{1'b0}};
            for (i = 0; i < TABLE_SIZE; i = i + 1) begin
                local_table[i]  <= 2'b10;
                global_table[i] <= 2'b10;
                selector[i]     <= 2'b01; // default: slightly prefer global
            end
        end else if (update_en) begin
            // Update local predictor
            if (actual_outcome && local_table[local_up_idx] != 2'b11)
                local_table[local_up_idx] <= local_table[local_up_idx] + 2'd1;
            else if (!actual_outcome && local_table[local_up_idx] != 2'b00)
                local_table[local_up_idx] <= local_table[local_up_idx] - 2'd1;

            // Update global predictor
            if (actual_outcome && global_table[g_up_idx] != 2'b11)
                global_table[g_up_idx] <= global_table[g_up_idx] + 2'd1;
            else if (!actual_outcome && global_table[g_up_idx] != 2'b00)
                global_table[g_up_idx] <= global_table[g_up_idx] - 2'd1;

            // Update selector: reward the predictor that was correct
            if (local_pred == actual_outcome && global_pred != actual_outcome) begin
                // Local was right → move selector toward local
                if (selector[sel_up_idx] != 2'b11)
                    selector[sel_up_idx] <= selector[sel_up_idx] + 2'd1;
            end else if (global_pred == actual_outcome && local_pred != actual_outcome) begin
                // Global was right → move selector toward global
                if (selector[sel_up_idx] != 2'b00)
                    selector[sel_up_idx] <= selector[sel_up_idx] - 2'd1;
            end
            // If both right or both wrong, don't change selector

            // Update global history register
            ghr <= {ghr[HIST_BITS-2:0], actual_outcome};
        end
    end
endmodule
