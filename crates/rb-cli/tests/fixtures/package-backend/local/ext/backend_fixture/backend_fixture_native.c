#include <ruby.h>

RUBY_FUNC_EXPORTED void Init_backend_fixture_native(void) {
    rb_define_module("BackendFixtureNative");
}
