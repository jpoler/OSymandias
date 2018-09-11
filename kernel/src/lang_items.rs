use console::_print;

#[no_mangle]
#[cfg(not(test))]
#[lang = "panic_fmt"]
pub extern "C" fn panic_fmt(
    fmt: ::std::fmt::Arguments,
    file: &'static str,
    line: u32,
    col: u32,
) -> ! {
    // FIXME: Print `fmt`, `file`, and `line` to the console.

    kprint!(
        "panic: {}\nfile: {}\nline: {}\ncol: {}\n",
        fmt,
        file,
        line,
        col
    );

    loop {
        unsafe { asm!("wfe") }
    }
}

#[cfg(not(test))]
#[lang = "eh_personality"]
pub extern "C" fn eh_personality() {}
