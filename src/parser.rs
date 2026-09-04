use crate::lexer::{Token, TokenKind};

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
	Neg,
	Not,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
	Add,
	Sub,
	Mul,
	Div,
	Mod,
	Eq,
	NotEq,
	Lt,
	LtEq,
	Gt,
	GtEq,
	And,
	Or,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
	Int(i64),
	Float(f64),
	Str(String),
	Bool(bool),
	Ident(String),
	Array(Vec<Expr>),
	Unary {
		op: UnaryOp,
		operand: Box<Expr>,
	},
	Binary {
		op: BinaryOp,
		left: Box<Expr>,
		right: Box<Expr>,
	},
	Call {
		callee: Box<Expr>,
		args: Vec<Expr>,
	},
	Index {
		object: Box<Expr>,
		index: Box<Expr>,
	},
}

pub type Block = Vec<Stmt>;
#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
	Let {
		name: String,
		value: Expr,
	},
	Expr(Expr),
	Block(Block),
	If {
		condition: Expr,
		then_branch: Block,
		else_branch: Option<Box<Stmt>>,
	},
	While {
		condition: Expr,
		body: Block,
	},
	For {
		var: String,
		iterable: Expr,
		body: Block,
	},
	Return(Option<Expr>),
	Break,
	Continue,
}

#[derive(Debug, Clone)]
pub struct Stmt {
	pub kind: StmtKind,
	pub line: usize,
	pub column: usize,
}

impl PartialEq for Stmt {
	fn eq(&self, other: &Self) -> bool {
		self.kind == other.kind
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
	pub message: String,
	pub line: usize,
	pub column: usize,		
}

pub struct Parser {
	tokens: Vec<Token>,
	pos: usize,
}

impl Parser {
	pub fn new(tokens: Vec<Token>) -> Self {
		Parser { tokens, pos: 0 }
	}

	pub fn parse_expression(&mut self) -> Result<Expr, ParseError> {
		self.parse_expr(0)
	}

	pub fn parse_program(&mut self) -> Result<Vec<Stmt>, ParseError> {
		let mut stmts = Vec::new();
		while self.peek_kind() != &TokenKind::Eof {
			stmts.push(self.parse_stmt()?);
		}
		Ok(stmts)
	}

	fn parse_stmt(&mut self) -> Result<Stmt, ParseError> {
		self.with_position(|parser| match parser.peek_kind() {
			TokenKind::Let => parser.parse_let_stmt(),
			TokenKind::If => parser.parse_if_stmt(),
			TokenKind::While => parser.parse_while_stmt(),
			TokenKind::For => parser.parse_for_stmt(),
			TokenKind::Return => parser.parse_return_stmt(),
			TokenKind::Break => {
				parser.advance();
				parser.expect(TokenKind::Semicolon, "expected ';' after 'break'")?;
				Ok(StmtKind::Break)
			}
			TokenKind::Continue => {
				parser.advance();
				parser.expect(TokenKind::Semicolon, "expected ';' after 'continue'")?;
				Ok(StmtKind::Continue)
			}
			TokenKind::LBrace => Ok(StmtKind::Block(parser.parse_block()?)),
			_ => parser.parse_expr_stmt(),
		})
	}

	fn with_position<F>(&mut self, f: F) -> Result<Stmt, ParseError>
	where F: FnOnce(&mut Self) -> Result<StmtKind, ParseError>,
	{
		let start = self.peek().clone();
		let kind = f(self)?;
		Ok(Stmt { kind, line: start.line, column: start.column })
	}

	fn parse_let_stmt(&mut self) -> Result<StmtKind, ParseError> {
		self.advance();
		let name = self.parse_ident()?;
		self.expect(TokenKind::Eq, "expected '=' after variable name")?;
		let value = self.parse_expression()?;
		self.expect(TokenKind::Semicolon, "expected ';' after let statement")?;
		
		Ok(StmtKind::Let { name, value })
	}

