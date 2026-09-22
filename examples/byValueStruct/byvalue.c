#include <stdint.h>
#ifdef _WIN32
  #define CHILLFFI_EXPORT __declspec(dllexport)
#else
  #define CHILLFFI_EXPORT
#endif
struct Point { int32_t x; int32_t y; };
struct Big { double a; double b; double c; int32_t tag; };
struct Nested { struct Point origin; float scale; };
CHILLFFI_EXPORT int32_t point_sum(struct Point p) { return p.x + p.y; }
CHILLFFI_EXPORT struct Point point_translate(struct Point p, int32_t dx, int32_t dy) {
  struct Point out; out.x = p.x + dx; out.y = p.y + dy; return out;
}
CHILLFFI_EXPORT double big_sum(struct Big b) { return b.a + b.b + b.c + (double)b.tag; }
CHILLFFI_EXPORT struct Big big_scale(struct Big b, double k) {
  struct Big out; out.a = b.a * k; out.b = b.b * k; out.c = b.c * k; out.tag = b.tag; return out;
}
CHILLFFI_EXPORT float nested_scale(struct Nested n) {
  return n.scale * (float)(n.origin.x + n.origin.y);
}
CHILLFFI_EXPORT struct Nested nested_double(struct Nested n) {
  struct Nested out;
  out.origin.x = n.origin.x * 2; out.origin.y = n.origin.y * 2; out.scale = n.scale * 2.0f;
  return out;
}
