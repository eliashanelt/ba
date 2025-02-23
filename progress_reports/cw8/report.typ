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
= Progress Report CW8

The advancement of micro- and nanoscale force sensing technologies has opened new possibilities for precision measurements in biophysics, material science, and nanotechnology. One such innovation is the self-excited FluidFM cantilever system, which combines microfluidic control with atomic force microscopy (AFM) principles. The Laboratory of Biosensors and Bioelectronics (LBB) has been developing this system to enhance stability, sensitivity, and automation in force measurements.

This thesis focuses on improving the self-excited FluidFM cantilever system by optimizing its excitation and control mechanisms. The objective is to refine its performance, accuracy, and potential applications in dynamic force sensing. By addressing key challenges such as signal stability, resonance behavior, and feedback control, this work aims to contribute to the broader goal of enhancing real-time force measurements in complex environments.


== Introduction to Fluidic Force Microscopy (FluidFM)

Fluidic Force Microscopy (FluidFM) is an advanced technology that combines the precision force measurement capabilities of atomic force microscopy (AFM) with microfluidics. In FluidFM, a hollow cantilever with an integrated microchannel enables the direct handling and manipulation of fluids at the microscale. This integration allows for applications such as single-cell injection, nano-printing of biomolecules, and localized substance delivery or extraction in a controlled manner.

== Advantages of FluidFM

- **Precise Force Control**  
  FluidFM retains the force measurement functionality of AFM, enabling high-precision force spectroscopy. This is crucial for studying cell mechanics, adhesion properties, and other biomechanical interactions at the single-cell or even subcellular level.

- **Versatile Fluid Handling**  
  The microchannel inside the cantilever allows for the controlled injection or aspiration of minute fluid volumes. Researchers can deliver biomolecules, drugs, or other solutions directly into cells or tissues, making it invaluable for targeted experimental studies.

- **Reduced Sample Damage**  
  Compared to conventional force microscopy or other micromanipulation techniques, FluidFM can apply lower forces and smaller volumes of liquid. This decreases the risk of damaging sensitive samples, such as living cells or delicate structures.

- **Broad Application Range**  
  The technology is used in various fields, including cell biology, materials science, and biosensor development. Its unique combination of force measurement and fluid handling paves the way for innovative research methods, such as intracellular measurements, cell sorting, and 3D bioprinting.

In summary, FluidFM offers a powerful and versatile platform for both researchers and industrial applications. By integrating microfluidic channels with force-sensing cantilevers, scientists can precisely manipulate minute volumes of liquid and measure microscale forces, providing valuable insights into biological, chemical, and materials processes.


== Introduction to the Problem
Getting the Fluid-FM to resonate at its first order resonant frequency. When trying to excite the
cantilever that carries the oil droplet we currently face the issue that the cantilever does not
always resonate at its first order resonance frequency but sometimes enters different modes at
higher frequencies. Since the mass of the oil droplet at the tip of the cantilever differs in size
and density the resonance frequency is not always the same and differs from droplet to droplet and
thus from experiment to experiment

== Problem Statement
While FluidFM (Fluidic Force Microscopy) offers many advantages, it also has some challenges and limitations, especially when dealing with different oil drop sizes in microfluidic and nanofluidic applications.

Difficult Droplet Formation: Generating consistent oil droplet sizes can be challenging due to:
Surface tension effects.
Variability in fluidic pressure control.
Differences in substrate interaction.
Size Variation: Even slight variations in pressure, temperature, or fluid viscosity can cause unexpected differences in droplet size.
Breakup & Coalescence: Oil droplets can merge or break apart unintentionally, leading to uncontrolled dispersion.
This is all results in a different resonance frequency and different swing modes


== Research Motivation
FluidFM is a powerful tool for nanotechnology, biophysics, and single-cell biology. Its ability to manipulate cells non-invasively, perform highly controlled liquid dispensing, and conduct force spectroscopy makes it superior to traditional AFM in many biomedical applications.

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
digital (logical) filters provide higher accuracy, flexibility, and stability, making them the preferred choice in most modern applications, especially where precision and adaptability are required.

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
= Proposed Section for Bachelor Thesis

