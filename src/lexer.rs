use std::iter::Peekable;
use std::str::Chars;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
	Int(i64),
	Float(f64),
	Str(String),
	Ident(String),	

	Let,
	Fn,
	If,
	Else,
	While,
	For,
	In,
	Return,
	Break,
	Continue,
	True,
	False,

	Plus,
	Minus,
	Star,
	Slash,
	Percent,
	Eq,
	EqEq,
	NotEq,
	Lt,
	LtEq,
	Gt,
	GtEq,
	AndAnd,
	OrOr,
	Bang,
	Dot,
	Comma,
	Semicolon,
	Colon,
	LParen,
	RParen,
	LBrace,
	RBrace,
	LBracket,
	RBracket,

	Eof,		
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
	pub kind: TokenKind,
	pub line: usize,
	pub column: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LexError {
	pub message: String,
	pub line: usize,
	pub column: usize,
}

pub struct Lexer<'a> {
	chars: Peekable<Chars<'a>>,
	line: usize,
	column: usize,
}

impl<'a> Lexer<'a> {
	pub fn new(source: &'a str) -> Self {
		Lexer {
			chars: source.chars().peekable(),
			line: 1,
			column: 1,
		}
	}

	pub fn tokenize(mut self) -> Result<Vec<Token>, LexError> {
		let mut tokens = Vec::new();
		loop {
			let token = self.next_token()?;
			let is_eof = token.kind == TokenKind::Eof;
			tokens.push(token);
			if is_eof {
				break;
			}
		}
		Ok(tokens)
	}

	fn next_token(&mut self) -> Result<Token, LexError> {
		self.skip_whitespace_and_comments();
		
		let (line, column) = (self.line, self.column);
		
		let Some(&c) = self.chars.peek() else {
			return Ok(Token { kind: TokenKind::Eof, line, column });
		};

		let kind = if c.is_ascii_digit() {
			self.read_number()?
		} else if c == '"' {
			self.read_string()?
		} else if c.is_alphabetic() || c == '_' {
			self.read_ident_or_keyword()
		} else {
			self.read_symbol()?
		};

		Ok(Token {kind, line, column })
	}

	fn skip_whitespace_and_comments(&mut self) {
		loop {
			match self.chars.peek() {
				Some(c) if c.is_whitespace() => {
					self.advance();
				}
				Some('/') => {
					let mut lookahead = self.chars.clone();
					lookahead.next();
					if lookahead.peek() == Some(&'/') {
						while let Some(&c) = self.chars.peek() {
							if c == '\n' {
								break;
							}
							self.advance();
						}
					} else {
						break;
					}
				}
				_ => break,
			}
		}
	}

	fn advance(&mut self) -> Option<char> {
		let c = self.chars.next();
		match c {
			Some('\n') => {
				self.line += 1;
				self.column = 1;
			}
			Some(_) => {
				self.column += 1;
			}
			None => {}
		}
		c
	}

	fn read_number(&mut self) -> Result<TokenKind, LexError> {
		let mut text = String::new();
		while let Some(&c) = self.chars.peek() {
			if c.is_ascii_digit() {
				text.push(c);
				self.advance();
			} else {
				break;
			}
		}

		let mut is_float = false;
		if self.chars.peek() == Some(&'.') {
			let mut lookahead = self.chars.clone();
			lookahead.next();
			if lookahead.peek().is_some_and(|c| c.is_ascii_digit()) {
				is_float = true;
				text.push('.');
				self.advance();
				while let Some(&c) = self.chars.peek() {
					if c.is_ascii_digit() {
						text.push(c);
						self.advance();
					} else {
						break;
					}
				}

			}
		}

		if is_float {
			text.parse::<f64>()
				.map(TokenKind::Float)
				.map_err(|_| self.error(format!("invalid float literal: {text}")))
		} else {
			text.parse::<i64>()
				.map(TokenKind::Int)
				.map_err(|_| self.error(format!("invalid int literal: {text}")))
		}	
	}

	fn read_string(&mut self) -> Result<TokenKind, LexError> {
		self.advance();
		let mut text = String::new();
		loop {
			match self.advance() {
				Some('"') => break,
				Some('\\') => match self.advance() {
					Some('n') => text.push('\n'),
					Some('t') => text.push('\t'),
					Some('"') => text.push('"'),
					Some('\\') => text.push('\\'),
					Some(other) => {
						return Err(self.error(format!("invalid escape sequence: \\{other}")));
					}
					None => return Err(self.error("unterminated string literal".to_string())),
				},
				Some(c) => text.push(c),
				None => return Err(self.error("unterminated string literal".to_string())),
			}
		}
		Ok(TokenKind::Str(text))
	}

