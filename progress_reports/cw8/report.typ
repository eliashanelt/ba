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
// Example figure
#figure(
  image("res/img/fig1.png", width: 70%),
  caption: [
    Informal schematic oversimplified representation of a Fluid-FM and the device requirements
    which are to be designed in the bachelor thesis.
  ],
)

== Hardware Selection
+ Arduino Pro™ Portenta H7 REV2
+ Arduino Pro™ Portenta Max Carrier
+ Arduino Pro™ Portenta Breakout



== Technical Requirements

Some technical details here...


#lorem(1000)
