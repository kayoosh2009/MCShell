/// Русская/украинская раскладка -> латинская клавиша на той же кнопке.
pub fn to_latin(c: char) -> char {
    const RU: &str = "йцукенгшщзхъфывапролджэячсмитьбюіїє";
    const EN: &str = "qwertyuiop[]asdfghjkl;'zxcvbnm,.s]'";
    let lc = c.to_lowercase().next().unwrap_or(c);
    RU.chars().position(|x| x == lc).and_then(|i| EN.chars().nth(i)).unwrap_or(c)
}