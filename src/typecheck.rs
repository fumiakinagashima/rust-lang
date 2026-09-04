use std::collections::HashMap;

use crate::parser::{BinaryOp, Block, Expr, Stmt, StmtKind, UnaryOp};

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
	Int,
	Float,
	Bool,
	String,
	Array(Box<Type>),
	Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeError {
	pub message: String,
	pub line: usize,
	pub column: usize,
}

struct Env {
	scopes: Vec<HashMap<String, Type>>,
}

impl Env {
	fn new() -> Self {
		Env { scopes: vec![HashMap::new()] }
	}

	fn push_scope(&mut self) {
		self.scopes.push(HashMap::new());
	}

	fn pop_scope(&mut self) {
		self.scopes.pop();
	}

	fn define(&mut self, name: String, ty: Type) {
		self.scopes.last_mut().expect("at least one scope is always present").insert(name, ty);
	}

	fn lookup(&self, name: &str) -> Option<&Type> {
		for scope in self.scopes.iter().rev() {
			if let Some(ty) = scope.get(name) {
				return Some(ty);
			}
		}
		None
	}
}

pub struct TypeChecker {
	env: Env,
}

impl TypeChecker {
	pub fn new() -> Self {
		TypeChecker { env: Env::new() }
	}

	pub fn check_program(&mut self, program: &[Stmt]) -> Result<(), TypeError> {
		self.check_stmts(program)
	}

	fn check_stmts(&mut self, stmts: &[Stmt]) -> Result<(), TypeError> {
		for stmt in stmts {
			self.check_stmt(stmt)?;
		}
		Ok(())
	}

	fn in_new_scope<F>(&mut self, f: F) -> Result<(), TypeError>
	where F:FnOnce(&mut Self) -> Result<(), TypeError>,
	{
		self.env.push_scope();
		let result = f(self);
		self.env.pop_scope();
		result
	}

	fn check_block(&mut self, block: &Block) -> Result<(), TypeError> {
		self.in_new_scope(|checker| checker.check_stmts(block))
	}

	fn check_stmt(&mut self, stmt: &Stmt) -> Result<(), TypeError> {
		let line = stmt.line;
		let column = stmt.column;

		match &stmt.kind {
			StmtKind::Let { name, value } => {
				let ty = self.check_expr(value, line, column)?;
				self.env.define(name.clone(), ty);
				Ok(())
			}
			StmtKind::Expr(expr) => {
				self.check_expr(expr, line, column)?;
				Ok(())
			}
			StmtKind::Block(block) => self.check_block(block),
			StmtKind::If { condition, then_branch, else_branch } => {
				let condition_ty = self.check_expr(condition, line, column)?;
				self.expect_bool(&condition_ty, line, column)?;
				self.check_block(then_branch)?;
				if let Some(else_stmt) = else_branch {
					self.check_stmt(else_stmt)?;
				}
				Ok(())
			}
			StmtKind::While { condition, body } => {
				let condition_ty = self.check_expr(condition, line, column)?;
				self.expect_bool(&condition_ty, line, column)?;
				self.check_block(body)
			}
			StmtKind::For { var, iterable, body } => {
				let iterable_ty = self.check_expr(iterable, line, column)?;
				let element_ty = match iterable_ty {
					Type::Array(element) => *element,
					Type::Unknown => Type::Unknown,
					other => {
						return Err(self.error(line, column, format!("for-loop expects an array, found {other:?}")));
					}
				};
				self.in_new_scope(|checker| {
					checker.env.define(var.clone(), element_ty);
					checker.check_stmts(body)
				})
			}
			StmtKind::Return(value) => {
				if let Some(expr) = value {
					self.check_expr(expr, line, column)?;
				}
				Ok(())
			}
			StmtKind::Break | StmtKind::Continue => Ok(()),
		}
	}

