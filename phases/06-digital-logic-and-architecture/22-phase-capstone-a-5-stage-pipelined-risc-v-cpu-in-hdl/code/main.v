module pipeline_skeleton(
  input clk,
  input rst,
  output reg [31:0] pc
);
  always @(posedge clk or posedge rst) begin
    if (rst) pc <= 0;
    else pc <= pc + 32'd4;
  end
endmodule
