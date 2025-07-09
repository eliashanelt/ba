vlib modelsim_lib/work
vlib modelsim_lib/msim

vlib modelsim_lib/msim/xilinx_vip
vlib modelsim_lib/msim/xpm
vlib modelsim_lib/msim/axi_infrastructure_v1_1_0
vlib modelsim_lib/msim/axi_vip_v1_1_7
vlib modelsim_lib/msim/processing_system7_vip_v1_0_9
vlib modelsim_lib/msim/xil_defaultlib
vlib modelsim_lib/msim/axi_lite_ipif_v3_0_4
vlib modelsim_lib/msim/lib_cdc_v1_0_2
vlib modelsim_lib/msim/interrupt_control_v3_1_4
vlib modelsim_lib/msim/axi_gpio_v2_0_23
vlib modelsim_lib/msim/proc_sys_reset_v5_0_13
vlib modelsim_lib/msim/xlconstant_v1_1_7
vlib modelsim_lib/msim/xlslice_v1_0_2
vlib modelsim_lib/msim/generic_baseblocks_v2_1_0
vlib modelsim_lib/msim/fifo_generator_v13_2_5
vlib modelsim_lib/msim/axi_data_fifo_v2_1_20
vlib modelsim_lib/msim/axi_register_slice_v2_1_21
vlib modelsim_lib/msim/axi_protocol_converter_v2_1_21

vmap xilinx_vip modelsim_lib/msim/xilinx_vip
vmap xpm modelsim_lib/msim/xpm
vmap axi_infrastructure_v1_1_0 modelsim_lib/msim/axi_infrastructure_v1_1_0
vmap axi_vip_v1_1_7 modelsim_lib/msim/axi_vip_v1_1_7
vmap processing_system7_vip_v1_0_9 modelsim_lib/msim/processing_system7_vip_v1_0_9
vmap xil_defaultlib modelsim_lib/msim/xil_defaultlib
vmap axi_lite_ipif_v3_0_4 modelsim_lib/msim/axi_lite_ipif_v3_0_4
vmap lib_cdc_v1_0_2 modelsim_lib/msim/lib_cdc_v1_0_2
vmap interrupt_control_v3_1_4 modelsim_lib/msim/interrupt_control_v3_1_4
vmap axi_gpio_v2_0_23 modelsim_lib/msim/axi_gpio_v2_0_23
vmap proc_sys_reset_v5_0_13 modelsim_lib/msim/proc_sys_reset_v5_0_13
vmap xlconstant_v1_1_7 modelsim_lib/msim/xlconstant_v1_1_7
vmap xlslice_v1_0_2 modelsim_lib/msim/xlslice_v1_0_2
vmap generic_baseblocks_v2_1_0 modelsim_lib/msim/generic_baseblocks_v2_1_0
vmap fifo_generator_v13_2_5 modelsim_lib/msim/fifo_generator_v13_2_5
vmap axi_data_fifo_v2_1_20 modelsim_lib/msim/axi_data_fifo_v2_1_20
vmap axi_register_slice_v2_1_21 modelsim_lib/msim/axi_register_slice_v2_1_21
vmap axi_protocol_converter_v2_1_21 modelsim_lib/msim/axi_protocol_converter_v2_1_21

vlog -work xilinx_vip  -incr -sv -L axi_vip_v1_1_7 -L processing_system7_vip_v1_0_9 -L xilinx_vip "+incdir+C:/Xilinx/Vivado/2020.1/data/xilinx_vip/include" \
"C:/Xilinx/Vivado/2020.1/data/xilinx_vip/hdl/axi4stream_vip_axi4streampc.sv" \
"C:/Xilinx/Vivado/2020.1/data/xilinx_vip/hdl/axi_vip_axi4pc.sv" \
"C:/Xilinx/Vivado/2020.1/data/xilinx_vip/hdl/xil_common_vip_pkg.sv" \
"C:/Xilinx/Vivado/2020.1/data/xilinx_vip/hdl/axi4stream_vip_pkg.sv" \
"C:/Xilinx/Vivado/2020.1/data/xilinx_vip/hdl/axi_vip_pkg.sv" \
"C:/Xilinx/Vivado/2020.1/data/xilinx_vip/hdl/axi4stream_vip_if.sv" \
"C:/Xilinx/Vivado/2020.1/data/xilinx_vip/hdl/axi_vip_if.sv" \
"C:/Xilinx/Vivado/2020.1/data/xilinx_vip/hdl/clk_vip_if.sv" \
"C:/Xilinx/Vivado/2020.1/data/xilinx_vip/hdl/rst_vip_if.sv" \

vlog -work xpm  -incr -sv -L axi_vip_v1_1_7 -L processing_system7_vip_v1_0_9 -L xilinx_vip "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/ec67/hdl" "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/6b56/hdl" "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/8b3d" "+incdir+C:/Xilinx/Vivado/2020.1/data/xilinx_vip/include" \
"C:/Xilinx/Vivado/2020.1/data/ip/xpm/xpm_cdc/hdl/xpm_cdc.sv" \
"C:/Xilinx/Vivado/2020.1/data/ip/xpm/xpm_memory/hdl/xpm_memory.sv" \

