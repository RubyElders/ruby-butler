#include <ruby.h>

static VALUE answer(VALUE self) {
    return INT2NUM(42);
}

void Init_source_native(void) {
    VALUE fixture = rb_define_module("Fixture");
    rb_define_singleton_method(fixture, "answer", answer, 0);
}