	fn parse_if_stmt(&mut self) -> Result<StmtKind, ParseError> {
		self.advance();
		self.expect(TokenKind::LParen, "expected '(' after 'if'")?;
		let condition = self.parse_expression()?;
		self.expect(TokenKind::RParen, "expected ')' after if condition")?;
		let then_branch = self.parse_block()?;

		let else_branch = if self.peek_kind() == &TokenKind::Else {
			self.advance();
			if self.peek_kind() == &TokenKind::If {
				Some(Box::new(self.with_position(|parser| parser.parse_if_stmt())?))
			} else {
				Some(Box::new(self.with_position(|parser| Ok(StmtKind::Block(parser.parse_block()?)))?))
			}
		} else {
			None
		};

		Ok(StmtKind::If { condition, then_branch, else_branch })
	}

	fn parse_while_stmt(&mut self) -> Result<StmtKind, ParseError> {
		self.advance();
		self.expect(TokenKind::LParen, "expected '(' after 'while'")?;
		let condition = self.parse_expression()?;
		self.expect(TokenKind::RParen, "expected ')' after while condition")?;
		let body = self.parse_block()?;

		Ok(StmtKind::While { condition, body })
	}

	fn parse_for_stmt(&mut self) -> Result<StmtKind, ParseError> {
		self.advance();
		self.expect(TokenKind::LParen, "expected '(' after 'for'")?;
		let var = self.parse_ident()?;
		self.expect(TokenKind::In, "expected 'in' after for-loop variable")?;
		let iterable = self.parse_expression()?;
		self.expect(TokenKind::RParen, "expected ')' after for-loop header")?;
		let body = self.parse_block()?;

		Ok(StmtKind::For { var, iterable, body })
	}

	fn parse_return_stmt(&mut self) -> Result<StmtKind, ParseError> {
		self.advance();
		let value = if self.peek_kind() == &TokenKind::Semicolon {
			None
		} else {
			Some(self.parse_expression()?)
		};
		self.expect(TokenKind::Semicolon, "expected ';' after return statement")?;

		Ok(StmtKind::Return(value))
	}

	fn parse_expr_stmt(&mut self) -> Result<StmtKind, ParseError> {
		let expr = self.parse_expression()?;
		self.expect(TokenKind::Semicolon, "expected ';' after expression statement")?;

		Ok(StmtKind::Expr(expr))
	}

	fn parse_block(&mut self) -> Result<Block, ParseError> {
		self.expect(TokenKind::LBrace, "expected '{'")?;
		let mut stmts = Vec::new();
		while self.peek_kind() != &TokenKind::RBrace {
			stmts.push(self.parse_stmt()?);
		}
		self.expect(TokenKind::RBrace, "expected '}'")?;

		Ok(stmts)
	}

	fn parse_ident(&mut self) -> Result<String, ParseError> {
		let token = self.advance();
		match token.kind {
			TokenKind::Ident(name) => Ok(name),
			other => Err(self.error(token.line, token.column, format!("expected identifier, found{other:?}"))),
		}
	}

	fn parse_expr(&mut self, min_bp: u8) -> Result<Expr, ParseError> {
		let mut lhs = self.parse_prefix()?;

		loop {
			if let Some(l_bp) = postfix_binding_power(self.peek_kind()) {
				if l_bp < min_bp {
					break;
				}
				lhs = self.parse_postfix(lhs)?;
				continue;
			}

			let Some((l_bp, r_bp)) = infix_binding_power(self.peek_kind()) else {
				break;
			};
			if l_bp < min_bp {
				break;
			}
			
			let op_token = self.advance();
			let op = to_binary_op(&op_token.kind);
			let rhs = self.parse_expr(r_bp)?;
			lhs = Expr::Binary { op, left: Box::new(lhs), right: Box::new(rhs) };
		}

		Ok(lhs)
	}

	fn parse_prefix(&mut self) -> Result<Expr, ParseError> {
		if let Some(bp) = prefix_binding_power(self.peek_kind()) {
			let op_token = self.advance();
			let op = match op_token.kind {
				TokenKind::Minus => UnaryOp::Neg,
				TokenKind::Bang => UnaryOp::Not,
				_ => unreachable!("prefix_binding_power only returns Some for '-' and '!'"),
			};
			let operand = self.parse_expr(bp)?;
			return Ok(Expr::Unary { op, operand: Box::new(operand) });
		}

		self.parse_primary()
	}

