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
module signal_merge #(
    parameter ADC_WIDTH        = 16,
    parameter AXIS_TDATA_WIDTH = 32
)(
    (* X_INTERFACE_PARAMETER = "FREQ_HZ 125000000" *)
    input  wire [AXIS_TDATA_WIDTH-1:0]  S_AXIS_tdata,
    input  wire                         S_AXIS_tvalid,
    input  wire signed [ADC_WIDTH-1:0]  freq_amp,
        
    (* X_INTERFACE_PARAMETER = "FREQ_HZ 125000000" *)
    output wire [AXIS_TDATA_WIDTH-1:0]  M_AXIS_tdata,
    output wire                         M_AXIS_tvalid
);
    assign M_AXIS_tdata  = { freq_amp, S_AXIS_tdata[ADC_WIDTH-1:0] };
    assign M_AXIS_tvalid = S_AXIS_tvalid;
endmodule