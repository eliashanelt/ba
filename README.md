# Bachelor Thesis: Enhancing the Self-Excited FluidFM Cantilever System
This is the official repo of my bachelor thesis. I will be working on improving LBB's self-excited FluidFM cantilever system
#### Where to find me?
_ETZ E 60.1_

#### Important Links
[nas setup](https://wiki.lbb.ethz.ch/it/backup)

[lbb wiki](https://people.ee.ethz.ch/~lbbwiki/doku.php?id=start)


#### How to flash fpga
open firmware/fpga/tmp/freq/freq.xpr with vivado
generate bitstream file with 'Generate Bitstream'

in firmware/fpga/tmp/freq/freq.runs/impl_1 create system_wrapper.bif containing:
all: { system_wrapper.bit }

also navigate there with the tcl shell and run
bootgen -image system_wrapper.bif -arch zynq -process_bitstream bin -o system_wrapper.bit.bin -w

after that run with the normal shell in that folder:
scp system_wrapper.bit.bin root@rp-f0bbb6.local:/root/

ssh root@rp-f0bbb6.local

password: root

on the redpitaya then run:
fpgautil - b system_wrapper.bit.bin