	fn parse_primary(&mut self) -> Result<Expr, ParseError> {
		let token = self.advance();
		match token.kind {
			TokenKind::Int(value) => Ok(Expr::Int(value)),
			TokenKind::Float(value) => Ok(Expr::Float(value)),
			TokenKind::Str(value) => Ok(Expr::Str(value)),
			TokenKind::True => Ok(Expr::Bool(true)),
			TokenKind::False => Ok(Expr::Bool(false)),
			TokenKind::Ident(name) => Ok(Expr::Ident(name)),
			TokenKind::LParen => {
				let expr = self.parse_expr(0)?;
				self.expect(TokenKind::RParen, "expected ')' after expression")?;
				Ok(expr)
			}
			TokenKind::LBracket => {
				let elements = self.parse_args(TokenKind::RBracket)?;
				Ok(Expr::Array(elements))
			}
			other => Err(self.error(token.line, token.column, format!("unexpected token in expression: {other:?}"))),
		}
	}

	fn parse_postfix(&mut self, lhs: Expr) -> Result<Expr, ParseError> {
		match self.peek_kind() {
			TokenKind::LParen => {
				self.advance();
				let args = self.parse_args(TokenKind::RParen)?;
				Ok(Expr::Call { callee: Box::new(lhs), args })
			}
			TokenKind::LBracket => {
				self.advance();
				let index = self.parse_expr(0)?;
				self.expect(TokenKind::RBracket, "expected ']' after index expression")?;
				Ok(Expr::Index { object: Box::new(lhs), index: Box::new(index) })
			}
			_ => unreachable!("parse_postfix is only called when postfix_binding_power matched"),
		}
	}

	fn parse_args(&mut self, closing: TokenKind) -> Result<Vec<Expr>, ParseError> {
		let mut args = Vec::new();
		if self.peek_kind() == &closing {
			self.advance();
			return Ok(args);
		}

		loop {
			args.push(self.parse_expr(0)?);
			if self.peek_kind() == &TokenKind::Comma {
				self.advance();
				continue;
			}
			break;
		}
		self.expect(closing, "expected ',' or closing delimiter")?;
		Ok(args)
	}

	fn peek(&self) -> &Token {
		&self.tokens[self.pos]
	}

	fn peek_kind(&self) -> &TokenKind {
		&self.tokens[self.pos].kind
	}

	fn advance(&mut self) -> Token {
		let token = self.tokens[self.pos].clone();
		if self.pos + 1 < self.tokens.len() {
			self.pos += 1;
		}
		token
	}

	fn expect(&mut self, kind: TokenKind, message: &str) -> Result<Token, ParseError> {
		if self.peek_kind() == &kind {
			Ok(self.advance())
		} else {
			let token = self.peek();
			Err(self.error(token.line, token.column, message.to_string()))
		}
	}

