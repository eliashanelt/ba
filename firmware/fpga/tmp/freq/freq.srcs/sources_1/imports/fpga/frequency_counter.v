`timescale 1ns / 1ps
//////////////////////////////////////////////////////////////////////////////////
// Company: 
// Engineer: Anton Potocnik
// 
// Create Date: 07.01.2017 22:50:51
// Design Name: 
// Module Name: frequency_counter
// Project Name: 
// Target Devices: 
// Tool Versions: 
// Description:  Reciprocal method with VP and VPP measurement
// 
// Dependencies: 
// 
// Revision:
// Revision 0.2 - Added VP and VPP outputs with 32-bit width
// Additional Comments:
// 
//////////////////////////////////////////////////////////////////////////////////
module frequency_counter #
(
    parameter ADC_WIDTH = 14,
    parameter AXIS_TDATA_WIDTH = 32,
    parameter COUNT_WIDTH = 32,
    parameter HIGH_THRESHOLD = -100,
    parameter LOW_THRESHOLD = -150
)
(
    (* X_INTERFACE_PARAMETER = "FREQ_HZ 125000000" *)
    input [AXIS_TDATA_WIDTH-1:0]   S_AXIS_IN_tdata,
    input                          S_AXIS_IN_tvalid,
    input                          clk,
    input                          rst,
    input [COUNT_WIDTH-1:0]        Ncycles,
    output [AXIS_TDATA_WIDTH-1:0]  M_AXIS_OUT_tdata,
    output                         M_AXIS_OUT_tvalid,
    output [COUNT_WIDTH-1:0]       counter_output,
    output [31:0]                  vp_output,      // Peak voltage (32-bit)
    output [31:0]                  vpp_output      // Peak-to-peak voltage (32-bit)
);
    
    wire signed [ADC_WIDTH-1:0]    data;
    reg                            state, state_next;
    reg [COUNT_WIDTH-1:0]          counter=0, counter_next=0;
    reg [COUNT_WIDTH-1:0]          counter_output=0, counter_output_next=0;
    reg [COUNT_WIDTH-1:0]          cycle=0, cycle_next=0;    
    
    // Registers for VP and VPP measurement
    reg signed [ADC_WIDTH-1:0]     max_value, max_value_next;
    reg signed [ADC_WIDTH-1:0]     min_value, min_value_next;
    reg [31:0]                     vp_output, vp_output_next;
    reg [31:0]                     vpp_output, vpp_output_next;
    
    // Wire AXIS IN to AXIS OUT
    assign  M_AXIS_OUT_tdata[ADC_WIDTH-1:0] = S_AXIS_IN_tdata[ADC_WIDTH-1:0];
    assign  M_AXIS_OUT_tvalid = S_AXIS_IN_tvalid;
    
    // Extract only the 14-bits of ADC data 
    assign  data = S_AXIS_IN_tdata[ADC_WIDTH-1:0];
    
    // Handling of the state buffer for finding signal transition at the threshold
    always @(posedge clk) 
    begin
        if (~rst) 
            state <= 1'b0;
        else
            state <= state_next;
    end
    
    always @*            // logic for state buffer
    begin
        if (data > HIGH_THRESHOLD)
            state_next = 1;
        else if (data < LOW_THRESHOLD)
            state_next = 0;
        else
            state_next = state;
    end
    
    // Handling of all measurement buffers
    always @(posedge clk) 
    begin
        if (~rst) 
        begin
            counter <= 0;
            counter_output <= 0;
            cycle <= 0;
            max_value <= {1'b1, {(ADC_WIDTH-1){1'b0}}}; // Most negative value
            min_value <= {1'b0, {(ADC_WIDTH-1){1'b1}}}; // Most positive value
            vp_output <= 0;
            vpp_output <= 0;
        end
        else
        begin
            counter <= counter_next;
            counter_output <= counter_output_next;
            cycle <= cycle_next;
            max_value <= max_value_next;
            min_value <= min_value_next;
            vp_output <= vp_output_next;
            vpp_output <= vpp_output_next;
        end
    end
    
    always @* // logic for all measurements
    begin
        // Default assignments
        counter_next = counter + 1; // increment on each clock cycle
        counter_output_next = counter_output;
        cycle_next = cycle;
        max_value_next = max_value;
        min_value_next = min_value;
        vp_output_next = vp_output;
        vpp_output_next = vpp_output;
        
        // Track max and min values continuously
        if (S_AXIS_IN_tvalid)
        begin
            if (data > max_value)
                max_value_next = data;
            if (data < min_value)
                min_value_next = data;
        end
                
        if (state < state_next) // low to high signal transition
        begin
            cycle_next = cycle + 1; // increment on each signal transition
            if (cycle >= Ncycles-1) 
            begin
                // Update all outputs
                counter_next = 0;
                counter_output_next = counter;
                cycle_next = 0;
                
                // Calculate and output VP and VPP with sign extension to 32 bits
                vp_output_next = {{(32-ADC_WIDTH){max_value[ADC_WIDTH-1]}}, max_value};
                vpp_output_next = {{(32-ADC_WIDTH){1'b0}}, max_value - min_value};
                
                // Reset max/min for next measurement cycle
                max_value_next = {1'b1, {(ADC_WIDTH-1){1'b0}}}; // Most negative value
                min_value_next = {1'b0, {(ADC_WIDTH-1){1'b1}}}; // Most positive value
            end
        end
   end
    
endmodule