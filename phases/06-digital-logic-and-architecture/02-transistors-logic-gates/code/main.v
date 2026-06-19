module logic_gates_demo;
  reg a, b;
  wire and_y, or_y, xor_y, not_a;

  assign and_y = a & b;
  assign or_y = a | b;
  assign xor_y = a ^ b;
  assign not_a = ~a;

  initial begin
    $display("a b | and or xor not_a");
    a=0; b=0; #1 $display("%b %b |  %b   %b   %b    %b", a,b,and_y,or_y,xor_y,not_a);
    a=0; b=1; #1 $display("%b %b |  %b   %b   %b    %b", a,b,and_y,or_y,xor_y,not_a);
    a=1; b=0; #1 $display("%b %b |  %b   %b   %b    %b", a,b,and_y,or_y,xor_y,not_a);
    a=1; b=1; #1 $display("%b %b |  %b   %b   %b    %b", a,b,and_y,or_y,xor_y,not_a);
    $finish;
  end
endmodule
