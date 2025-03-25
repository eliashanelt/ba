#include "Arduino.h"
#include "hal/analogout_api.h"
#include "hal/analogin_api.h"

dac_t myDac;
analogin_t myAdc;

void setup() {
  PinName dacPin = digitalPinToPinName(A6);
  PinName adcPin = digitalPinToPinName(A1);

  // Initialize DAC and ADC
  analogout_init(&myDac, dacPin);
  analogin_init(&myAdc, adcPin);
}

void loop() {
  // Read the ADC as a 16-bit integer
  uint16_t adcValue = analogin_read_u16(&myAdc);
  
  // Write the 16-bit value directly to the DAC
  analogout_write_u16(&myDac, adcValue);
}