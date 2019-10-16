macro_rules! cstr {
    ($s:expr) => {
        const_cstr!($s).as_ptr()
    };
}

macro_rules! get {
    ($o:expr, $k:expr) => {
        $o.get($k).ok_or_else(|| Error::Undefined($k.to_string()))
    };
}
