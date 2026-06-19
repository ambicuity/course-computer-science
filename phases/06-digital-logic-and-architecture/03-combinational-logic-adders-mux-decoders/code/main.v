module half_adder(input a, input b, output sum, output carry);
  assign sum = a ^ b;
  assign carry = a & b;
endmodule

module mux2(input sel, input d0, input d1, output y);
  assign y = sel ? d1 : d0;
endmodule

module combo_demo;
  reg a, b, sel;
  wire sum, carry, y;
  half_adder ha(.a(a), .b(b), .sum(sum), .carry(carry));
  mux2 m(.sel(sel), .d0(a), .d1(b), .y(y));

  initial begin
    a=0; b=1; sel=0; #1;
    $display("sum=%b carry=%b mux=%b", sum, carry, y);
    sel=1; #1;
    $display("sum=%b carry=%b mux=%b", sum, carry, y);
    $finish;
  end
endmodule
