use std::fmt::Octal;

pub fn as_octal(num: impl Octal) -> u32 {
    format!("{:o}", num)
        .parse()
        .expect("octal conversion should never fail")
}