vcom -work xpm  -93 \
"C:/Xilinx/Vivado/2020.1/data/ip/xpm/xpm_VCOMP.vhd" \

vlog -work axi_infrastructure_v1_1_0  -incr "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/ec67/hdl" "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/6b56/hdl" "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/8b3d" "+incdir+C:/Xilinx/Vivado/2020.1/data/xilinx_vip/include" \
"../../../../freq.srcs/sources_1/bd/system/ipshared/ec67/hdl/axi_infrastructure_v1_1_vl_rfs.v" \

vlog -work axi_vip_v1_1_7  -incr -sv -L axi_vip_v1_1_7 -L processing_system7_vip_v1_0_9 -L xilinx_vip "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/ec67/hdl" "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/6b56/hdl" "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/8b3d" "+incdir+C:/Xilinx/Vivado/2020.1/data/xilinx_vip/include" \
"../../../../freq.srcs/sources_1/bd/system/ipshared/ce6c/hdl/axi_vip_v1_1_vl_rfs.sv" \

vlog -work processing_system7_vip_v1_0_9  -incr -sv -L axi_vip_v1_1_7 -L processing_system7_vip_v1_0_9 -L xilinx_vip "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/ec67/hdl" "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/6b56/hdl" "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/8b3d" "+incdir+C:/Xilinx/Vivado/2020.1/data/xilinx_vip/include" \
"../../../../freq.srcs/sources_1/bd/system/ipshared/6b56/hdl/processing_system7_vip_v1_0_vl_rfs.sv" \

vlog -work xil_defaultlib  -incr "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/ec67/hdl" "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/6b56/hdl" "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/8b3d" "+incdir+C:/Xilinx/Vivado/2020.1/data/xilinx_vip/include" \
"../../../bd/system/ip/system_processing_system7_0_0/sim/system_processing_system7_0_0.v" \

vcom -work xil_defaultlib  -93 \
"../../../bd/system/ip/system_util_ds_buf_1_0/util_ds_buf.vhd" \
"../../../bd/system/ip/system_util_ds_buf_1_0/sim/system_util_ds_buf_1_0.vhd" \
"../../../bd/system/ip/system_util_ds_buf_2_0/sim/system_util_ds_buf_2_0.vhd" \

vcom -work axi_lite_ipif_v3_0_4  -93 \
"../../../../freq.srcs/sources_1/bd/system/ipshared/66ea/hdl/axi_lite_ipif_v3_0_vh_rfs.vhd" \

vcom -work lib_cdc_v1_0_2  -93 \
"../../../../freq.srcs/sources_1/bd/system/ipshared/ef1e/hdl/lib_cdc_v1_0_rfs.vhd" \

vcom -work interrupt_control_v3_1_4  -93 \
"../../../../freq.srcs/sources_1/bd/system/ipshared/a040/hdl/interrupt_control_v3_1_vh_rfs.vhd" \

vcom -work axi_gpio_v2_0_23  -93 \
"../../../../freq.srcs/sources_1/bd/system/ipshared/bb35/hdl/axi_gpio_v2_0_vh_rfs.vhd" \

vcom -work xil_defaultlib  -93 \
"../../../bd/system/ip/system_axi_gpio_0_0/sim/system_axi_gpio_0_0.vhd" \

vlog -work xil_defaultlib  -incr "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/ec67/hdl" "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/6b56/hdl" "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/8b3d" "+incdir+C:/Xilinx/Vivado/2020.1/data/xilinx_vip/include" \
"../../../bd/system/ipshared/d32c/src/axis_red_pitaya_adc.v" \
"../../../bd/system/ip/system_axis_red_pitaya_adc_0_0/sim/system_axis_red_pitaya_adc_0_0.v" \

vcom -work proc_sys_reset_v5_0_13  -93 \
"../../../../freq.srcs/sources_1/bd/system/ipshared/8842/hdl/proc_sys_reset_v5_0_vh_rfs.vhd" \

vcom -work xil_defaultlib  -93 \
"../../../bd/system/ip/system_rst_ps7_0_125M_0/sim/system_rst_ps7_0_125M_0.vhd" \

vlog -work xil_defaultlib  -incr "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/ec67/hdl" "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/6b56/hdl" "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/8b3d" "+incdir+C:/Xilinx/Vivado/2020.1/data/xilinx_vip/include" \
"../../../bd/system/ip/system_signal_split_0_0/sim/system_signal_split_0_0.v" \
"../../../bd/system/ip/system_frequency_counter_0_0/sim/system_frequency_counter_0_0.v" \

vlog -work xlconstant_v1_1_7  -incr "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/ec67/hdl" "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/6b56/hdl" "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/8b3d" "+incdir+C:/Xilinx/Vivado/2020.1/data/xilinx_vip/include" \
"../../../../freq.srcs/sources_1/bd/system/ipshared/fcfc/hdl/xlconstant_v1_1_vl_rfs.v" \

