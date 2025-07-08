`timescale 1ns / 1ps
//////////////////////////////////////////////////////////////////////////////////
// Company: 
// Engineer: 
// 
// Create Date: 07/08/2025 04:18:12 PM
// Design Name: 
// Module Name: adc_to_dual_dac
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


module adc_to_dual_dac #(
    parameter DATA_WIDTH       = 14,  // Red Pitaya fast ADC/DAC width
    parameter AXIS_TDATA_WIDTH = 32   // must stay 32 for the DAC IP
)(
    input                       clk,
    input                       rst,                // active-high synchronous reset

    // ??? AXI-Stream input ???????????????????????????????????????????
    input  [AXIS_TDATA_WIDTH-1:0] S_AXIS_IN_tdata,
    input                        S_AXIS_IN_tvalid,
    // input                     S_AXIS_IN_tready,  // uncomment if needed

    // ??? AXI-Stream output (to axis_red_pitaya_dac) ????????????????
    output reg [AXIS_TDATA_WIDTH-1:0] M_AXIS_OUT_tdata,
    output reg                     M_AXIS_OUT_tvalid
    // output                    M_AXIS_OUT_tready  // uncomment if needed
);

    //----------------------------------------------------------------
    // Combinational packing
    //----------------------------------------------------------------
    wire [DATA_WIDTH-1:0] sample_chA = S_AXIS_IN_tdata[DATA_WIDTH-1:0];

    // Pack the 32-bit word: 2x14-bit sample plus two 2-bit gaps (= 32 bits)
    //   [31:30]   unused  ? 0
    //   [29:16]   channel B (duplicate of channel A)
    //   [15:14]   unused  ? 0
    //   [13:0 ]   channel A
    wire [AXIS_TDATA_WIDTH-1:0] next_tdata = {
        2'b00,              // MSBs unused
        sample_chA,         // Channel B (OUT2)
        2'b00,              // gap
        sample_chA          // Channel A (OUT1)
    };

    //----------------------------------------------------------------
    // Sequential logic
    //----------------------------------------------------------------
    always @(posedge clk) begin
        if (rst) begin
            // synchronous reset: drive zeros / invalid
            M_AXIS_OUT_tdata  <= {AXIS_TDATA_WIDTH{1'b0}};
            M_AXIS_OUT_tvalid <= 1'b0;
        end
        else begin
            // normal operation
            M_AXIS_OUT_tdata  <= next_tdata;
            M_AXIS_OUT_tvalid <= S_AXIS_IN_tvalid;
        end
    end

endmodule
