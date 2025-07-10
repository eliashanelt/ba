`timescale 1ns / 1ps
//////////////////////////////////////////////////////////////////////////////////
// Company: 
// Engineer: 
// 
// Create Date: 07/10/2025 05:53:45 PM
// Design Name: 
// Module Name: signal_gain
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


// For gain of 2.5, set gain to:
//assign gain = 32'd2560;  2.5 * 2^10

module signal_gain #
(
    parameter ADC_WIDTH = 14,
    parameter DAC_WIDTH = 14,
    parameter AXIS_TDATA_WIDTH = 32,
    parameter GAIN_WIDTH = 32,
    parameter GAIN_FRAC_BITS = 10  // Number of fractional bits in gain (gain = actual_gain * 2^10)
)
(
    input                          clk,
    input                          rst,
    input signed [GAIN_WIDTH-1:0]  gain,           // Gain value: -10000 to 10000
    
    // AXI Stream Input
    input [AXIS_TDATA_WIDTH-1:0]   S_AXIS_tdata,
    input                          S_AXIS_tvalid,
    output                         S_AXIS_tready,
    
    // AXI Stream Output
    output [AXIS_TDATA_WIDTH-1:0]  M_AXIS_tdata,
    output                         M_AXIS_tvalid,
    input                          M_AXIS_tready
);

    // Internal signals
    wire signed [ADC_WIDTH-1:0]    adc_data;
    wire signed [DAC_WIDTH-1:0]    dac_data;
    
    // Pipeline registers for timing
    reg [AXIS_TDATA_WIDTH-1:0]     axis_data_reg;
    reg                            axis_valid_reg;
    
    // Multiplication and saturation logic
    wire signed [ADC_WIDTH + GAIN_WIDTH - 1:0] mult_result;
    wire signed [ADC_WIDTH + GAIN_WIDTH - GAIN_FRAC_BITS - 1:0] scaled_result;
    
    // DAC range constants
    localparam signed [DAC_WIDTH-1:0] DAC_MAX = (1 << (DAC_WIDTH-1)) - 1;  // 8191 for 14-bit
    localparam signed [DAC_WIDTH-1:0] DAC_MIN = -(1 << (DAC_WIDTH-1));     // -8192 for 14-bit
    
    // Extract ADC data from input stream
    assign adc_data = S_AXIS_tdata[ADC_WIDTH-1:0];
    
    // Multiply ADC data with gain
    assign mult_result = adc_data * gain;
    
    // Scale down by fractional bits (divide by 2^GAIN_FRAC_BITS)
    assign scaled_result = mult_result >>> GAIN_FRAC_BITS;
    
    // Saturate to DAC range
    assign dac_data = (scaled_result > DAC_MAX) ? DAC_MAX :
                      (scaled_result < DAC_MIN) ? DAC_MIN :
                      scaled_result[DAC_WIDTH-1:0];
    
    // AXI Stream flow control
    assign S_AXIS_tready = M_AXIS_tready;  // Simple pass-through for backpressure
    
    // Pipeline the data for better timing
    always @(posedge clk) begin
        if (~rst) begin
            axis_data_reg <= 0;
            axis_valid_reg <= 1'b0;
        end else if (M_AXIS_tready) begin
            // Pass through upper bits unchanged, replace lower bits with DAC data
            axis_data_reg <= {S_AXIS_tdata[AXIS_TDATA_WIDTH-1:DAC_WIDTH], dac_data};
            axis_valid_reg <= S_AXIS_tvalid;
        end
    end
    
    // Output assignment
    assign M_AXIS_tdata = axis_data_reg;
    assign M_AXIS_tvalid = axis_valid_reg;

endmodule