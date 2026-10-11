#include <ruby.h>

static VALUE answer(VALUE self) {
    return INT2NUM(42);
}

void Init_butler_native(void) {
    VALUE module = rb_define_module("ButlerNative");
    rb_define_singleton_method(module, "answer", answer, 0);
}
