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

== Hardware Selection
+ Arduino Pro™ Portenta H7 REV2
+ Arduino Pro™ Portenta Max Carrier
+ Arduino Pro™ Portenta Breakout



== Technical Requirements

Some technical details here...


