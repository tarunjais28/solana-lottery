pub const fn vec_max_len(element_size: usize, num_elements: usize) -> usize {
    // Borsh stores Vec length as u32
    4 + element_size * num_elements
}

pub const fn option_max_len(element_size: usize) -> usize {
    1 + element_size
}
