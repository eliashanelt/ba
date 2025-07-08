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
    parameter DATA_WIDTH = 14,        // RedPitaya ADC/DAC width
    parameter AXIS_TDATA_WIDTH = 32,  // AXI Stream data width
    parameter DEFAULT_MAX_LIMIT = 8191,  // Default maximum limit
    parameter DEFAULT_MIN_LIMIT = -8191 // Default minimum limit
)(
    input clk,
    input rst,
    
    // Configurable limit inputs
    input signed [DATA_WIDTH-1:0] max_limit,
    input signed [DATA_WIDTH-1:0] min_limit,
    
    // Input from frequency counter
    input [AXIS_TDATA_WIDTH-1:0] S_AXIS_IN_tdata,
    input S_AXIS_IN_tvalid,
    
    // Output with clipped amplitude
    output reg [AXIS_TDATA_WIDTH-1:0] M_AXIS_OUT_tdata,
    output reg M_AXIS_OUT_tvalid
);
    // Extract ADC data from AXI stream
    wire signed [DATA_WIDTH-1:0] adc_data;
    assign adc_data = S_AXIS_IN_tdata[DATA_WIDTH-1:0];
    
    // Clipped ADC data
    reg signed [DATA_WIDTH-1:0] adc_data_clipped;
    
    // Internal limit values (use defaults if limits are invalid)
    wire signed [DATA_WIDTH-1:0] effective_max_limit;
    wire signed [DATA_WIDTH-1:0] effective_min_limit;
    
    // Ensure min <= max, use defaults if invalid
    assign effective_max_limit = (min_limit <= max_limit) ? max_limit : DEFAULT_MAX_LIMIT;
    assign effective_min_limit = (min_limit <= max_limit) ? min_limit : DEFAULT_MIN_LIMIT;
    
    always @(posedge clk) begin
        if (~rst) begin
            M_AXIS_OUT_tdata <= 0;
            M_AXIS_OUT_tvalid <= 1'b0;
            adc_data_clipped <= 0;
        end
        else begin
            // Clip the ADC amplitude using configurable limits
            if (adc_data > effective_max_limit)
                adc_data_clipped <= effective_max_limit;
            else if (adc_data < effective_min_limit)
                adc_data_clipped <= effective_min_limit;
            else
                adc_data_clipped <= adc_data;
            
            // Output: clipped ADC data + pass through everything else unchanged
            M_AXIS_OUT_tdata[DATA_WIDTH-1:0] <= adc_data_clipped;
            M_AXIS_OUT_tdata[AXIS_TDATA_WIDTH-1:DATA_WIDTH] <= S_AXIS_IN_tdata[AXIS_TDATA_WIDTH-1:DATA_WIDTH];
            M_AXIS_OUT_tvalid <= S_AXIS_IN_tvalid;
        end
    end
endmodule