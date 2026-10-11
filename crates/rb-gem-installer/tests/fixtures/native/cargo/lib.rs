use std::ffi::c_char;

extern "C" {
    fn rb_define_module(name: *const c_char) -> usize;
}

#[no_mangle]
pub unsafe extern "C" fn Init_butler_rust() {
    rb_define_module(b"ButlerRust\0".as_ptr().cast());
}
