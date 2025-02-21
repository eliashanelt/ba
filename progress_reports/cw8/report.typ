// Set up page header/footer
#set page(
	margin: (inside: 2.5cm, outside: 2cm, y: 1.75cm),
	header: [
	  _Bachelor Thesis Progress Report CW8_
	  #h(1fr)
	  Elias Hanelt
	],
	footer: [
		#align(right, [#counter(page).display()])
  	]
)
#counter(page).update(1)
#v(1%)
= Progress Report CW8
#v(3%)

start of the bachelor thesis

working on improving LBB's self-excited FluidFM cantilever system


== What is a Fluid-FM


== Introduction to the Problem
Getting the Fluid-FM to resonate at its first order resonant frequency. When trying to excite the
cantilever that carries the oil droplet we currently face the issue that the cantilever does not
always resonate at its first order resonance frequency but sometimes enters different modes at
higher frequencies. Since the mass of the oil droplet at the tip of the cantilever differs in size
and density the resonance frequency is not always the same and differs from droplet to droplet and
thus from experiment to experiment

== Problem Statement
Why exactly is this a problem?

== Research Motivation
What would be the advantages if we solve these


== Current Approach

problems: 
+ higher resonance frequencies, using a low pass filter
+ unstable system, (goes to infinity or maximum) amplitude threshold filter 
+ excitement, the threshold filter introduces a new problem it prevents excitement of the
  oscillation in the beginning. Just applying a step function does not solve it





#figure(
  image("res/img/fig1.png", width: 70%),
  caption: [
    Informal schematic oversimplified representation of a Fluid-FM and the device requirements
    which are to be designed in the bachelor thesis.
  ],
)

== Towards a Solution: Developing a logical Filter


#figure(
  image("res/img/fig2.png", width: 70%),
  caption: [
    Informal schematic oversimplified representation of a Fluid-FM and the device requirements
    which are to be designed in the bachelor thesis.
  ],
)
== Technical requirements
high sampling rate, high resolution DAC, ADC, two cores, networking, coxial input
== Hardware Selection
+ Arduino Pro™ Portenta H7 REV2
+ Arduino Pro™ Portenta Max Carrier
+ Arduino Pro™ Portenta Breakout



== Issues
Altough programming with the Arduino IDE was possible the whole Arduino Ecosystems lacks a lot of
great features such as a debugger and uses their own version of C which i found to tedious to work
with.
Trying to program/flash the Portenta with rust seemed like a good idea at first
(advantages of rust embedded) so I wrote a simple program, but flashing the program seemed
impossible dfu-util and the amazon zyphr tutorial were helpful here

But since we need to design a pcb anyway to mount the coxial connector since the portenta master
carrier coxial connectors are only connected to the Lora chip and the gsm module.

On top of that both the max carrier board and even the portenta h7 come with a lot of components
we dont use.
So if we want to develop an actual useable solution for this problem it would be wise to create an
own pcb where we can place all and only the neccessary components


Since the stm32h7xx family is a great choice for our tasks since the adcs have quite a high
resolution at a high sampling rate. Additionally the two cores allow us to simultaneously run the
logic that transforms the signal the we get from the Fluid-FM and the networking and and interae to
the control unit and in the future maybe even an labview interface.

Making our own board would probably even cheaper, especially for people who want to use this device
as well


Also looking at jlcpcbs parts stock, we can see that assembly at their plant is feasible which
makes the whole process easier


== Test results
using the arduino protenta h7 with my oscilloscope
