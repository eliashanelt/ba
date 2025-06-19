`timescale 1ns / 1ps
//////////////////////////////////////////////////////////////////////////////////
// Company: 
// Engineer: 
// 
// Create Date: 06/19/2025 03:00:03 PM
// Design Name: 
// Module Name: passthrough
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


module passthrough # 
(
    parameter UPDATE_RATE = 1,
    parameter CLOCK_FREQ = 125000000
)(
    (* X_INTERFACE_PARAMETER = "FREQ_HZ 125000000" *)
    input wire clk,           // System clock (usually 125 MHz for Red Pitaya)
    input wire rst_n,         // Active low reset
    
    // Signal input/output
    input wire signed [13:0] signal_in,    // 14-bit signed input from ADC
    output wire signed [13:0] signal_out,  // 14-bit signed output to DAC
    
    // Frequency counter output
    output reg [31:0] frequency_hz,
    output reg freq_valid,
    
    // Optional: threshold for zero-crossing detection
    input wire signed [13:0] threshold
);


// Internal signals
reg signed [13:0] prev_sample;
reg [31:0] edge_count;
reg [31:0] sample_count;
reg [26:0] update_counter; // Counter for 1-second updates

// **Direct pass-through from input to output**
assign signal_out = signal_in;

// Frequency counting logic
always @(posedge clk) begin
    if (!rst_n) begin
        prev_sample <= 14'd0;
        edge_count <= 32'd0;
        sample_count <= 32'd0;
        update_counter <= 27'd0;
        frequency_hz <= 32'd0;
        freq_valid <= 1'b0;
    end else begin
        // Store previous sample for edge detection
        prev_sample <= signal_in;
        
        // Detect positive zero crossings (or threshold crossings)
        if ((prev_sample < threshold) && (signal_in >= threshold)) begin
            edge_count <= edge_count + 1;
        end
        
        // Update counter for 1-second intervals
        update_counter <= update_counter + 1;
        
        // Every second, calculate frequency
        if (update_counter >= (CLOCK_FREQ/UPDATE_RATE - 1)) begin
            frequency_hz <= edge_count * UPDATE_RATE; // Hz = edges per second
            edge_count <= 32'd0;
            update_counter <= 27'd0;
            freq_valid <= 1'b1;
        end else begin
            freq_valid <= 1'b0;
        end
    end
end



endmodule
