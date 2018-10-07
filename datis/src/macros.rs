macro_rules! cstr {
    ($s:expr) => {
        const_cstr!($s).as_ptr()
    };
}

macro_rules! from_cstr {
    ($s:expr) => {
        ::std::ffi::CStr::from_ptr($s.as_ref().unwrap())
            .to_string_lossy()
            .to_owned();
    };
}

macro_rules! get {
    ($o:expr, $k:expr) => {
        $o.get($k).ok_or_else(|| Error::Undefined($k.to_string()))
    };
}
