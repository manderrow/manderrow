pub fn main() {
    let max_digits = std::env::args()
        .skip(1)
        .next()
        .expect("Missing argument DIGITS")
        .parse::<u32>()
        .expect("Invalid value for argument DIGITS");

    let max_bits = ((((max_digits as f64) / 3.0) / 2.0f64.log10()).ceil() as u32) * 3;
    let index_bits = (max_bits - 1).next_power_of_two().ilog2() + 1;
    println!("Digit bits: {}", max_bits);
    println!("Index bits: {} * 2", index_bits);
    println!("          = {}", max_bits + index_bits * 2);

    println!();

    let packer = packed_semver::Packer::from_digits(max_digits);
    println!("Digit bits: {}", packer.digit_bits);
    println!("Index bits: {} * 2", packer.index_bits);
    println!("          = {}", packer.digit_bits + packer.index_bits * 2);
    println!("Digit mask: {:x}", packer.digit_mask);
    println!("Index mask: {:x}", packer.index_mask);
}