	fn read_ident_or_keyword(&mut self) -> TokenKind {
		let mut text = String::new();
		while let Some(&c) = self.chars.peek() {
			if c.is_alphanumeric() || c == '_' {
				text.push(c);
				self.advance();
			} else {
				break;
			}
		}

		match text.as_str() {
			"let" => TokenKind::Let,
			"fn" => TokenKind::Fn,
			"if" => TokenKind::If,
			"else" => TokenKind::Else,
			"while" => TokenKind::While,
			"for" => TokenKind::For,
			"in" => TokenKind::In,
			"return" => TokenKind::Return,
			"break" => TokenKind::Break,
			"continue" => TokenKind::Continue,
			"true" => TokenKind::True,
			"false" => TokenKind::False,
			_ => TokenKind::Ident(text),
		}
	}

	fn read_symbol(&mut self) -> Result<TokenKind, LexError> {
		let c = self.advance().expect("confirm existing by peek");

		let kind = match c {
			'+' => TokenKind::Plus,
			'-' => TokenKind::Minus,
			'*' => TokenKind::Star,
			'/' => TokenKind::Slash,
			'%' => TokenKind::Percent,
			'=' => {
				if self.consume_if('=') {
					TokenKind::EqEq
				} else {
					TokenKind::Eq
				}
			}
			'!' => {
				if self.consume_if('=') {
					TokenKind::NotEq
				} else {
					TokenKind::Bang
				}
			}
			'<' => {
				if self.consume_if('=') {
					TokenKind::LtEq
				} else {
					TokenKind::Lt
				}
			}
			'>' => {
				if self.consume_if('=') {
					TokenKind::GtEq
				} else {
					TokenKind::Gt
				}
			}
			'&' => {
				if self.consume_if('&') {
					TokenKind::AndAnd
				} else {
					return Err(self.error("invalid literal: '&' (you can use only '&&')".to_string()));
				}
			}
			'|' => {
				if self.consume_if('|') {
					TokenKind::OrOr
				} else {
					return Err(self.error("invalid literal: '|' (you can use only '||')".to_string()));
				}
			}
			'.' => TokenKind::Dot,
			',' => TokenKind::Comma,
			';' => TokenKind::Semicolon,
			':' => TokenKind::Colon,
			'(' => TokenKind::LParen,
			')' => TokenKind::RParen,
			'{' => TokenKind::LBrace,
			'}' => TokenKind::RBrace,
			'[' => TokenKind::LBracket,
			']' => TokenKind::RBracket,
			other => return Err(self.error(format!("invalid literal: '{other}'"))),
		};

		Ok(kind)
	}

	fn consume_if(&mut self, expected: char) -> bool {
		if self.chars.peek() == Some(&expected) {
			self.advance();
			true
		} else {
			false
		}
	}

	fn error(&self, message: String) -> LexError {
		LexError { message, line: self.line, column: self.column }
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn kinds(source: &str) -> Vec<TokenKind> {
		Lexer::new(source)
			.tokenize()
			.unwrap()
			.into_iter()
			.map(|t| t.kind)
			.collect()
	}

	#[test]
	fn empty_source_is_eof_only() {
		assert_eq!(kinds(""), vec![TokenKind::Eof]);
	}

	#[test]
	fn numbers() {
		assert_eq!(
			kinds("42 3.14"),
			vec![TokenKind::Int(42), TokenKind::Float(3.14), TokenKind::Eof]
		);
	}

	#[test]
	fn method_call_dot_is_not_confused_with_float() {
		assert_eq!(
			kinds("42.foo"),
			vec![
				TokenKind::Int(42),
				TokenKind::Dot,
				TokenKind::Ident("foo".to_string()),
				TokenKind::Eof
			]
		);
	}

	#[test]
	fn string_with_escapes() {
		assert_eq!(
			kinds(r#""hello\nworld""#),
			vec![TokenKind::Str("hello\nworld".to_string()), TokenKind::Eof]
		);
	}

	#[test]
	fn keywords_and_identifiers() {
		assert_eq!(
			kinds("let x = foo"),
			vec![
				TokenKind::Let,
				TokenKind::Ident("x".to_string()),
				TokenKind::Eq,
				TokenKind::Ident("foo".to_string()),
				TokenKind::Eof
			]
		);
	}

	#[test]
	fn operators() {
		assert_eq!(
			kinds("== != <= >= && ||"),
			vec![
				TokenKind::EqEq,
				TokenKind::NotEq,
				TokenKind::LtEq,
				TokenKind::GtEq,
				TokenKind::AndAnd,
				TokenKind::OrOr,
				TokenKind::Eof,
			]
		);
	}

	#[test]
	fn line_comment_is_skipped() {
		assert_eq!(
			kinds("1 // this is a comment\n2"),
			vec![TokenKind::Int(1), TokenKind::Int(2), TokenKind::Eof]
		);
	}

	#[test]
	fn tracks_line_and_column() {
		let tokens = Lexer::new("let\nx").tokenize().unwrap();
		assert_eq!(tokens[0].line, 1);
		assert_eq!(tokens[1].line, 2);
		assert_eq!(tokens[1].column, 1);
	}

	#[test]
	fn unterminated_string_is_error() {
		assert!(Lexer::new("\"abc").tokenize().is_err());
	}
}