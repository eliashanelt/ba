`timescale 1ns / 1ps
//////////////////////////////////////////////////////////////////////////////////
// Company: 
// Engineer: 
// 
// Create Date: 06/30/2025 06:31:06 PM
// Design Name: 
// Module Name: signal_clipper
// Project Name: 
// Target Devices: 
// Tool Versions: 
// Description: 
// 
// Dependencies: 
// 
// Revision:
// Revision 0.01 - File Created
// Additional Comments:
// 
//////////////////////////////////////////////////////////////////////////////////
module signal_clipper #(
    parameter DATA_WIDTH = 14,  // RedPitaya ADC/DAC width
    parameter LIMIT_VALUE = 8191  // Adjustable limit
)(
    input clk,
    input signed [DATA_WIDTH-1:0] data_in,
    output reg signed [DATA_WIDTH-1:0] data_out
);

always @(posedge clk) begin
    if (data_in > LIMIT_VALUE)
        data_out <= LIMIT_VALUE;
    else if (data_in < -LIMIT_VALUE)
        data_out <= -LIMIT_VALUE;
    else
        data_out <= data_in;
end
endmodule