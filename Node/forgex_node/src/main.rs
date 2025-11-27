fn to_utf8_bytes<T: ToString>(value: T) -> Vec<u8> {
    let s = value.to_string();
    let bytes = s.as_bytes().to_vec();

    println!("Строка: {}", s);
    println!("Байты: {:?}", bytes);
    println!("Количество байтов: {}", bytes.len());

    bytes
}

fn main() {
    to_utf8_bytes("pipe-v1");
}