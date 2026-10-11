#include <ruby.h>
#include <array>

static_assert(__cplusplus >= 201703L, "C++17 is required");

static VALUE answer(VALUE self) {
    constexpr std::array<int, 2> values{20, 22};
    return INT2NUM(values[0] + values[1]);
}

extern "C" void Init_butler_cpp(void) {
    VALUE module = rb_define_module("ButlerCpp");
    rb_define_singleton_method(module, "answer", answer, 0);
}
