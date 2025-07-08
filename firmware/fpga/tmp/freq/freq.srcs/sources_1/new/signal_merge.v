`timescale 1ns / 1ps
//////////////////////////////////////////////////////////////////////////////////
// Company: 
// Engineer: 
// 
// Create Date: 07/08/2025 07:42:20 PM
// Design Name: 
// Module Name: signal_merge
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


//  ????????????????????????????????????
//  ?  Port-1 (Chan-A)  | Port-2 (Chan-B) ?
//  ???????????????????????????????????????
//                ??? merge  ??
//
// Result:  M_AXIS_tdata[15:0]  = Chan-A (LSBs)
//          M_AXIS_tdata[31:16] = Chan-B (LSBs)
//
//          M_AXIS_tvalid = 1 when *both* input words are present
//
module signal_merge #
(
    parameter ADC_DATA_WIDTH  = 16 ,   // width of a single sample
    parameter AXIS_TDATA_WIDTH = 32    // must be 2×ADC_DATA_WIDTH
)
(
    // port-1: channel-A stream (sign-extended to 32 b by your splitter)
    (* X_INTERFACE_PARAMETER = "FREQ_HZ 125000000" *)
    input  wire [AXIS_TDATA_WIDTH-1:0] S_AXIS_tdata,
    input  wire                        S_AXIS_tvalid,

    // combined dual-channel stream
    (* X_INTERFACE_PARAMETER = "FREQ_HZ 125000000" *)
    output wire [AXIS_TDATA_WIDTH-1:0] M_AXIS_tdata,
    output wire                        M_AXIS_tvalid
);

localparam [ADC_DATA_WIDTH-1:0] CONST_8191 = 8191;  // auto-truncated if width < 13

assign M_AXIS_tdata = {
                       S_AXIS_tdata[ADC_DATA_WIDTH-1:0],   // MSB half
                       CONST_8191
                      };
    assign M_AXIS_tvalid = S_AXIS_tvalid;

endmodule