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
    parameter LIMIT_VALUE = 8191      // Adjustable limit
)(
    input clk,
    input rst,
    
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

    always @(posedge clk) begin
        if (~rst) begin
            M_AXIS_OUT_tdata <= 0;
            M_AXIS_OUT_tvalid <= 1'b0;
        end
        else begin
            // Clip only the ADC amplitude
            if (adc_data > LIMIT_VALUE)
                adc_data_clipped <= LIMIT_VALUE;
            else if (adc_data < -LIMIT_VALUE)
                adc_data_clipped <= -LIMIT_VALUE;
            else
                adc_data_clipped <= adc_data;
            
            // Output: clipped ADC data + pass through everything else unchanged
            M_AXIS_OUT_tdata[DATA_WIDTH-1:0] <= adc_data_clipped;
            M_AXIS_OUT_tdata[AXIS_TDATA_WIDTH-1:DATA_WIDTH] <= S_AXIS_IN_tdata[AXIS_TDATA_WIDTH-1:DATA_WIDTH];
            M_AXIS_OUT_tvalid <= S_AXIS_IN_tvalid;
        end
    end

endmodule