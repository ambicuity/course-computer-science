module two_bit_predictor(
  input clk,
  input update,
  input taken,
  output predict_taken
);
  reg [1:0] state = 2'b10;
  assign predict_taken = state[1];

  always @(posedge clk) begin
    if (update) begin
      case ({state, taken})
        3'b000: state <= 2'b00;
        3'b001: state <= 2'b01;
        3'b010: state <= 2'b00;
        3'b011: state <= 2'b10;
        3'b100: state <= 2'b01;
        3'b101: state <= 2'b11;
        3'b110: state <= 2'b10;
        3'b111: state <= 2'b11;
      endcase
    end
  end
endmodule
