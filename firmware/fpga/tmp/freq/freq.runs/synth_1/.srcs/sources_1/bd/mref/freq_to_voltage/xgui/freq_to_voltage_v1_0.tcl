# Definitional proc to organize widgets for parameters.
proc init_gui { IPINST } {
  ipgui::add_param $IPINST -name "Component_Name"
  #Adding Page
  set Page_0 [ipgui::add_page $IPINST -name "Page 0"]
  ipgui::add_param $IPINST -name "DAC_WIDTH" -parent ${Page_0}
  ipgui::add_param $IPINST -name "FREQ_WIDTH" -parent ${Page_0}
  ipgui::add_param $IPINST -name "MAX_FREQ" -parent ${Page_0}
  ipgui::add_param $IPINST -name "MIN_FREQ" -parent ${Page_0}


}

proc update_PARAM_VALUE.DAC_WIDTH { PARAM_VALUE.DAC_WIDTH } {
	# Procedure called to update DAC_WIDTH when any of the dependent parameters in the arguments change
}

proc validate_PARAM_VALUE.DAC_WIDTH { PARAM_VALUE.DAC_WIDTH } {
	# Procedure called to validate DAC_WIDTH
	return true
}

proc update_PARAM_VALUE.FREQ_WIDTH { PARAM_VALUE.FREQ_WIDTH } {
	# Procedure called to update FREQ_WIDTH when any of the dependent parameters in the arguments change
}

proc validate_PARAM_VALUE.FREQ_WIDTH { PARAM_VALUE.FREQ_WIDTH } {
	# Procedure called to validate FREQ_WIDTH
	return true
}

proc update_PARAM_VALUE.MAX_FREQ { PARAM_VALUE.MAX_FREQ } {
	# Procedure called to update MAX_FREQ when any of the dependent parameters in the arguments change
}

proc validate_PARAM_VALUE.MAX_FREQ { PARAM_VALUE.MAX_FREQ } {
	# Procedure called to validate MAX_FREQ
	return true
}

proc update_PARAM_VALUE.MIN_FREQ { PARAM_VALUE.MIN_FREQ } {
	# Procedure called to update MIN_FREQ when any of the dependent parameters in the arguments change
}

proc validate_PARAM_VALUE.MIN_FREQ { PARAM_VALUE.MIN_FREQ } {
	# Procedure called to validate MIN_FREQ
	return true
}


proc update_MODELPARAM_VALUE.FREQ_WIDTH { MODELPARAM_VALUE.FREQ_WIDTH PARAM_VALUE.FREQ_WIDTH } {
	# Procedure called to set VHDL generic/Verilog parameter value(s) based on TCL parameter value
	set_property value [get_property value ${PARAM_VALUE.FREQ_WIDTH}] ${MODELPARAM_VALUE.FREQ_WIDTH}
}

proc update_MODELPARAM_VALUE.DAC_WIDTH { MODELPARAM_VALUE.DAC_WIDTH PARAM_VALUE.DAC_WIDTH } {
	# Procedure called to set VHDL generic/Verilog parameter value(s) based on TCL parameter value
	set_property value [get_property value ${PARAM_VALUE.DAC_WIDTH}] ${MODELPARAM_VALUE.DAC_WIDTH}
}

proc update_MODELPARAM_VALUE.MAX_FREQ { MODELPARAM_VALUE.MAX_FREQ PARAM_VALUE.MAX_FREQ } {
	# Procedure called to set VHDL generic/Verilog parameter value(s) based on TCL parameter value
	set_property value [get_property value ${PARAM_VALUE.MAX_FREQ}] ${MODELPARAM_VALUE.MAX_FREQ}
}

proc update_MODELPARAM_VALUE.MIN_FREQ { MODELPARAM_VALUE.MIN_FREQ PARAM_VALUE.MIN_FREQ } {
	# Procedure called to set VHDL generic/Verilog parameter value(s) based on TCL parameter value
	set_property value [get_property value ${PARAM_VALUE.MIN_FREQ}] ${MODELPARAM_VALUE.MIN_FREQ}
}

