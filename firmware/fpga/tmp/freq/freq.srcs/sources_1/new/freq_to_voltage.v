`timescale 1ns / 1ps
//////////////////////////////////////////////////////////////////////////////////
// Company: 
// Engineer: 
// 
// Create Date: 06/30/2025 06:32:49 PM
// Design Name: 
// Module Name: freq_to_voltage
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

module freq_to_voltage #(
    parameter FREQ_WIDTH = 32,
    parameter DAC_WIDTH = 14,
    parameter MAX_FREQ = 62500000,  // Max detectable frequency
    parameter MIN_FREQ = 1000       // Min detectable frequency
)(
    input clk,
    input [FREQ_WIDTH-1:0] frequency_in,
    output reg signed [DAC_WIDTH-1:0] voltage_out
);

always @(posedge clk) begin
    // Scale frequency to voltage: 1V = 8191 DAC counts
    if (frequency_in > MAX_FREQ)
        voltage_out <= 8191;  // +1V
    else if (frequency_in < MIN_FREQ)
        voltage_out <= -8191; // -1V
    else
        voltage_out <= ((frequency_in - MIN_FREQ) * 16382) / (MAX_FREQ - MIN_FREQ) - 8191;
end
endmodule