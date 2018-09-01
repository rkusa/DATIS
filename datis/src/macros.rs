macro_rules! cstr {
    ($s:expr) => {
        ::std::ffi::CString::new($s).unwrap().as_ptr()
    };
}

macro_rules! from_cstr {
    ($s:expr) => {
        ::std::ffi::CStr::from_ptr($s.as_ref().unwrap())
            .to_string_lossy()
            .to_owned();
    };
}
