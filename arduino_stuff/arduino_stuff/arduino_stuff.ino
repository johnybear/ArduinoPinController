

#include <SoftwareSerial.h>

void setup()
{
  int i;
  for(i=0; i < 9; i++){
    pinMode(i, OUTPUT);  
  }
  Serial.begin(9600);
}


void change(int arg, int stat) {
  if (stat==1) {  
    digitalWrite(arg, HIGH);
  } else {
    digitalWrite(arg, LOW);
  }
}

void loop() {
  char buff[2];
  int stat, pin;
  if (Serial.available()){
    Serial.readBytes(buff, 2);
    pin = buff[0];
    stat = buff[1];
    change(pin, stat);  
  }
  delay(200);
}


int get_num(int z){
  int i;
  return z-48;
}


