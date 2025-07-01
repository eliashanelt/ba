`timescale 1ns / 1ps
//////////////////////////////////////////////////////////////////////////////////
// Company: 
// Engineer: 
// 
// Create Date: 07/01/2025 04:03:01 AM
// Design Name: 
// Module Name: freq_to_dac
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


// Map measured frequency (Hz) to a signed 14-bit DAC code (-8192…+8191)

module freq_to_dac #(
    parameter integer CLK_HZ      = 125_000_000,   // Red Pitaya ADC/DAC clock
    parameter integer F_MIN_HZ    = 0,             // lower end of span (Hz)
    parameter integer F_MAX_HZ    = 62_500_000    // upper end of span (Hz)
)(
    input  wire                clk,
    input  wire                rst,
    input  wire  [31:0]        period_counts,      // counter_output from freq_counter
    input  wire                period_valid,       // 1 clk pulse when updated
    output wire signed [15:0]  m_axis_tdata,       // {Ch-B, Ch-A} packed
    output wire                m_axis_tvalid
);
    //------------------------------------------------------------------
    // 1) Convert period to frequency in Hz:  freq = CLK / period_counts
    //------------------------------------------------------------------
    // Use Xilinx div_gen IP for 32-bit / 32-bit unsigned divide.
    wire [31:0] freq_hz;
    wire        freq_valid;

    div_gen_0 reciprocal_u (
        .aclk   (clk),
        .s_axis_divisor_tvalid (period_valid),
        .s_axis_divisor_tdata  (period_counts),
        .s_axis_dividend_tvalid(period_valid),
        .s_axis_dividend_tdata(CLK_HZ),
        .m_axis_dout_tvalid    (freq_valid),
        .m_axis_dout_tdata     (freq_hz)
    );

    //------------------------------------------------------------------
    // 2) Linear map freq_hz -> signed 14-bit code
    //         code = round( (freq - Fcenter) * 8191 / (Fspan/2) )
    //------------------------------------------------------------------
    localparam integer FSPAN = F_MAX_HZ - F_MIN_HZ;
    localparam integer FCENT = F_MIN_HZ + (FSPAN >> 1);

    // signed (freq_hz - FCENT)
    wire signed [32:0] freq_delta = $signed({1'b0,freq_hz}) - FCENT;

    // scale with a single multiply.  Gain =  8191 / (FSPAN/2)
    // Pre-compute the reciprocal in fixed-point (18.14 format)
    localparam signed [31:0] GAIN_Q14 =
        (8191 << 14) / (FSPAN >> 1);      // round(8191 · 2^14 / (Fspan/2))

    wire signed [47:0] prod = freq_delta * GAIN_Q14; // 33 × 32 ? 48 bits

    // take the middle 16 bits ? Q14 ? signed 16-bit DAC word
    wire signed [15:0] dac_word = prod[28:13];

    //------------------------------------------------------------------
    // 3) Present a *constant* stream for the DAC (TVA=1 every clock)
    //------------------------------------------------------------------
    assign m_axis_tdata  = {2'b00,            // unused bits in 16-bit lane
                            dac_word[13:0],   // Ch A on lower 14 bits
                            16'h0000};        // Ch B left unused
    assign m_axis_tvalid = 1'b1;              // always ready
endmodule