One of the initial ideas for prototyping involved using the Arduino Portenta H7 board. Although this board can be programmed with the Arduino IDE, it poses several challenges for professional embedded development. First, the Arduino ecosystem lacks a robust debugging interface, making it difficult to perform efficient troubleshooting during the development phase. Moreover, Arduino uses its own variant of C/C++, which can be tedious to work with when implementing more advanced features.

An attempt to program and flash the Portenta H7 using Rust highlighted further limitations. While Rust is rapidly growing in the embedded domain and offers advantages in terms of memory safety, performance, and tooling, uploading code to the Portenta proved cumbersome. Tools such as `dfu-util` and tutorials (e.g., those provided by Amazon Zephyr) offered partial guidance, but stable and straightforward Rust support remained elusive on the Portenta platform.

== Limitations of Off-the-Shelf Carrier Boards

The Portenta family also offers carrier boards with various connectors (including coaxial connectors). However, these are often tied to specific modules—such as LoRa or GSM modules—not necessarily aligned with the custom requirements of this project. Additionally, the Portenta Max Carrier board, as well as the Portenta H7 itself, come with numerous components that are superfluous for a highly specialized application. This increases the overall cost and complicates the hardware setup, as most of these features remain unused.

== Rationale for a Custom Board

Given the challenges encountered with the Portenta ecosystem and the need for a tailored hardware solution, designing a custom printed circuit board (PCB) emerges as the most viable strategy. The key advantages of a custom PCB include:

- **Control Over Hardware Design**  
  By selecting only the necessary components for signal filtering, coaxial input, and data processing, the board can be optimized for size, cost, and noise performance. Unused features—common in many development boards—are avoided.

- **Improved Debugging and Development Environment**  
  A custom board allows for seamless integration of an external debugger and supports a broader range of software tools. Developers can choose environments such as Rust or standard C/C++ with full debugging capabilities, a critical requirement for reliable and efficient embedded systems development.

- **Optimized Microcontroller Selection**  
  The STM32H747XIHx microcontroller offers promising features, particularly for this application. Its dual-core architecture allows one core to handle peripheral communication (e.g., controlling fluidic systems, networking, LabVIEW interfacing) while the other core processes and filters the incoming signal data. Additionally, this family of MCUs boasts high-resolution, high-speed ADCs that fit the project’s need for accurate and fast signal sampling.

- **Cost-Effectiveness at Scale**  
  For professional or commercial applications, the per-unit cost of custom boards often becomes more attractive compared to using feature-heavy development boards. By sourcing components through manufacturers like JLCPCB, assembly can be consolidated into a single service, simplifying logistics and reducing potential errors in manual assembly.

== Implementation Plan

1. **Initial Prototyping**  
   Short-term testing and validation of essential concepts can still be performed using the Portenta H7 on a simple breadboard. This step confirms software viability and basic signal acquisition before committing to a final PCB design.

2. **PCB Design**  
   Once the feasibility has been validated, the custom board layout will be designed to incorporate the STM32H747XIHx MCU, necessary power regulation, and coaxial connectors for clean signal inputs. Specialized debugging interfaces (e.g., SWD or JTAG) will also be included.

3. **Manufacturing and Assembly**  
   The board design files will be sent to a manufacturer (e.g., JLCPCB), which provides both PCB fabrication and component assembly services. This ensures a streamlined supply chain and consistent quality.

4. **Software Development and Integration  
   Leveraging the Rust ecosystem—or standard C/C++ with a professional IDE—will offer a more robust development workflow, with advanced features such as breakpoints, real-time debugging, and better resource management.

In conclusion, while the Arduino Portenta H7 can serve as a convenient development platform for preliminary tests, its limitations in debugging, the complexity of customizing it for Rust, and extraneous hardware components make it less suitable for this project’s professional and specialized requirements. A custom STM32H747XIHx-based PCB addresses these drawbacks by providing a tailored, cost-effective, and high-performance solution.

== Test results
using the arduino protenta h7 with my oscilloscope
