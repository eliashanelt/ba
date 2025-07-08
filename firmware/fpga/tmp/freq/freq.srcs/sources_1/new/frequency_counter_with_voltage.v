`timescale 1ns / 1ps
//////////////////////////////////////////////////////////////////////////////////
// Company: 
// Engineer: 
// 
// Create Date: 07/08/2025 08:11:50 PM
// Design Name: 
// Module Name: frequency_counter_with_voltage
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


module frequency_counter_with_voltage #
(
    parameter ADC_WIDTH = 14,
    parameter AXIS_TDATA_WIDTH = 32,
    parameter COUNT_WIDTH = 32,
    parameter HIGH_THRESHOLD = -100,
    parameter LOW_THRESHOLD = -150,
    parameter CLK_FREQ = 125000000,           // 125 MHz clock
    parameter MIN_FREQ = 1000,                // Minimum frequency to measure (1 kHz)
    parameter MAX_FREQ = 10000000,            // Maximum frequency to measure (10 MHz)
    parameter DAC_WIDTH = 14                  // DAC resolution (same as ADC)
)
(
    (* X_INTERFACE_PARAMETER = "FREQ_HZ 125000000" *)
    input [AXIS_TDATA_WIDTH-1:0]   S_AXIS_IN_tdata,
    input                          S_AXIS_IN_tvalid,
    input                          clk,
    input                          rst,
    input [COUNT_WIDTH-1:0]        Ncycles,
    
    // Two separate AXI Stream outputs
    output [AXIS_TDATA_WIDTH-1:0]  M_AXIS_OUT1_tdata,    // Original signal passthrough
    output                         M_AXIS_OUT1_tvalid,
    output [AXIS_TDATA_WIDTH-1:0]  M_AXIS_OUT2_tdata,    // Frequency voltage output
    output                         M_AXIS_OUT2_tvalid,
    
    output [COUNT_WIDTH-1:0]       counter_output,
    output [COUNT_WIDTH-1:0]       frequency_hz          // Calculated frequency in Hz
);

    // Internal signals
    wire [COUNT_WIDTH-1:0] freq_counter_output;
    reg [COUNT_WIDTH-1:0] frequency_hz_reg;
    reg [DAC_WIDTH-1:0] voltage_output;
    reg output_valid;
    
    // Instantiate the original frequency counter
    frequency_counter #(
        .ADC_WIDTH(ADC_WIDTH),
        .AXIS_TDATA_WIDTH(AXIS_TDATA_WIDTH),
        .COUNT_WIDTH(COUNT_WIDTH),
        .HIGH_THRESHOLD(HIGH_THRESHOLD),
        .LOW_THRESHOLD(LOW_THRESHOLD)
    ) freq_counter_inst (
        .S_AXIS_IN_tdata(S_AXIS_IN_tdata),
        .S_AXIS_IN_tvalid(S_AXIS_IN_tvalid),
        .clk(clk),
        .rst(rst),
        .Ncycles(Ncycles),
        .M_AXIS_OUT_tdata(M_AXIS_OUT1_tdata),      // Direct passthrough to output1
        .M_AXIS_OUT_tvalid(M_AXIS_OUT1_tvalid),
        .counter_output(freq_counter_output)
    );
    
    assign counter_output = freq_counter_output;
    assign frequency_hz = frequency_hz_reg;
    
    // Calculate frequency in Hz from counter output
    // Frequency = CLK_FREQ / (counter_output / Ncycles)
    // Simplified: Frequency = (CLK_FREQ * Ncycles) / counter_output
    always @(posedge clk) begin
        if (~rst) begin
            frequency_hz_reg <= 0;
            voltage_output <= 0;
            output_valid <= 0;
        end
        else begin
            if (freq_counter_output > 0) begin
                // Calculate frequency: f = (CLK_FREQ * Ncycles) / counter_output
                frequency_hz_reg <= (CLK_FREQ * Ncycles) / freq_counter_output;
                
                // Convert frequency to voltage (-1V to +1V)
                // Map MIN_FREQ to -1V (0x2000 for 14-bit 2's complement)
                // Map MAX_FREQ to +1V (0x1FFF for 14-bit 2's complement)
                if (frequency_hz_reg < MIN_FREQ) begin
                    voltage_output <= 14'h2000;  // -1V (minimum)
                end
                else if (frequency_hz_reg > MAX_FREQ) begin
                    voltage_output <= 14'h1FFF;  // +1V (maximum)
                end
                else begin
                    // Linear interpolation between MIN_FREQ and MAX_FREQ
                    // voltage = -1 + 2 * (freq - MIN_FREQ) / (MAX_FREQ - MIN_FREQ)
                    // For 14-bit signed: -1V = 0x2000, +1V = 0x1FFF
                    voltage_output <= 14'h2000 + 
                        (((frequency_hz_reg - MIN_FREQ) * 14'h3FFF) / (MAX_FREQ - MIN_FREQ));
                end
                
                output_valid <= 1'b1;
            end
            else begin
                voltage_output <= 14'h2000;  // Default to -1V when no signal
                output_valid <= 1'b0;
            end
        end
    end
    
    // Output2: Frequency as voltage
    assign M_AXIS_OUT2_tdata[DAC_WIDTH-1:0] = voltage_output;
    assign M_AXIS_OUT2_tdata[AXIS_TDATA_WIDTH-1:DAC_WIDTH] = 0;  // Pad with zeros
    assign M_AXIS_OUT2_tvalid = output_valid;
    
endmodule