	fn check_expr(&mut self, expr: &Expr, line: usize, column: usize) -> Result<Type, TypeError> {
		match expr {
			Expr::Int(_) => Ok(Type::Int),
			Expr::Float(_) => Ok(Type::Float),
			Expr::Str(_) => Ok(Type::String),
			Expr::Bool(_) => Ok(Type::Bool),
			Expr::Ident(name) => self
				.env
				.lookup(name)
				.cloned()
				.ok_or_else(|| self.error(line, column, format!("undefined variable '{name}'"))),
			Expr::Array(elements) => self.check_array(elements, line, column),
			Expr::Unary { op, operand } => {
				let operand_ty = self.check_expr(operand, line, column)?;
				self.check_unary_op(op, operand_ty, line, column)
			}
			Expr::Binary { op, left, right } => {
				let left_ty = self.check_expr(left, line, column)?;
				let right_ty = self.check_expr(right, line, column)?;
				self.check_binary_op(op, left_ty, right_ty, line, column)
			}
			Expr::Call { .. } => Ok(Type::Unknown),
			Expr::Index { object, index } => {
				let object_ty = self.check_expr(object, line, column)?;
				let index_ty = self.check_expr(index, line, column)?;
				if index_ty != Type::Int && index_ty != Type::Unknown {
					return Err(self.error(line, column, format!("array index must be int, found {index_ty:?}")));
				}
				match object_ty {
					Type::Array(element) => Ok(*element),
					Type::Unknown => Ok(Type::Unknown),
					other => Err(self.error(line, column, format!("cannot index into {other:?}"))),
				}
			}
		}
	}

	fn check_array(&mut self, elements: &[Expr], line: usize, column: usize) -> Result<Type, TypeError> {
		let mut elements = elements.iter();
		let Some(first) = elements.next() else {
			return Err(self.error(line, column, "cannot infer the type of an empty array".to_string()));
		};

		let element_ty = self.check_expr(first, line, column)?;
		for element in elements {
			let ty = self.check_expr(element, line, column)?;
			if ty != element_ty && ty != Type::Unknown && element_ty != Type::Unknown {
				return Err(self.error(
					line,
					column,
					format!("array elements must have the same type: expected {element_ty:?}, found {ty:?}"),
				));
			}
		}
		Ok(Type::Array(Box::new(element_ty)))
	}

	fn check_unary_op(&self, op: &UnaryOp, operand: Type, line: usize, column: usize) -> Result<Type, TypeError> {
		match op {
			UnaryOp::Neg => match operand {
				Type::Int | Type::Float | Type::Unknown => Ok(operand),
				other => Err(self.error(line, column, format!("unary '-' requires a number, found {other:?}"))),
			},
			UnaryOp::Not => match operand {
				Type::Bool | Type::Unknown => Ok(operand),
				other => Err(self.error(line, column, format!("unary '!' requires a bool, found {other:?}"))),
			},
		}
	}

	fn check_binary_op(&self, op: &BinaryOp, left: Type, right: Type, line: usize, column: usize) -> Result<Type, TypeError> {
		if left == Type::Unknown || right == Type::Unknown {
			return Ok(self.binary_op_result_for_unknown(op));
		}

		match op {
			BinaryOp::Add
			| BinaryOp::Sub
			| BinaryOp::Mul
			| BinaryOp::Div
			| BinaryOp::Mod => match (&left, &right) {
				(Type::Int, Type::Int) => Ok(Type::Int),
				(Type::Float, Type::Float) => Ok(Type::Float),
				_ => Err(self.error(line, column, format!("operator '{op:?}' cannot be applied to {left:?} and {right:?}"))),
			},
			BinaryOp::Eq | BinaryOp::NotEq => {
				if left == right {
					Ok(Type::Bool)
				} else {
					Err(self.error(line, column, format!("cannot compare {left:?} and {right:?}")))
				}
			}
			BinaryOp::Lt
			| BinaryOp::LtEq
			| BinaryOp::Gt
			| BinaryOp::GtEq => match (&left, &right) {
				(Type::Int, Type::Int) | (Type::Float, Type::Float) => Ok(Type::Bool),
				_ => Err(self.error(line, column, format!("operator '{op:?}' requires two numbers of the type, found {left:?} and {right:?}"))),
			},
			BinaryOp::And | BinaryOp::Or => match (&left, &right) {
				(Type::Bool, Type::Bool) => Ok(Type::Bool),
				_ => Err(self.error(line, column, format!("operator '{op:?}' requires two bools, found {left:?} and {right:?}"))),
			},
		}
	}

