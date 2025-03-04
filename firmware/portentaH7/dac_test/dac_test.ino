#include "Arduino.h"
#include "hal/analogout_api.h"
#include "hal/analogin_api.h"

// Our objects
dac_t myDac;
analogin_t myAdc;

void setup() {

  // Convert the Arduino pin name (e.g. A0) to an Mbed pin name
  PinName dacPin = digitalPinToPinName(A6);  // Suppose your board says A0 is a DAC
  PinName adcPin = digitalPinToPinName(A1);  // Suppose A1 is a regular analog input

  // Initialize the DAC on A0
  analogout_init(&myDac, dacPin);

  // Initialize the ADC on A1
  analogin_init(&myAdc, adcPin);
}

void loop() {
  // Read a 16-bit value from the ADC
  //uint16_t adcValue = analogin_read_u16(&myAdc);
  float adcValue = analogin_read(&myAdc);
  // Write that value to the DAC
  analogout_write(&myDac, adcValue);
}