	fn error(&self, line: usize, column: usize, message: String) -> ParseError {
		ParseError { message, line, column }
	}
}

fn to_binary_op(kind: &TokenKind) -> BinaryOp {
	match kind {
		TokenKind::Plus => BinaryOp::Add,
		TokenKind::Minus => BinaryOp::Sub,
		TokenKind::Star => BinaryOp::Mul,
		TokenKind::Slash => BinaryOp::Div,
		TokenKind::Percent => BinaryOp::Mod,
		TokenKind::EqEq => BinaryOp::Eq,
		TokenKind::NotEq => BinaryOp::NotEq,
		TokenKind::Lt => BinaryOp::Lt,
		TokenKind::LtEq => BinaryOp::LtEq,
		TokenKind::Gt => BinaryOp::Gt,
		TokenKind::GtEq => BinaryOp::GtEq,
		TokenKind::AndAnd => BinaryOp::And,
		TokenKind::OrOr => BinaryOp::Or,
		other => unreachable!("infix_binding_power only returns Some for binary operators, got {other:?}"),
	}
}

fn infix_binding_power(kind: &TokenKind) -> Option<(u8, u8)> {
	match kind {
		TokenKind::OrOr => Some((1, 2)),
		TokenKind::AndAnd => Some((3, 4)),
		TokenKind::EqEq | TokenKind::NotEq => Some((5, 6)),
		TokenKind::Lt | TokenKind::LtEq | TokenKind::Gt | TokenKind::GtEq => Some((7, 8)),
		TokenKind::Plus | TokenKind::Minus => Some((9, 10)),
		TokenKind::Star | TokenKind::Slash | TokenKind::Percent => Some((11, 12)),
		_ => None,
	}
}

fn prefix_binding_power(kind: &TokenKind) -> Option<u8> {
	match kind {
		TokenKind::Minus | TokenKind::Bang => Some(13),
		_ => None,
	}
}

fn postfix_binding_power(kind: &TokenKind) -> Option<u8> {
	match kind {
		TokenKind::LParen | TokenKind::LBracket => Some(15),
		_ => None,
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::lexer::Lexer;

	fn parse(source: &str) -> Expr {
		let tokens = Lexer::new(source).tokenize().unwrap();
		Parser::new(tokens).parse_expression().unwrap()
	}

	#[test]
	fn precedence_of_arithmetic_operators() {
		assert_eq!(
			parse("1 + 2 * 3"),
			Expr::Binary {
				op: BinaryOp::Add,
				left: Box::new(Expr::Int(1)),
				right: Box::new(Expr::Binary {
					op: BinaryOp::Mul,
					left: Box::new(Expr::Int(2)),
					right: Box::new(Expr::Int(3)),
				}),
			}
		);
	}

	#[test]
	fn left_associativity_of_same_precedence_operators() {
		assert_eq!(
			parse("1 - 2 - 3"),
			Expr::Binary {
				op: BinaryOp::Sub,
				left: Box::new(Expr::Binary {
					op: BinaryOp::Sub,
					left: Box::new(Expr::Int(1)),
					right: Box::new(Expr::Int(2)),
				}),
				right: Box::new(Expr::Int(3)),
			}
		);
	}

	#[test]
	fn parentheses_override_precedence() {
		assert_eq!(
			parse("(1 + 2) * 3"),
			Expr::Binary {
				op: BinaryOp::Mul,
				left: Box::new(Expr::Binary {
					op: BinaryOp::Add,
					left: Box::new(Expr::Int(1)),
					right: Box::new(Expr::Int(2)),
				}),
				right: Box::new(Expr::Int(3)),
			}
		);
	}

	#[test]
	fn unary_minus_binds_tighter_than_binary_operators() {
		assert_eq!(
			parse("-1 + 2"),
			Expr::Binary {
				op: BinaryOp::Add,
				left: Box::new(Expr::Unary { op: UnaryOp::Neg, operand: Box::new(Expr::Int(1)) }),
				right: Box::new(Expr::Int(2)),
			}
		);
	}

	#[test]
	fn logical_operators_precedence() {
		assert_eq!(
			parse("true || false && true"),
			Expr::Binary {
				op: BinaryOp::Or,
				left: Box::new(Expr::Bool(true)),
				right: Box::new(Expr::Binary {
					op: BinaryOp::And,
					left: Box::new(Expr::Bool(false)),
					right: Box::new(Expr::Bool(true)),
				}),
			}
		);
	}

	#[test]
	fn function_call_with_arguments() {
		assert_eq!(
			parse("add(1, 2 + 3)"),
			Expr::Call {
				callee: Box::new(Expr::Ident("add".to_string())),
				args: vec![
					Expr::Int(1),
					Expr::Binary {
						op: BinaryOp::Add,
						left: Box::new(Expr::Int(2)),
						right: Box::new(Expr::Int(3)),
					},
				],
			}
		);
	}

	#[test]
	fn array_literal_and_index() {
		assert_eq!(
			parse("[1, 2, 3][0]"),
			Expr::Index {
				object: Box::new(Expr::Array(vec![Expr::Int(1), Expr::Int(2), Expr::Int(3)])),
				index: Box::new(Expr::Int(0)),
			}
		);
	}

	#[test]
	fn chained_calls_and_index() {
		assert_eq!(
			parse("f(1)[0]"),
			Expr::Index {
				object: Box::new(Expr::Call {
					callee: Box::new(Expr::Ident("f".to_string())),
					args: vec![Expr::Int(1)],
				}),
				index: Box::new(Expr::Int(0)),
			}
		);
	}

	#[test]
	fn missing_closing_paren_is_error() {
		let tokens = Lexer::new("(1 + 2").tokenize().unwrap();
		assert!(Parser::new(tokens).parse_expression().is_err());
	}

	fn stmt(kind: StmtKind) -> Stmt {
		Stmt { kind, line: 0, column: 0 }
	}

	fn parse_program(source: &str) -> Vec<Stmt> {
		let tokens = Lexer::new(source).tokenize().unwrap();
		Parser::new(tokens).parse_program().unwrap()
	}

	#[test]
	fn let_statement() {
		assert_eq!(
			parse_program("let x = 1 + 2;"),
			vec![stmt(StmtKind::Let {
				name: "x".to_string(),
				value: Expr::Binary {
					op: BinaryOp::Add,
					left: Box::new(Expr::Int(1)),
					right: Box::new(Expr::Int(2)),
				},
			})]
		);
	}

	#[test]
	fn if_else_statement() {
		assert_eq!(
			parse_program("if (x > 5) { x; } else { 0; }"),
			vec![stmt(StmtKind::If {
				condition: Expr::Binary {
					op: BinaryOp::Gt,
					left: Box::new(Expr::Ident("x".to_string())),
					right: Box::new(Expr::Int(5)),
				},
				then_branch: vec![stmt(StmtKind::Expr(Expr::Ident("x".to_string())))],
				else_branch: Some(Box::new(stmt(StmtKind::Block(vec![stmt(StmtKind::Expr(Expr::Int(0)))])))),
			})]
		);
	}

	#[test]
	fn else_if_chain() {
		assert_eq!(
			parse_program("if (a) { 1; } else if (b) { 2; } else { 3; }"),
			vec![stmt(StmtKind::If {
				condition: Expr::Ident("a".to_string()),
				then_branch: vec![stmt(StmtKind::Expr(Expr::Int(1)))],
				else_branch: Some(Box::new(stmt(StmtKind::If {
					condition: Expr::Ident("b".to_string()),
					then_branch: vec![stmt(StmtKind::Expr(Expr::Int(2)))],
					else_branch: Some(Box::new(stmt(StmtKind::Block(vec![stmt(StmtKind::Expr(Expr::Int(3)))])))),
				}))),
			})]
		);
	}

	#[test]
	fn while_statement() {
		assert_eq!(
			parse_program("while (x < 10) { x; }"),
			vec![stmt(StmtKind::While {
				condition: Expr::Binary {
					op: BinaryOp::Lt,
					left: Box::new(Expr::Ident("x".to_string())),
					right: Box::new(Expr::Int(10)),
				},
				body: vec![stmt(StmtKind::Expr(Expr::Ident("x".to_string())))],
			})]
		);
	}

	#[test]
	fn for_statement() {
		assert_eq!(
			parse_program("for (item in arr) { item; }"),
			vec![stmt(StmtKind::For {
				var: "item".to_string(),
				iterable: Expr::Ident("arr".to_string()),
				body: vec![stmt(StmtKind::Expr(Expr::Ident("item".to_string())))],
			})]
		);
	}

	#[test]
	fn return_break_continue_statements() {
		assert_eq!(
			parse_program("return 1; return; break; continue;"),
			vec![
				stmt(StmtKind::Return(Some(Expr::Int(1)))),
				stmt(StmtKind::Return(None)),
				stmt(StmtKind::Break),
				stmt(StmtKind::Continue),
			]
		);
	}

	#[test]
	fn missing_semicolon_is_error() {
		let tokens = Lexer::new("let x = 1").tokenize().unwrap();
		assert!(Parser::new(tokens).parse_program().is_err());
	}
	
	#[test]
	fn missing_closing_bracket_is_error() {
		let tokens = Lexer::new("arr[0").tokenize().unwrap();
		assert!(Parser::new(tokens).parse_expression().is_err());
	}

	#[test]
	fn stmt_tracks_its_starting_position() {
		let program = parse_program("let x = 1;\nlet y = 2;");
		assert_eq!(program[0].line, 1);
		assert_eq!(program[1].line, 2);
	}
}