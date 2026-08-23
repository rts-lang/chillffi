#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

// Принимает сырые байты аргументов (склеенные подряд), 
// печатает их как строку и возвращает NULL.
uint8_t* print(const uint8_t* data, size_t len) 
{
  fwrite(data, 1, len, stdout);
  fflush(stdout);
  return NULL;
}