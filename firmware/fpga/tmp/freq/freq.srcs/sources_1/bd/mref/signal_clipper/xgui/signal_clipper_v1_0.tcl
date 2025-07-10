# Definitional proc to organize widgets for parameters.
proc init_gui { IPINST } {
  ipgui::add_param $IPINST -name "Component_Name"
  #Adding Page
  set Page_0 [ipgui::add_page $IPINST -name "Page 0"]
  ipgui::add_param $IPINST -name "AXIS_TDATA_WIDTH" -parent ${Page_0}
  ipgui::add_param $IPINST -name "DATA_WIDTH" -parent ${Page_0}
  ipgui::add_param $IPINST -name "DEFAULT_MAX_LIMIT" -parent ${Page_0}
  ipgui::add_param $IPINST -name "DEFAULT_MIN_LIMIT" -parent ${Page_0}


}

proc update_PARAM_VALUE.AXIS_TDATA_WIDTH { PARAM_VALUE.AXIS_TDATA_WIDTH } {
	# Procedure called to update AXIS_TDATA_WIDTH when any of the dependent parameters in the arguments change
}

proc validate_PARAM_VALUE.AXIS_TDATA_WIDTH { PARAM_VALUE.AXIS_TDATA_WIDTH } {
	# Procedure called to validate AXIS_TDATA_WIDTH
	return true
}

proc update_PARAM_VALUE.DATA_WIDTH { PARAM_VALUE.DATA_WIDTH } {
	# Procedure called to update DATA_WIDTH when any of the dependent parameters in the arguments change
}

proc validate_PARAM_VALUE.DATA_WIDTH { PARAM_VALUE.DATA_WIDTH } {
	# Procedure called to validate DATA_WIDTH
	return true
}

proc update_PARAM_VALUE.DEFAULT_MAX_LIMIT { PARAM_VALUE.DEFAULT_MAX_LIMIT } {
	# Procedure called to update DEFAULT_MAX_LIMIT when any of the dependent parameters in the arguments change
}

proc validate_PARAM_VALUE.DEFAULT_MAX_LIMIT { PARAM_VALUE.DEFAULT_MAX_LIMIT } {
	# Procedure called to validate DEFAULT_MAX_LIMIT
	return true
}

proc update_PARAM_VALUE.DEFAULT_MIN_LIMIT { PARAM_VALUE.DEFAULT_MIN_LIMIT } {
	# Procedure called to update DEFAULT_MIN_LIMIT when any of the dependent parameters in the arguments change
}

proc validate_PARAM_VALUE.DEFAULT_MIN_LIMIT { PARAM_VALUE.DEFAULT_MIN_LIMIT } {
	# Procedure called to validate DEFAULT_MIN_LIMIT
	return true
}


proc update_MODELPARAM_VALUE.DATA_WIDTH { MODELPARAM_VALUE.DATA_WIDTH PARAM_VALUE.DATA_WIDTH } {
	# Procedure called to set VHDL generic/Verilog parameter value(s) based on TCL parameter value
	set_property value [get_property value ${PARAM_VALUE.DATA_WIDTH}] ${MODELPARAM_VALUE.DATA_WIDTH}
}

proc update_MODELPARAM_VALUE.AXIS_TDATA_WIDTH { MODELPARAM_VALUE.AXIS_TDATA_WIDTH PARAM_VALUE.AXIS_TDATA_WIDTH } {
	# Procedure called to set VHDL generic/Verilog parameter value(s) based on TCL parameter value
	set_property value [get_property value ${PARAM_VALUE.AXIS_TDATA_WIDTH}] ${MODELPARAM_VALUE.AXIS_TDATA_WIDTH}
}

proc update_MODELPARAM_VALUE.DEFAULT_MAX_LIMIT { MODELPARAM_VALUE.DEFAULT_MAX_LIMIT PARAM_VALUE.DEFAULT_MAX_LIMIT } {
	# Procedure called to set VHDL generic/Verilog parameter value(s) based on TCL parameter value
	set_property value [get_property value ${PARAM_VALUE.DEFAULT_MAX_LIMIT}] ${MODELPARAM_VALUE.DEFAULT_MAX_LIMIT}
}

proc update_MODELPARAM_VALUE.DEFAULT_MIN_LIMIT { MODELPARAM_VALUE.DEFAULT_MIN_LIMIT PARAM_VALUE.DEFAULT_MIN_LIMIT } {
	# Procedure called to set VHDL generic/Verilog parameter value(s) based on TCL parameter value
	set_property value [get_property value ${PARAM_VALUE.DEFAULT_MIN_LIMIT}] ${MODELPARAM_VALUE.DEFAULT_MIN_LIMIT}
}

