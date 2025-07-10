`timescale 1ns / 1ps

module redpitaya_mem_interface #(
    parameter integer C_S_AXI_DATA_WIDTH = 32,
    parameter integer C_S_AXI_ADDR_WIDTH = 32
)(
    // AXI4-Lite slave interface
    input  wire                                   s_axi_aclk,
    input  wire                                   s_axi_aresetn,
    input  wire [C_S_AXI_ADDR_WIDTH-1:0]        s_axi_awaddr,
    input  wire [2:0]                            s_axi_awprot,
    input  wire                                   s_axi_awvalid,
    output wire                                   s_axi_awready,
    input  wire [C_S_AXI_DATA_WIDTH-1:0]        s_axi_wdata,
    input  wire [(C_S_AXI_DATA_WIDTH/8)-1:0]    s_axi_wstrb,
    input  wire                                   s_axi_wvalid,
    output wire                                   s_axi_wready,
    output wire [1:0]                            s_axi_bresp,
    output wire                                   s_axi_bvalid,
    input  wire                                   s_axi_bready,
    input  wire [C_S_AXI_ADDR_WIDTH-1:0]        s_axi_araddr,
    input  wire [2:0]                            s_axi_arprot,
    input  wire                                   s_axi_arvalid,
    output wire                                   s_axi_arready,
    output wire [C_S_AXI_DATA_WIDTH-1:0]        s_axi_rdata,
    output wire [1:0]                            s_axi_rresp,
    output wire                                   s_axi_rvalid,
    input  wire                                   s_axi_rready,
    
    // User registers - inputs from your application
    input  wire [31:0]                           frequency,
    input  wire [31:0]                           vpp,
    input  wire [31:0]                           vp,
    
    // User registers - outputs to your application
    output wire [31:0]                           limits,
    output wire [31:0]                           gain,
    output wire [31:0]                           delay
);

    // AXI4LITE signals
    reg [C_S_AXI_ADDR_WIDTH-1:0]  axi_awaddr;
    reg                            axi_awready;
    reg                            axi_wready;
    reg [1:0]                      axi_bresp;
    reg                            axi_bvalid;
    reg [C_S_AXI_ADDR_WIDTH-1:0]  axi_araddr;
    reg                            axi_arready;
    reg [C_S_AXI_DATA_WIDTH-1:0]  axi_rdata;
    reg [1:0]                      axi_rresp;
    reg                            axi_rvalid;

    // Example-specific design signals
    // local parameter for addressing 32 bit / 64 bit C_S_AXI_DATA_WIDTH
    // ADDR_LSB is used for addressing 32/64 bit registers/memories
    // ADDR_LSB = 2 for 32 bits (n downto 2)
    // ADDR_LSB = 3 for 64 bits (n downto 3)
    localparam integer ADDR_LSB = (C_S_AXI_DATA_WIDTH/32) + 1;
    localparam integer OPT_MEM_ADDR_BITS = 3;
    
    //----------------------------------------------
    //-- Signals for user logic register space
    //----------------------------------------------
    //-- Number of Slave Registers 6
    reg [C_S_AXI_DATA_WIDTH-1:0]   slv_reg0;  // frequency (read-only)
    reg [C_S_AXI_DATA_WIDTH-1:0]   slv_reg1;  // vpp (read-only)
    reg [C_S_AXI_DATA_WIDTH-1:0]   slv_reg2;  // vp (read-only)
    reg [C_S_AXI_DATA_WIDTH-1:0]   slv_reg3;  // limits (writable)
    reg [C_S_AXI_DATA_WIDTH-1:0]   slv_reg4;  // gain (writable)
    reg [C_S_AXI_DATA_WIDTH-1:0]   slv_reg5;  // delay (writable)
    
    wire                            slv_reg_rden;
    wire                            slv_reg_wren;
    reg [C_S_AXI_DATA_WIDTH-1:0]    reg_data_out;
    integer                         byte_index;
    reg                             aw_en;

    // I/O Connections assignments
    assign s_axi_awready = axi_awready;
    assign s_axi_wready  = axi_wready;
    assign s_axi_bresp   = axi_bresp;
    assign s_axi_bvalid  = axi_bvalid;
    assign s_axi_arready = axi_arready;
    assign s_axi_rdata   = axi_rdata;
    assign s_axi_rresp   = axi_rresp;
    assign s_axi_rvalid  = axi_rvalid;
    
    // User logic connections
    assign limits = slv_reg3;
    assign gain = slv_reg4;
    assign delay = slv_reg5;

    // Implement axi_awready generation
    always @(posedge s_axi_aclk) begin
        if (!s_axi_aresetn) begin
            axi_awready <= 1'b0;
            aw_en <= 1'b1;
        end else begin    
            if (!axi_awready && s_axi_awvalid && s_axi_wvalid && aw_en) begin
                axi_awready <= 1'b1;
                aw_en <= 1'b0;
            end else if (s_axi_bready && axi_bvalid) begin
                aw_en <= 1'b1;
                axi_awready <= 1'b0;
            end else begin
                axi_awready <= 1'b0;
            end
        end
    end       

    // Implement axi_awaddr latching
    always @(posedge s_axi_aclk) begin
        if (!s_axi_aresetn) begin
            axi_awaddr <= 0;
        end else begin    
            if (!axi_awready && s_axi_awvalid && s_axi_wvalid && aw_en) begin
                axi_awaddr <= s_axi_awaddr;
            end
        end
    end       

    // Implement axi_wready generation
    always @(posedge s_axi_aclk) begin
        if (!s_axi_aresetn) begin
            axi_wready <= 1'b0;
        end else begin    
            if (!axi_wready && s_axi_wvalid && s_axi_awvalid && aw_en) begin
                axi_wready <= 1'b1;
            end else begin
                axi_wready <= 1'b0;
            end
        end
    end       

    // Implement memory mapped register select and write logic generation
    assign slv_reg_wren = axi_wready && s_axi_wvalid && axi_awready && s_axi_awvalid;

    always @(posedge s_axi_aclk) begin
        if (!s_axi_aresetn) begin
            slv_reg3 <= 0;
            slv_reg4 <= 0;
            slv_reg5 <= 0;
        end else begin
            if (slv_reg_wren) begin
                case (axi_awaddr[ADDR_LSB+OPT_MEM_ADDR_BITS:ADDR_LSB])
                    3'h3:
                        for (byte_index = 0; byte_index <= (C_S_AXI_DATA_WIDTH/8)-1; byte_index = byte_index+1)
                            if (s_axi_wstrb[byte_index] == 1) begin
                                slv_reg3[(byte_index*8) +: 8] <= s_axi_wdata[(byte_index*8) +: 8];
                            end  
                    3'h4:
                        for (byte_index = 0; byte_index <= (C_S_AXI_DATA_WIDTH/8)-1; byte_index = byte_index+1)
                            if (s_axi_wstrb[byte_index] == 1) begin
                                slv_reg4[(byte_index*8) +: 8] <= s_axi_wdata[(byte_index*8) +: 8];
                            end  
                    3'h5:
                        for (byte_index = 0; byte_index <= (C_S_AXI_DATA_WIDTH/8)-1; byte_index = byte_index+1)
                            if (s_axi_wstrb[byte_index] == 1) begin
                                slv_reg5[(byte_index*8) +: 8] <= s_axi_wdata[(byte_index*8) +: 8];
                            end  
                    default : begin
                        slv_reg3 <= slv_reg3;
                        slv_reg4 <= slv_reg4;
                        slv_reg5 <= slv_reg5;
                    end
                endcase
            end
        end
    end    

    // Implement write response logic generation
    always @(posedge s_axi_aclk) begin
        if (!s_axi_aresetn) begin
            axi_bvalid  <= 0;
            axi_bresp   <= 2'b0;
        end else begin    
            if (axi_awready && s_axi_awvalid && !axi_bvalid && axi_wready && s_axi_wvalid) begin
                axi_bvalid <= 1'b1;
                axi_bresp  <= 2'b0; // 'OKAY' response 
            end else begin
                if (s_axi_bready && axi_bvalid) begin
                    axi_bvalid <= 1'b0;
                end  
            end
        end
    end   

    // Implement axi_arready generation
    always @(posedge s_axi_aclk) begin
        if (!s_axi_aresetn) begin
            axi_arready <= 1'b0;
            axi_araddr  <= 32'b0;
        end else begin    
            if (!axi_arready && s_axi_arvalid) begin
                axi_arready <= 1'b1;
                axi_araddr  <= s_axi_araddr;
            end else begin
                axi_arready <= 1'b0;
            end
        end
    end       

    // Implement axi_arvalid generation
    always @(posedge s_axi_aclk) begin
        if (!s_axi_aresetn) begin
            axi_rvalid <= 0;
            axi_rresp  <= 0;
        end else begin    
            if (axi_arready && s_axi_arvalid && !axi_rvalid) begin
                axi_rvalid <= 1'b1;
                axi_rresp  <= 2'b0; // 'OKAY' response
            end else if (axi_rvalid && s_axi_rready) begin
                axi_rvalid <= 1'b0;
            end                
        end
    end    

    // Implement memory mapped register select and read logic generation
    assign slv_reg_rden = axi_arready & s_axi_arvalid & !axi_rvalid;
    
    always @(*) begin
        // Address decoding for reading registers
        case (axi_araddr[ADDR_LSB+OPT_MEM_ADDR_BITS:ADDR_LSB])
            3'h0   : reg_data_out = frequency;  // frequency (from input)
            3'h1   : reg_data_out = vpp;        // vpp (from input)
            3'h2   : reg_data_out = vp;         // vp (from input)
            3'h3   : reg_data_out = slv_reg3;   // limits
            3'h4   : reg_data_out = slv_reg4;   // gain
            3'h5   : reg_data_out = slv_reg5;   // delay
            default: reg_data_out = 0;
        endcase
    end

    // Output register or memory read data
    always @(posedge s_axi_aclk) begin
        if (!s_axi_aresetn) begin
            axi_rdata  <= 0;
        end else begin    
            if (slv_reg_rden) begin
                axi_rdata <= reg_data_out;
            end   
        end
    end    

endmodule