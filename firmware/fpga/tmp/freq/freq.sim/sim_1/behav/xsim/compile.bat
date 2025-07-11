@echo off
REM ****************************************************************************
REM Vivado (TM) v2020.1 (64-bit)
REM
REM Filename    : compile.bat
REM Simulator   : Xilinx Vivado Simulator
REM Description : Script for compiling the simulation design source files
REM
REM Generated by Vivado on Mon Jun 30 18:08:14 +0200 2025
REM SW Build 2902540 on Wed May 27 19:54:49 MDT 2020
REM
REM Copyright 1986-2020 Xilinx, Inc. All Rights Reserved.
REM
REM usage: compile.bat
REM
REM ****************************************************************************
echo "xvlog --incr --relax -L axi_vip_v1_1_7 -L processing_system7_vip_v1_0_9 -L xilinx_vip -prj system_wrapper_vlog.prj"
call xvlog  --incr --relax -L axi_vip_v1_1_7 -L processing_system7_vip_v1_0_9 -L xilinx_vip -prj system_wrapper_vlog.prj -log xvlog.log
call type xvlog.log > compile.log
echo "xvhdl --incr --relax -prj system_wrapper_vhdl.prj"
call xvhdl  --incr --relax -prj system_wrapper_vhdl.prj -log xvhdl.log
call type xvhdl.log >> compile.log
if "%errorlevel%"=="1" goto END
if "%errorlevel%"=="0" goto SUCCESS
:END
exit 1
:SUCCESS
exit 0
