module tiny_alu(
  input [7:0] a,
  input [7:0] b,
  input [1:0] op,
  output reg [7:0] y,
  output zero
);
  always @(*) begin
    case (op)
      2'b00: y = a + b;
      2'b01: y = a - b;
      2'b10: y = a & b;
      default: y = a | b;
    endcase
  end

  assign zero = (y == 8'b0);
endmodule