	fn binary_op_result_for_unknown(&self, op: &BinaryOp) -> Type {
		match op {
			BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => Type::Unknown,
			BinaryOp::Eq | BinaryOp::NotEq | BinaryOp::Lt | BinaryOp::LtEq | BinaryOp::Gt | BinaryOp::GtEq | BinaryOp::And | BinaryOp::Or => Type::Bool,
		}
	}

	fn expect_bool(&self, ty: &Type, line: usize, column: usize) -> Result<(), TypeError> {
		match ty {
			Type::Bool | Type::Unknown => Ok(()),
			other => Err(self.error(line, column, format!("expected bool, found {other:?}"))),
		}
	}

	fn error(&self, line: usize, column: usize, message: String) -> TypeError {
		TypeError { message, line, column }
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::lexer::Lexer;
	use crate::parser::Parser;

	fn check(source: &str) -> Result<(), TypeError> {
		let tokens = Lexer::new(source).tokenize().unwrap();
		let program = Parser::new(tokens).parse_program().unwrap();
		TypeChecker::new().check_program(&program)
	}

	#[test]
	fn let_infers_type_from_initializer() {
		assert!(check("let x = 1; let y = x + 1;").is_ok());
	}

	#[test]
	fn mismatched_arithmetic_operands_is_error() {
		assert!(check("let x = 1 + true;").is_err());
	}

	#[test]
	fn int_and_float_do_not_mix() {
		assert!(check("let x = 1 + 1.0;").is_err());
	}

	#[test]
	fn comparison_produces_bool() {
		assert!(check("let x = 1 < 2; if (x) { 1; }").is_ok());
	}

	#[test]
	fn if_condition_must_be_bool() {
		assert!(check("if (1) { 1; }").is_err());
	}

	#[test]
	fn while_condition_must_be_bool() {
		assert!(check("while (1) { 1; }").is_err());
	}

	#[test]
	fn undefined_variable_is_error() {
		assert!(check("let x = y;").is_err());
	}

	#[test]
	fn block_scoped_variables_do_not_leak() {
		assert!(check("if (true) { let x = 1; } let y = x;").is_err());
	}

	#[test]
	fn variables_can_be_shadowed_in_inner_scope() {
		assert!(check("let x = 1; if (true) { let x = true; if (x) { 1; } }").is_ok());
	}

	#[test]
	fn array_elements_must_share_a_type() {
		assert!(check("let a = [1, true, 3];").is_err());
	}

	#[test]
	fn empty_array_type_cannot_be_inferred() {
		assert!(check("let a = [];").is_err());
	}

	#[test]
	fn for_loop_binds_element_type_from_array() {
		assert!(check("for (n in [1, 2, 3]) { let x = n + 1; }").is_ok());
	}

	#[test]
	fn for_loop_requires_an_array() {
		assert!(check("for (n in 1) { n; }").is_err());
	}

	#[test]
	fn indexing_a_non_array_is_error() {
		assert!(check("let x = 1; x[0];").is_err());
	}

	#[test]
	fn array_index_must_be_int() {
		assert!(check("let a = [1, 2]; a[true];").is_err());
	}

	#[test]
	fn function_calls_are_unchecked_for_now() {
		assert!(check("let x = add(1, 2);").is_ok());
	}
}