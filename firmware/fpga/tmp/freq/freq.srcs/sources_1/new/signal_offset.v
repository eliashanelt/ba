`timescale 1ns / 1ps
//////////////////////////////////////////////////////////////////////////////////
// Company: 
// Engineer: 
// 
// Create Date: 07/01/2025 01:40:11 PM
// Design Name: 
// Module Name: signal_offset
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
module signal_offset #(
    parameter DATA_WIDTH = 14,        // RedPitaya ADC/DAC width
    parameter AXIS_TDATA_WIDTH = 32   // AXI Stream data width
)(
    input clk,
    input rst,
    
    // Offset control input
    input signed [DATA_WIDTH-1:0] offset_value,
    
    // Input (from previous component like clipper)
    input [AXIS_TDATA_WIDTH-1:0] S_AXIS_IN_tdata,
    input S_AXIS_IN_tvalid,
    
    // Output with offset applied
    output reg [AXIS_TDATA_WIDTH-1:0] M_AXIS_OUT_tdata,
    output reg M_AXIS_OUT_tvalid
);
    // Extract ADC data from AXI stream
    wire signed [DATA_WIDTH-1:0] adc_data;
    assign adc_data = S_AXIS_IN_tdata[DATA_WIDTH-1:0];
    
    // Offset ADC data
    reg signed [DATA_WIDTH-1:0] adc_data_offset;
    always @(posedge clk) begin
        if (~rst) begin
            M_AXIS_OUT_tdata <= 0;
            M_AXIS_OUT_tvalid <= 1'b0;
        end
        else begin
            // Apply offset to ADC data using input offset_value
            adc_data_offset <= adc_data + offset_value;
            
            // Output: offset ADC data + pass through everything else unchanged
            M_AXIS_OUT_tdata[DATA_WIDTH-1:0] <= adc_data_offset;
            M_AXIS_OUT_tdata[AXIS_TDATA_WIDTH-1:DATA_WIDTH] <= S_AXIS_IN_tdata[AXIS_TDATA_WIDTH-1:DATA_WIDTH];
            M_AXIS_OUT_tvalid <= S_AXIS_IN_tvalid;
        end
    end
endmodule
