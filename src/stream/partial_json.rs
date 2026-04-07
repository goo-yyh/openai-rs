use serde_json::{Map, Value};

pub(super) fn parse_optional_json(payload: &str) -> Option<Value> {
    serde_json::from_str(payload)
        .ok()
        .or_else(|| PartialJsonParser::new(payload).parse())
}

/// 表示一次宽松 JSON 解析的中间结果。
struct PartialParse<T> {
    value: T,
    complete: bool,
}

/// 一个尽力而为的 partial JSON 解析器。
///
/// 该解析器用于流式 structured output / tool arguments 的中途快照解析：
/// 当文本尚未闭合时，会尽可能补全末尾缺口并返回当前可推导的 JSON 值。
struct PartialJsonParser {
    chars: Vec<char>,
    index: usize,
}

impl PartialJsonParser {
    fn new(payload: &str) -> Self {
        Self {
            chars: payload.chars().collect(),
            index: 0,
        }
    }

    fn parse(mut self) -> Option<Value> {
        self.skip_whitespace();
        let parsed = self.parse_value()?;
        self.skip_whitespace();
        Some(parsed.value)
    }

    fn parse_value(&mut self) -> Option<PartialParse<Value>> {
        self.skip_whitespace();
        match self.peek()? {
            '{' => self.parse_object(),
            '[' => self.parse_array(),
            '"' => self.parse_string().map(|parsed| PartialParse {
                value: Value::String(parsed.value),
                complete: parsed.complete,
            }),
            't' => self.parse_literal("true", Value::Bool(true)),
            'f' => self.parse_literal("false", Value::Bool(false)),
            'n' => self.parse_literal("null", Value::Null),
            '-' | '0'..='9' => self.parse_number(),
            _ => None,
        }
    }

    fn parse_object(&mut self) -> Option<PartialParse<Value>> {
        self.consume('{')?;
        let mut object = Map::new();
        let mut complete = false;

        loop {
            self.skip_whitespace();
            match self.peek() {
                None => break,
                Some('}') => {
                    self.index += 1;
                    complete = true;
                    break;
                }
                _ => {}
            }

            let Some(key) = self.parse_string() else {
                return if self.peek().is_none() {
                    Some(PartialParse {
                        value: Value::Object(object),
                        complete: false,
                    })
                } else {
                    None
                };
            };

            self.skip_whitespace();
            match self.peek() {
                Some(':') => {
                    self.index += 1;
                }
                None => {
                    return Some(PartialParse {
                        value: Value::Object(object),
                        complete: false,
                    });
                }
                Some(_) => return None,
            }

            self.skip_whitespace();
            let Some(value) = self.parse_value() else {
                return if self.peek().is_none() {
                    Some(PartialParse {
                        value: Value::Object(object),
                        complete: false,
                    })
                } else {
                    None
                };
            };
            object.insert(key.value, value.value);

            self.skip_whitespace();
            match self.peek() {
                Some(',') => {
                    self.index += 1;
                }
                Some('}') => {
                    self.index += 1;
                    complete = true;
                    break;
                }
                None => break,
                Some(_) => return None,
            }
        }

        Some(PartialParse {
            value: Value::Object(object),
            complete,
        })
    }

    fn parse_array(&mut self) -> Option<PartialParse<Value>> {
        self.consume('[')?;
        let mut values = Vec::new();
        let mut complete = false;

        loop {
            self.skip_whitespace();
            match self.peek() {
                None => break,
                Some(']') => {
                    self.index += 1;
                    complete = true;
                    break;
                }
                _ => {}
            }

            let Some(value) = self.parse_value() else {
                return if self.peek().is_none() {
                    Some(PartialParse {
                        value: Value::Array(values),
                        complete: false,
                    })
                } else {
                    None
                };
            };
            values.push(value.value);

            self.skip_whitespace();
            match self.peek() {
                Some(',') => {
                    self.index += 1;
                }
                Some(']') => {
                    self.index += 1;
                    complete = true;
                    break;
                }
                None => break,
                Some(_) => return None,
            }
        }

        Some(PartialParse {
            value: Value::Array(values),
            complete,
        })
    }

    fn parse_string(&mut self) -> Option<PartialParse<String>> {
        self.consume('"')?;
        let mut output = String::new();
        let mut complete = false;

        while let Some(ch) = self.next() {
            match ch {
                '"' => {
                    complete = true;
                    break;
                }
                '\\' => {
                    let Some(escaped) = self.next() else {
                        break;
                    };
                    match escaped {
                        '"' | '\\' | '/' => output.push(escaped),
                        'b' => output.push('\u{0008}'),
                        'f' => output.push('\u{000C}'),
                        'n' => output.push('\n'),
                        'r' => output.push('\r'),
                        't' => output.push('\t'),
                        'u' => {
                            let mut code = String::new();
                            for _ in 0..4 {
                                let Some(hex) = self.next() else {
                                    return Some(PartialParse {
                                        value: output,
                                        complete: false,
                                    });
                                };
                                if !hex.is_ascii_hexdigit() {
                                    return None;
                                }
                                code.push(hex);
                            }
                            if let Ok(codepoint) = u32::from_str_radix(&code, 16)
                                && let Some(decoded) = char::from_u32(codepoint)
                            {
                                output.push(decoded);
                            }
                        }
                        _ => output.push(escaped),
                    }
                }
                other => output.push(other),
            }
        }

        Some(PartialParse {
            value: output,
            complete,
        })
    }

    fn parse_number(&mut self) -> Option<PartialParse<Value>> {
        let start = self.index;
        while matches!(self.peek(), Some('-' | '+' | '.' | 'e' | 'E' | '0'..='9')) {
            self.index += 1;
        }

        if start == self.index {
            return None;
        }

        let raw: String = self.chars[start..self.index].iter().collect();
        let mut candidate = raw.clone();
        while !candidate.is_empty() {
            if let Ok(value) = serde_json::from_str::<Value>(&candidate) {
                return Some(PartialParse {
                    value,
                    complete: candidate.len() == raw.len(),
                });
            }
            candidate.pop();
        }

        None
    }

    fn parse_literal(&mut self, literal: &str, value: Value) -> Option<PartialParse<Value>> {
        let start = self.index;
        let mut matched = 0usize;

        for expected in literal.chars() {
            match self.peek() {
                Some(actual) if actual == expected => {
                    self.index += 1;
                    matched += 1;
                }
                Some(_) => {
                    self.index = start;
                    return None;
                }
                None => break,
            }
        }

        if matched == 0 {
            self.index = start;
            return None;
        }

        Some(PartialParse {
            value,
            complete: matched == literal.chars().count(),
        })
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(' ' | '\n' | '\r' | '\t')) {
            self.index += 1;
        }
    }

    fn consume(&mut self, expected: char) -> Option<()> {
        (self.peek()? == expected).then(|| {
            self.index += 1;
        })
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.index).copied()
    }

    fn next(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.index += 1;
        Some(ch)
    }
}

#[cfg(test)]
mod tests {
    use super::parse_optional_json;
    use serde_json::json;

    #[test]
    fn test_should_parse_partial_json_object_snapshot() {
        assert_eq!(
            parse_optional_json("{\"city\":\"Sha"),
            Some(json!({"city":"Sha"}))
        );
        assert_eq!(
            parse_optional_json("{\"items\":[1,2,"),
            Some(json!({"items":[1,2]}))
        );
        assert_eq!(parse_optional_json("{\"city\":}"), None);
        assert_eq!(parse_optional_json("{\"items\":[1,,2]}"), None);
        assert_eq!(parse_optional_json("hello"), None);
    }
}
