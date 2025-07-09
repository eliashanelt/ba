`timescale 1ns / 1ps
//////////////////////////////////////////////////////////////////////////////////
// Company: 
// Engineer: 
// 
// Create Date: 07/08/2025 10:24:16 PM
// Design Name: 
// Module Name: freq_mapper
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
module freq_mapper #(
    parameter  ADC_WIDTH      = 16,               // Width of signed output (?14)
    parameter  COUNT_WIDTH    = 32,               // Width of period counter input
    parameter  integer LOG2_NCYCLES = 10,          // Measure 2^LOG2_NCYCLES cycles per reading
    parameter  CLK_FREQ_HZ    = 125_000_000       // System clock frequency (Hz)
)(
    input  wire                       clk,
    input  wire                       rst,        // Active-high synchronous reset
    input  wire [COUNT_WIDTH-1:0]     counter_val,// Counts clock ticks for 2^LOG2_NCYCLES periods
    input  wire [31:0]                f_min,      // Minimum of mapping range (Hz)
    input  wire [31:0]                f_max,      // Maximum of mapping range (Hz)
    output reg  signed [ADC_WIDTH-1:0] amp        // Signed 14-bit value centred on zero
);

    // -------------------------------------------------------------------------
    // Constants
    // -------------------------------------------------------------------------
    localparam integer LOGICAL_WIDTH = 14;                       // Resolution bits
    localparam integer FULL_SCALE    = (1 << LOGICAL_WIDTH) - 1; // 16383
    localparam integer OFFSET        = (1 << (LOGICAL_WIDTH-1)); //  8192

    // Numerator = CLK_FREQ_HZ * ncycles = CLK_FREQ_HZ << LOG2_NCYCLES
    //   * Calculated at elaboration; no multiplier in hardware.
    //   * Keep in 64 bits to cover up to 125 MHz * 2^32 ? 5.4e19.
    // -------------------------------------------------------------------------
    localparam [63:0] NUMERATOR = CLK_FREQ_HZ << LOG2_NCYCLES;

    // -------------------------------------------------------------------------
    // Frequency estimator: freq = (CLK_FREQ_HZ * ncycles) / counter_val
    // -------------------------------------------------------------------------
    reg [31:0] freq;           // 0 … 2*CLK_FREQ_HZ fits easily in 32 bits

      // only overwrite freq when counter_val is valid; otherwise hold it
    always @(posedge clk) begin
      if (~rst) begin
        freq <= 32'd0;
      end else if (counter_val != 0) begin
        freq <= NUMERATOR / counter_val;
      end
    end

    // -------------------------------------------------------------------------
    // Linear mapping:   f_min ? -8192,   f_max ? +8191
    // -------------------------------------------------------------------------
    reg  [LOGICAL_WIDTH-1:0] amp_u;              // unsigned 14-bit temp
    reg  signed [ADC_WIDTH-1:0] amp_next;

    always @* begin
        if (freq <= f_min) begin
            amp_next = -OFFSET;                  // Clamp low
        end else if (freq >= f_max) begin
            amp_next = OFFSET - 1;               // Clamp high
        end else begin
            amp_u    = ((freq - f_min) * FULL_SCALE) / (f_max - f_min);
            amp_next = $signed({2'b00, amp_u}) - OFFSET; // Centre around zero
        end
    end

    // -------------------------------------------------------------------------
    // Registered output
    // -------------------------------------------------------------------------
    //localparam [16-1:0] CONST_8191 = 4000;  // auto-truncated if width < 13
    always @(posedge clk) begin
        if (~rst)
            amp <= -OFFSET;    // hold the default value while in reset
        else
            amp <= amp_next;  // update with the mapped value when running
        end

endmodule 