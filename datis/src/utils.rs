pub fn round(n: f64, max_decimal_places: i32) -> f64 {
    if max_decimal_places == 0 {
        return n.round();
    }
    let m = (10.0f64).powi(max_decimal_places);
    (n * m).round() / m
}

static PHONETIC_NUMBERS: &'static [&str] = &[
    "ZERO", "WUN", "TOO", "TREE", "FOWER", "FIFE", "SIX", "SEVEN", "AIT", "NINER",
];

pub fn pronounce_number<S>(n: S) -> String
where
    S: ToString,
{
    n.to_string()
        .chars()
        .filter_map(|c| match c {
            '.' => Some(String::from("DAYSEEMAL")),
            '0'..='9' => Some(String::from(
                PHONETIC_NUMBERS[c.to_digit(10).unwrap() as usize],
            )),
            _ => Some(c.to_string()),
        })
        .collect::<Vec<String>>()
        .join(" ")
}
