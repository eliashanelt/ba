#include "AnalogOut.h"
#include "hal/analogout_api.h"

////AnalogOut *dac;
dac_t myDac;

void setup() {
  // put your setup code here, to run once:
  const PinMap *pinMap;
  pinMap = analogout_pinmap();

  pinMode(LEDR,OUTPUT);
  pinMode(LEDG,OUTPUT);
  pinMode(LEDB,OUTPUT); 

  digitalWrite(LEDR, HIGH); //red LED OFF - wait for terminal: default 9600baud
  digitalWrite(LEDG, HIGH); //HIGH is OFF!
  digitalWrite(LEDB, HIGH);

  // Serial.begin(9600);
  while (!Serial) {
    ; // wait for serial port to connect. Needed for native USB port only
  }

  Serial.println("*** DAC demo setup ***");
  Serial.print("pin: 0x");
  Serial.println(pinMap->pin, HEX);

  ////dac = AnalogOut(pinMap->pin);
  analogout_init_direct(&myDac, pinMap);
  analogout_write(&myDac, 8192);
}

void loop() {
  //Serial.println("Loop | DAC demo:");
  //digitalWrite(LEDG, HIGH);
  //analogout_write(&myDac, 0);
  //delay(1000);
  //digitalWrite(LEDG, LOW);
  //uint16_t adcValue = analogout_read_u16(A0);
  //analogout_write(&myDac, adcValue);
  analogout_write(&myDac, 1024);
  //delay(1000);
  //analogout_write(&myDac, 2048);
  //delay(1000);
}