vlog -work xil_defaultlib  -incr "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/ec67/hdl" "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/6b56/hdl" "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/8b3d" "+incdir+C:/Xilinx/Vivado/2020.1/data/xilinx_vip/include" \
"../../../bd/system/ip/system_xlconstant_0_0/sim/system_xlconstant_0_0.v" \
"../../../bd/system/ip/system_pow2_0_0/sim/system_pow2_0_0.v" \

vlog -work xlslice_v1_0_2  -incr "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/ec67/hdl" "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/6b56/hdl" "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/8b3d" "+incdir+C:/Xilinx/Vivado/2020.1/data/xilinx_vip/include" \
"../../../../freq.srcs/sources_1/bd/system/ipshared/11d0/hdl/xlslice_v1_0_vl_rfs.v" \

vlog -work xil_defaultlib  -incr "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/ec67/hdl" "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/6b56/hdl" "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/8b3d" "+incdir+C:/Xilinx/Vivado/2020.1/data/xilinx_vip/include" \
"../../../bd/system/ip/system_xlslice_0_1/sim/system_xlslice_0_1.v" \
"../../../bd/system/ipshared/8b85/src/axis_red_pitaya_dac.v" \
"../../../bd/system/ip/system_axis_red_pitaya_dac_0_1/sim/system_axis_red_pitaya_dac_0_1.v" \
"../../../bd/system/ip/system_clk_wiz_0_1/system_clk_wiz_0_1_clk_wiz.v" \
"../../../bd/system/ip/system_clk_wiz_0_1/system_clk_wiz_0_1.v" \
"../../../bd/system/ip/system_signal_merge_0_0/sim/system_signal_merge_0_0.v" \
"../../../bd/system/ip/system_freq_mapper_0_0/sim/system_freq_mapper_0_0.v" \
"../../../bd/system/ip/system_xlconstant_0_2/sim/system_xlconstant_0_2.v" \
"../../../bd/system/ip/system_xlconstant_0_3/sim/system_xlconstant_0_3.v" \

vlog -work generic_baseblocks_v2_1_0  -incr "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/ec67/hdl" "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/6b56/hdl" "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/8b3d" "+incdir+C:/Xilinx/Vivado/2020.1/data/xilinx_vip/include" \
"../../../../freq.srcs/sources_1/bd/system/ipshared/b752/hdl/generic_baseblocks_v2_1_vl_rfs.v" \

vlog -work fifo_generator_v13_2_5  -incr "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/ec67/hdl" "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/6b56/hdl" "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/8b3d" "+incdir+C:/Xilinx/Vivado/2020.1/data/xilinx_vip/include" \
"../../../../freq.srcs/sources_1/bd/system/ipshared/276e/simulation/fifo_generator_vlog_beh.v" \

vcom -work fifo_generator_v13_2_5  -93 \
"../../../../freq.srcs/sources_1/bd/system/ipshared/276e/hdl/fifo_generator_v13_2_rfs.vhd" \

vlog -work fifo_generator_v13_2_5  -incr "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/ec67/hdl" "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/6b56/hdl" "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/8b3d" "+incdir+C:/Xilinx/Vivado/2020.1/data/xilinx_vip/include" \
"../../../../freq.srcs/sources_1/bd/system/ipshared/276e/hdl/fifo_generator_v13_2_rfs.v" \

vlog -work axi_data_fifo_v2_1_20  -incr "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/ec67/hdl" "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/6b56/hdl" "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/8b3d" "+incdir+C:/Xilinx/Vivado/2020.1/data/xilinx_vip/include" \
"../../../../freq.srcs/sources_1/bd/system/ipshared/47c9/hdl/axi_data_fifo_v2_1_vl_rfs.v" \

vlog -work axi_register_slice_v2_1_21  -incr "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/ec67/hdl" "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/6b56/hdl" "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/8b3d" "+incdir+C:/Xilinx/Vivado/2020.1/data/xilinx_vip/include" \
"../../../../freq.srcs/sources_1/bd/system/ipshared/2ef9/hdl/axi_register_slice_v2_1_vl_rfs.v" \

vlog -work axi_protocol_converter_v2_1_21  -incr "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/ec67/hdl" "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/6b56/hdl" "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/8b3d" "+incdir+C:/Xilinx/Vivado/2020.1/data/xilinx_vip/include" \
"../../../../freq.srcs/sources_1/bd/system/ipshared/8dfa/hdl/axi_protocol_converter_v2_1_vl_rfs.v" \

vlog -work xil_defaultlib  -incr "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/ec67/hdl" "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/6b56/hdl" "+incdir+../../../../freq.srcs/sources_1/bd/system/ipshared/8b3d" "+incdir+C:/Xilinx/Vivado/2020.1/data/xilinx_vip/include" \
"../../../bd/system/ip/system_auto_pc_0/sim/system_auto_pc_0.v" \
"../../../bd/system/sim/system.v" \

vlog -work xil_defaultlib \
"glbl.v"

