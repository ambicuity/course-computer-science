module hardwired_control(
  input [6:0] opcode,
  output reg reg_write,
  output reg mem_read,
  output reg mem_write
);
  always @(*) begin
    reg_write = 0;
    mem_read = 0;
    mem_write = 0;
    case (opcode)
      7'b0000011: begin mem_read = 1; reg_write = 1; end
      7'b0100011: begin mem_write = 1; end
      7'b0110011: begin reg_write = 1; end
      default: begin end
    endcase
  end
endmodule
