# Definitional proc to organize widgets for parameters.
proc init_gui { IPINST } {
  ipgui::add_param $IPINST -name "Component_Name"
  #Adding Page
  set Page_0 [ipgui::add_page $IPINST -name "Page 0"]
  ipgui::add_param $IPINST -name "ADC_WIDTH" -parent ${Page_0}
  ipgui::add_param $IPINST -name "AXIS_TDATA_WIDTH" -parent ${Page_0}
  ipgui::add_param $IPINST -name "DAC_WIDTH" -parent ${Page_0}
  ipgui::add_param $IPINST -name "GAIN_FRAC_BITS" -parent ${Page_0}
  ipgui::add_param $IPINST -name "GAIN_WIDTH" -parent ${Page_0}


}

proc update_PARAM_VALUE.ADC_WIDTH { PARAM_VALUE.ADC_WIDTH } {
	# Procedure called to update ADC_WIDTH when any of the dependent parameters in the arguments change
}

proc validate_PARAM_VALUE.ADC_WIDTH { PARAM_VALUE.ADC_WIDTH } {
	# Procedure called to validate ADC_WIDTH
	return true
}

proc update_PARAM_VALUE.AXIS_TDATA_WIDTH { PARAM_VALUE.AXIS_TDATA_WIDTH } {
	# Procedure called to update AXIS_TDATA_WIDTH when any of the dependent parameters in the arguments change
}

proc validate_PARAM_VALUE.AXIS_TDATA_WIDTH { PARAM_VALUE.AXIS_TDATA_WIDTH } {
	# Procedure called to validate AXIS_TDATA_WIDTH
	return true
}

proc update_PARAM_VALUE.DAC_WIDTH { PARAM_VALUE.DAC_WIDTH } {
	# Procedure called to update DAC_WIDTH when any of the dependent parameters in the arguments change
}

proc validate_PARAM_VALUE.DAC_WIDTH { PARAM_VALUE.DAC_WIDTH } {
	# Procedure called to validate DAC_WIDTH
	return true
}

proc update_PARAM_VALUE.GAIN_FRAC_BITS { PARAM_VALUE.GAIN_FRAC_BITS } {
	# Procedure called to update GAIN_FRAC_BITS when any of the dependent parameters in the arguments change
}

proc validate_PARAM_VALUE.GAIN_FRAC_BITS { PARAM_VALUE.GAIN_FRAC_BITS } {
	# Procedure called to validate GAIN_FRAC_BITS
	return true
}

proc update_PARAM_VALUE.GAIN_WIDTH { PARAM_VALUE.GAIN_WIDTH } {
	# Procedure called to update GAIN_WIDTH when any of the dependent parameters in the arguments change
}

proc validate_PARAM_VALUE.GAIN_WIDTH { PARAM_VALUE.GAIN_WIDTH } {
	# Procedure called to validate GAIN_WIDTH
	return true
}


proc update_MODELPARAM_VALUE.ADC_WIDTH { MODELPARAM_VALUE.ADC_WIDTH PARAM_VALUE.ADC_WIDTH } {
	# Procedure called to set VHDL generic/Verilog parameter value(s) based on TCL parameter value
	set_property value [get_property value ${PARAM_VALUE.ADC_WIDTH}] ${MODELPARAM_VALUE.ADC_WIDTH}
}

proc update_MODELPARAM_VALUE.DAC_WIDTH { MODELPARAM_VALUE.DAC_WIDTH PARAM_VALUE.DAC_WIDTH } {
	# Procedure called to set VHDL generic/Verilog parameter value(s) based on TCL parameter value
	set_property value [get_property value ${PARAM_VALUE.DAC_WIDTH}] ${MODELPARAM_VALUE.DAC_WIDTH}
}

proc update_MODELPARAM_VALUE.AXIS_TDATA_WIDTH { MODELPARAM_VALUE.AXIS_TDATA_WIDTH PARAM_VALUE.AXIS_TDATA_WIDTH } {
	# Procedure called to set VHDL generic/Verilog parameter value(s) based on TCL parameter value
	set_property value [get_property value ${PARAM_VALUE.AXIS_TDATA_WIDTH}] ${MODELPARAM_VALUE.AXIS_TDATA_WIDTH}
}

proc update_MODELPARAM_VALUE.GAIN_WIDTH { MODELPARAM_VALUE.GAIN_WIDTH PARAM_VALUE.GAIN_WIDTH } {
	# Procedure called to set VHDL generic/Verilog parameter value(s) based on TCL parameter value
	set_property value [get_property value ${PARAM_VALUE.GAIN_WIDTH}] ${MODELPARAM_VALUE.GAIN_WIDTH}
}

proc update_MODELPARAM_VALUE.GAIN_FRAC_BITS { MODELPARAM_VALUE.GAIN_FRAC_BITS PARAM_VALUE.GAIN_FRAC_BITS } {
	# Procedure called to set VHDL generic/Verilog parameter value(s) based on TCL parameter value
	set_property value [get_property value ${PARAM_VALUE.GAIN_FRAC_BITS}] ${MODELPARAM_VALUE.GAIN_FRAC_BITS}
}

