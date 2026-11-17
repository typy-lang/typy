//! A small interpreter: arithmetic expressions written out, tokenized and evaluated.

const MODULUS: i64 = 1000003;

fn expression(seed: i64) -> String {
    let parts: Vec<String> = (0..12)
        .map(|k| {
            let value = (seed * 31 + k * 17) % 97 + 1;
            match k % 4 {
                0 => format!("({value}"),
                3 => format!("{value})"),
                _ => value.to_string(),
            }
        })
        .collect();
    let operators = ["+", "*", "-", "+"];
    let mut text = parts[0].clone();
    for k in 1..12 {
        text += &format!(" {} {}", operators[((seed + k) % 4) as usize], parts[k as usize]);
    }
    text
}

fn tokens(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut number = String::new();
    for character in text.chars() {
        if character.is_ascii_digit() {
            number.push(character);
            continue;
        }
        if !number.is_empty() {
            found.push(std::mem::take(&mut number));
        }
        if character != ' ' {
            found.push(character.to_string());
        }
    }
    if !number.is_empty() {
        found.push(number);
    }
    found
}

struct Parser {
    items: Vec<String>,
    position: usize,
}

impl Parser {
    fn peek(&self) -> &str {
        self.items.get(self.position).map_or("", String::as_str)
    }

    fn take(&mut self) -> String {
        self.position += 1;
        self.items[self.position - 1].clone()
    }

    fn sum(&mut self) -> i64 {
        let mut value = self.product();
        while self.peek() == "+" || self.peek() == "-" {
            if self.take() == "+" {
                value = (value + self.product()).rem_euclid(MODULUS);
            } else {
                value = (value - self.product()).rem_euclid(MODULUS);
            }
        }
        value
    }

    fn product(&mut self) -> i64 {
        let mut value = self.atom();
        while self.peek() == "*" {
            self.take();
            value = (value * self.atom()).rem_euclid(MODULUS);
        }
        value
    }

    fn atom(&mut self) -> i64 {
        let item = self.take();
        if item == "(" {
            let value = self.sum();
            self.take();
            return value;
        }
        item.parse().unwrap()
    }
}

fn main() {
    let n: i64 = std::env::args().nth(1).and_then(|n| n.parse().ok()).unwrap_or(2000);
    let mut total = 0;
    for i in 0..n {
        let mut parser = Parser { items: tokens(&expression(i)), position: 0 };
        total = (total + parser.sum()).rem_euclid(MODULUS);
    }
    println!("{total}");
}
