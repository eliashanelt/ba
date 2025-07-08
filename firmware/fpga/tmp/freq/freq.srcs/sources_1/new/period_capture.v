`timescale 1ns / 1ps

//////////////////////////////////////////////////////////////////////////////////
// Company: 
// Engineer: 
// 
// Create Date: 07.01.2025
// Design Name: 
// Module Name: period_capture
// Project Name: 
// Target Devices: Red Pitaya
// Tool Versions: 
// Description: Captures exactly one period of signal with 128 samples
// 
// Dependencies: frequency_counter module
// 
// Revision:
// Revision 0.1 - Initial implementation
// Additional Comments:
// 
//////////////////////////////////////////////////////////////////////////////////

module period_capture #
(
    parameter ADC_WIDTH = 14,
    parameter AXIS_TDATA_WIDTH = 32,
    parameter COUNT_WIDTH = 32,
    parameter MEMORY_DEPTH = 128,
    parameter MEMORY_ADDR_WIDTH = 7,  // log2(128) = 7
    parameter HIGH_THRESHOLD = -100,
    parameter LOW_THRESHOLD = -150
)
(
    (* X_INTERFACE_PARAMETER = "FREQ_HZ 125000000" *)
    input [AXIS_TDATA_WIDTH-1:0]   S_AXIS_IN_tdata,
    input                          S_AXIS_IN_tvalid,
    input                          clk,
    input                          rst,
    input                          capture_enable,
    
    // AXI-Lite interface for CPU access
    input [31:0]                   s_axi_awaddr,
    input                          s_axi_awvalid,
    output                         s_axi_awready,
    input [31:0]                   s_axi_wdata,
    input [3:0]                    s_axi_wstrb,
    input                          s_axi_wvalid,
    output                         s_axi_wready,
    output [1:0]                   s_axi_bresp,
    output                         s_axi_bvalid,
    input                          s_axi_bready,
    input [31:0]                   s_axi_araddr,
    input                          s_axi_arvalid,
    output                         s_axi_arready,
    output [31:0]                  s_axi_rdata,
    output [1:0]                   s_axi_rresp,
    output                         s_axi_rvalid,
    input                          s_axi_rready,
    
    // Status outputs
    output                         capture_done,
    output [COUNT_WIDTH-1:0]       period_length,
    output [AXIS_TDATA_WIDTH-1:0]  M_AXIS_OUT_tdata,
    output                         M_AXIS_OUT_tvalid,
    
    // Output the 4 i32 values for use in FPGA logic
    output [31:0]                  param0,
    output [31:0]                  param1,
    output [31:0]                  param2,
    output [31:0]                  param3
);

    // Internal signals
    wire signed [ADC_WIDTH-1:0]    data;
    reg                            state, state_next;
    reg [2:0]                      capture_state, capture_state_next;
    reg [MEMORY_ADDR_WIDTH-1:0]    write_addr, write_addr_next;
    reg [COUNT_WIDTH-1:0]          sample_counter, sample_counter_next;
    reg [COUNT_WIDTH-1:0]          period_length_reg, period_length_next;
    reg                            capture_done_reg, capture_done_next;
    reg                            mem_we;
    
    // Memory signals
    reg [15:0]                     memory [0:MEMORY_DEPTH-1];
    reg [MEMORY_ADDR_WIDTH-1:0]    read_addr;
    wire [15:0]                    mem_data_out;
    
    // Parameter registers for C-to-FPGA communication
    reg [31:0]                     param0_reg, param1_reg, param2_reg, param3_reg;
    
    // AXI-Lite signals
    reg                            axi_awready;
    reg                            axi_wready;
    reg [1:0]                      axi_bresp;
    reg                            axi_bvalid;
    reg                            axi_arready;
    reg [31:0]                     axi_rdata;
    reg [1:0]                      axi_rresp;
    reg                            axi_rvalid;
    
    // State machine states
    localparam IDLE = 3'b000;
    localparam WAIT_TRIGGER = 3'b001;
    localparam CAPTURING = 3'b010;
    localparam DONE = 3'b011;
    
    // Wire AXIS IN to AXIS OUT (passthrough)
    assign M_AXIS_OUT_tdata = S_AXIS_IN_tdata;
    assign M_AXIS_OUT_tvalid = S_AXIS_IN_tvalid;
    
    // Extract ADC data
    assign data = S_AXIS_IN_tdata[ADC_WIDTH-1:0];
    
    // Output assignments
    assign capture_done = capture_done_reg;
    assign period_length = period_length_reg;
    assign param0 = param0_reg;
    assign param1 = param1_reg;
    assign param2 = param2_reg;
    assign param3 = param3_reg;
    
    // Memory read
    assign mem_data_out = memory[read_addr];
    
    // AXI-Lite assignments
    assign s_axi_awready = axi_awready;
    assign s_axi_wready = axi_wready;
    assign s_axi_bresp = axi_bresp;
    assign s_axi_bvalid = axi_bvalid;
    assign s_axi_arready = axi_arready;
    assign s_axi_rdata = axi_rdata;
    assign s_axi_rresp = axi_rresp;
    assign s_axi_rvalid = axi_rvalid;
    
    // State detection logic (same as frequency counter)
    always @(posedge clk) 
    begin
        if (~rst) 
            state <= 1'b0;
        else
            state <= state_next;
    end
    
    always @*
    begin
        if (data > HIGH_THRESHOLD)
            state_next = 1;
        else if (data < LOW_THRESHOLD)
            state_next = 0;
        else
            state_next = state;
    end
    
    // Main capture state machine
    always @(posedge clk)
    begin
        if (~rst)
        begin
            capture_state <= IDLE;
            write_addr <= 0;
            sample_counter <= 0;
            period_length_reg <= 0;
            capture_done_reg <= 0;
            param0_reg <= 0;
            param1_reg <= 0;
            param2_reg <= 0;
            param3_reg <= 0;
        end
        else
        begin
            capture_state <= capture_state_next;
            write_addr <= write_addr_next;
            sample_counter <= sample_counter_next;
            period_length_reg <= period_length_next;
            capture_done_reg <= capture_done_next;
        end
    end
    
    // Capture logic
    always @*
    begin
        capture_state_next = capture_state;
        write_addr_next = write_addr;
        sample_counter_next = sample_counter;
        period_length_next = period_length_reg;
        capture_done_next = capture_done_reg;
        mem_we = 0;
        
        case (capture_state)
            IDLE:
            begin
                capture_done_next = 0;
                if (capture_enable)
                begin
                    capture_state_next = WAIT_TRIGGER;
                    write_addr_next = 0;
                    sample_counter_next = 0;
                end
            end
            
            WAIT_TRIGGER:
            begin
                if (S_AXIS_IN_tvalid && (state == 0 && state_next == 1)) // Rising edge trigger
                begin
                    capture_state_next = CAPTURING;
                    mem_we = 1;
                    write_addr_next = write_addr + 1;
                    sample_counter_next = sample_counter + 1;
                end
            end
            
            CAPTURING:
            begin
                if (S_AXIS_IN_tvalid)
                begin
                    mem_we = 1;
                    sample_counter_next = sample_counter + 1;
                    
                    // Check for next rising edge (end of period)
                    if ((state == 0 && state_next == 1) && sample_counter > 10) // Minimum 10 samples to avoid noise
                    begin
                        capture_state_next = DONE;
                        period_length_next = sample_counter;
                        capture_done_next = 1;
                    end
                    // Check if we've filled the buffer
                    else if (write_addr >= MEMORY_DEPTH - 1)
                    begin
                        capture_state_next = DONE;
                        period_length_next = sample_counter;
                        capture_done_next = 1;
                        write_addr_next = 0; // Wrap around
                    end
                    else
                    begin
                        write_addr_next = write_addr + 1;
                    end
                end
            end
            
            DONE:
            begin
                if (~capture_enable)
                    capture_state_next = IDLE;
            end
        endcase
    end
    
    // Memory write
    always @(posedge clk)
    begin
        if (mem_we && S_AXIS_IN_tvalid)
        begin
            // Store as signed 16-bit, extending from 14-bit ADC
            memory[write_addr] <= {{2{data[ADC_WIDTH-1]}}, data};
        end
    end
    
    // AXI-Lite interface implementation
    always @(posedge clk)
    begin
        if (~rst)
        begin
            axi_awready <= 0;
            axi_wready <= 0;
            axi_bvalid <= 0;
            axi_arready <= 0;
            axi_rvalid <= 0;
        end
        else
        begin
            // Write address ready
            if (~axi_awready && s_axi_awvalid)
                axi_awready <= 1;
            else
                axi_awready <= 0;
                
            // Write data ready
            if (~axi_wready && s_axi_wvalid)
                axi_wready <= 1;
            else
                axi_wready <= 0;
                
            // Write response
            if (~axi_bvalid && axi_awready && axi_wready)
            begin
                axi_bvalid <= 1;
                axi_bresp <= 2'b00; // OKAY
            end
            else if (axi_bvalid && s_axi_bready)
                axi_bvalid <= 0;
                
            // Read address ready
            if (~axi_arready && s_axi_arvalid)
                axi_arready <= 1;
            else
                axi_arready <= 0;
                
            // Read data valid
            if (~axi_rvalid && axi_arready)
            begin
                axi_rvalid <= 1;
                axi_rresp <= 2'b00; // OKAY
            end
            else if (axi_rvalid && s_axi_rready)
                axi_rvalid <= 0;
        end
    end
    
    // Write logic for parameter registers
    always @(posedge clk)
    begin
        if (~rst)
        begin
            // Initialize parameters to default values
            param0_reg <= 32'h00000000;
            param1_reg <= 32'h00000000;
            param2_reg <= 32'h00000000;
            param3_reg <= 32'h00000000;
        end
        else
        begin
            // Handle AXI write transactions
            if (axi_awready && s_axi_awvalid && axi_wready && s_axi_wvalid)
            begin
                case (s_axi_awaddr[7:2]) // Word addressing
                    6'h04: param0_reg <= s_axi_wdata;  // Address 0x10
                    6'h05: param1_reg <= s_axi_wdata;  // Address 0x14
                    6'h06: param2_reg <= s_axi_wdata;  // Address 0x18
                    6'h07: param3_reg <= s_axi_wdata;  // Address 0x1C
                    default: ; // Do nothing for other addresses
                endcase
            end
        end
    end
    
    // Read logic
    always @*
    begin
        // Address decoding for AXI reads
        case (s_axi_araddr[9:2]) // Word addressing
            8'h00: axi_rdata = {31'b0, capture_done_reg};
            8'h01: axi_rdata = period_length_reg;
            8'h02: axi_rdata = {29'b0, capture_state};
            8'h04: axi_rdata = param0_reg;  // Read back param0
            8'h05: axi_rdata = param1_reg;  // Read back param1
            8'h06: axi_rdata = param2_reg;  // Read back param2
            8'h07: axi_rdata = param3_reg;  // Read back param3
            default:
            begin
                if (s_axi_araddr[9:2] >= 8'h10 && s_axi_araddr[9:2] < (8'h10 + MEMORY_SIZE))
                begin
                    read_addr = s_axi_araddr[8:2] - 7'h10; // Memory starts at offset 0x40
                    axi_rdata = {16'b0, mem_data_out};
                end
                else
                    axi_rdata = 32'h0;
            end
        endcase
    end
    
endmodule