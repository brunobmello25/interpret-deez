use crate::{
    ast::{
        expression::Expression,
        operator::{InfixOperator, PrefixOperator},
        program::Program,
        statement::Statement,
    },
    expect_peek,
    lexer::Lexer,
    parser::precedence::Precedence,
    token::Token,
};

pub struct ParserError {}

pub struct Parser {
    lexer: Lexer,
    current_token: Token,
    peeking_token: Token,
    pub errors: Vec<ParserError>,
}

impl Parser {
    pub fn new(mut lexer: Lexer) -> Self {
        let current_token = lexer.next_token();
        let peeking_token = lexer.next_token();

        let parser = Parser {
            lexer,
            current_token,
            peeking_token,
            errors: vec![],
        };

        parser
    }

    pub fn parse_program(&mut self) -> Program {
        let mut program = Program::new();

        while self.current_token != Token::EOF {
            let stmt = self.parse_statement();

            match stmt {
                Ok(stmt) => program.statements.push(stmt),
                Err(err) => self.errors.push(err),
            }

            self.advance_tokens();
        }

        program
    }

    fn parse_statement(&mut self) -> Result<Statement, ParserError> {
        match self.current_token {
            Token::Let => self.parse_let_statement(),
            Token::Return => self.parse_return_statement(),
            _ => self.parse_expression_statement(),
        }
    }

    fn parse_expression_statement(&mut self) -> Result<Statement, ParserError> {
        let expression = self.parse_expression(Precedence::LOWEST)?;

        if self.peeking_token == Token::Semicolon {
            self.next_token();
        };

        Ok(Statement::expression(expression))
    }

    fn parse_expression(&mut self, precedence: Precedence) -> Result<Expression, ParserError> {
        let mut lhs = self.parse_prefix()?;

        while self.peeking_token != Token::Semicolon
            && precedence < Precedence::from(&self.peeking_token)
        {
            self.next_token();

            lhs = self.parse_infix(lhs)?;
        }

        Ok(lhs)
    }

    fn advance_tokens(&mut self) {
        while self.current_token != Token::Semicolon && self.current_token != Token::EOF {
            self.next_token();
        }

        if self.current_token == Token::Semicolon {
            self.next_token();
        }
    }

    fn parse_prefix(&mut self) -> Result<Expression, ParserError> {
        match &self.current_token {
            Token::Identifier(identifier) => Ok(Expression::identifier(identifier)),
            Token::Integer(integer_literal) => self.parse_integer(integer_literal),
            Token::LParen => self.parse_grouped_expression(),
            Token::True | Token::False => self.parse_boolean(),
            Token::Bang | Token::Minus => self.parse_prefix_expression(),
            Token::If => self.parse_if_expression(),
            Token::Function => self.parse_function_literal(),
            _ => Err(ParserError {}),
        }
    }

    fn parse_call_expression(&mut self, function: Expression) -> Result<Expression, ParserError> {
        let arguments = self.parse_call_arguments()?;
        Ok(Expression::Call {
            function: Box::new(function),
            arguments,
        })
    }

    fn parse_call_arguments(&mut self) -> Result<Vec<Expression>, ParserError> {
        let mut arguments = vec![];

        if self.peeking_token == Token::RParen {
            self.next_token();
            return Ok(arguments);
        }

        self.next_token();

        arguments.push(self.parse_expression(Precedence::LOWEST)?);

        while self.peeking_token == Token::Comma {
            self.next_token();
            self.next_token();
            arguments.push(self.parse_expression(Precedence::LOWEST)?);
        }

        expect_peek!(self, RParen)?;

        Ok(arguments)
    }

    fn parse_function_literal(&mut self) -> Result<Expression, ParserError> {
        expect_peek!(self, LParen)?;

        let parameters = self.parse_function_params()?;

        expect_peek!(self, LBrace)?;

        let body = self.parse_block_statement()?;

        Ok(Expression::function(parameters, body))
    }

    fn parse_function_params(&mut self) -> Result<Vec<Expression>, ParserError> {
        let mut params = vec![];

        if self.peeking_token == Token::RParen {
            self.next_token();
            return Ok(params);
        }

        self.next_token();

        while let Token::Identifier(identifier) = &self.current_token {
            params.push(Expression::identifier(identifier));

            self.next_token();
            if let Token::Comma = self.current_token {
                self.next_token();
            }
        }

        Ok(params)
    }

    fn parse_if_expression(&mut self) -> Result<Expression, ParserError> {
        expect_peek!(self, LParen)?;

        self.next_token();

        let condition = self.parse_expression(Precedence::LOWEST)?;

        expect_peek!(self, RParen)?;

        expect_peek!(self, LBrace)?;

        let consequence = self.parse_block_statement()?;

        let mut alternative: Option<Vec<Statement>> = None;

        if self.peeking_token == Token::Else {
            self.next_token();

            expect_peek!(self, LBrace)?;

            alternative = Some(self.parse_block_statement()?);
        }

        Ok(Expression::r#if(condition, consequence, alternative))
    }

    fn parse_block_statement(&mut self) -> Result<Vec<Statement>, ParserError> {
        self.next_token();

        let mut statements = vec![];

        while self.current_token != Token::RBrace && self.current_token != Token::EOF {
            let statement = self.parse_statement()?;
            statements.push(statement);
            self.next_token();
        }

        Ok(statements)
    }

    fn parse_grouped_expression(&mut self) -> Result<Expression, ParserError> {
        self.next_token();

        let expression = self.parse_expression(Precedence::LOWEST);

        expect_peek!(self, RParen)?;

        expression
    }

    fn parse_prefix_expression(&mut self) -> Result<Expression, ParserError> {
        let operator = match &self.current_token {
            Token::Bang => PrefixOperator::Not,
            Token::Minus => PrefixOperator::Negative,
            _ => return Err(ParserError {}),
        };

        self.next_token();

        self.parse_expression(Precedence::PREFIX)
            .map(|expression| Expression::prefix(expression, operator))
            .map_err(|_| ParserError {})
    }

    fn parse_infix(&mut self, lhs: Expression) -> Result<Expression, ParserError> {
        let precedence = Precedence::from(&self.current_token);

        let operator = match &self.current_token {
            Token::Eq => InfixOperator::Equal,
            Token::NotEq => InfixOperator::NotEqual,
            Token::Plus => InfixOperator::Add,
            Token::Minus => InfixOperator::Sub,
            Token::Asterisk => InfixOperator::Mult,
            Token::Slash => InfixOperator::Div,
            Token::GT => InfixOperator::GreaterThan,
            Token::LT => InfixOperator::LessThan,
            Token::LParen => return self.parse_call_expression(lhs),
            _ => return Err(ParserError {}),
        };

        self.next_token();

        let rhs = self.parse_expression(precedence);

        match rhs {
            Ok(rhs) => Ok(Expression::infix(lhs, rhs, operator)),
            Err(_) => Err(ParserError {}),
        }
    }

    fn parse_boolean(&self) -> Result<Expression, ParserError> {
        match &self.current_token {
            Token::True => Ok(Expression::Bool(true)),
            Token::False => Ok(Expression::Bool(false)),
            _ => Err(ParserError {}),
        }
    }

    fn parse_integer(&self, literal: &String) -> Result<Expression, ParserError> {
        literal
            .parse()
            .map(Expression::Int)
            .map_err(|_| ParserError {})
    }

    fn parse_let_statement(&mut self) -> Result<Statement, ParserError> {
        self.next_token();

        let identifier = match &self.current_token {
            Token::Identifier(identifier) => identifier.clone(),
            _ => return Err(ParserError {}),
        };

        expect_peek!(self, Assign)?;

        self.next_token();

        let expression = self.parse_expression(Precedence::LOWEST)?;

        Ok(Statement::r#let(identifier, expression))
    }

    fn parse_return_statement(&mut self) -> Result<Statement, ParserError> {
        self.next_token();

        let expression = self.parse_expression(Precedence::LOWEST)?;

        Ok(Statement::r#return(expression))
    }

    fn next_token(&mut self) {
        std::mem::swap(&mut self.current_token, &mut self.peeking_token);
        self.peeking_token = self.lexer.next_token();
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use crate::{
        ast::{
            expression::Expression,
            operator::{InfixOperator, PrefixOperator},
            statement::Statement,
        },
        lexer::Lexer,
        token::Token,
    };

    use super::Parser;

    #[test]
    fn test_call_expression_parsing() {
        let mut parser = make_parser("add(1, 2 * 3, 4 + 5, 6 / 2);");
        let program = parser.parse_program();

        assert_eq!(parser.errors.len(), 0);
        assert_eq!(program.statements.len(), 1);
        assert_eq!(
            program.statements[0],
            Statement::Expression(Expression::call(
                Expression::identifier("add"),
                vec![
                    Expression::Int(1),
                    Expression::infix(Expression::Int(2), Expression::Int(3), InfixOperator::Mult),
                    Expression::infix(Expression::Int(4), Expression::Int(5), InfixOperator::Add),
                    Expression::infix(Expression::Int(6), Expression::Int(2), InfixOperator::Div)
                ]
            ))
        )
    }

    #[test]
    fn test_function_literal_parsing() {
        let mut parser = make_parser(indoc! {"
            fn(x, y) {
                x + y;
            }
        "});
        let program = parser.parse_program();

        assert_eq!(parser.errors.len(), 0);
        assert_eq!(program.statements.len(), 1);

        assert_eq!(
            program.statements[0],
            Statement::Expression(Expression::function(
                vec![Expression::identifier("x"), Expression::identifier("y")],
                vec![Statement::expression(Expression::infix(
                    Expression::identifier("x"),
                    Expression::identifier("y"),
                    InfixOperator::Add,
                ))]
            ))
        );
    }

    #[test]
    fn test_parsing_infix_expressions_with_integers() {
        let mut parser = make_parser(indoc! {"
            5 + 5;
            5 - 5;
            5 * 5;
            5 / 5;
            5 > 5;
            5 < 5;
            5 == 5;
            5 != 5;
        "});
        let program = parser.parse_program();

        assert_eq!(parser.errors.len(), 0);
        assert_eq!(program.statements.len(), 8);

        macro_rules! infix_assert {
            ($index:expr, $op:expr) => {
                assert_eq!(
                    program.statements[$index],
                    Statement::expression(Expression::infix(
                        Expression::Int(5),
                        Expression::Int(5),
                        $op,
                    ))
                )
            };
        }
        infix_assert!(0, InfixOperator::Add);
        infix_assert!(1, InfixOperator::Sub);
        infix_assert!(2, InfixOperator::Mult);
        infix_assert!(3, InfixOperator::Div);
        infix_assert!(4, InfixOperator::GreaterThan);
        infix_assert!(5, InfixOperator::LessThan);
        infix_assert!(6, InfixOperator::Equal);
        infix_assert!(7, InfixOperator::NotEqual);
    }

    #[test]
    fn test_parsing_infix_with_multiple_expressions() {
        let mut parser = make_parser(indoc! {"
            5 + 7 * 10;
            1 - 2 + 3;
            5 * 7 + 10;
        "});
        let program = parser.parse_program();

        assert_eq!(parser.errors.len(), 0);
        assert_eq!(program.statements.len(), 3);
        assert_eq!(
            program.statements[0],
            Statement::Expression(Expression::infix(
                Expression::Int(5),
                Expression::infix(Expression::Int(7), Expression::Int(10), InfixOperator::Mult),
                InfixOperator::Add
            ))
        );
        assert_eq!(
            program.statements[1],
            Statement::Expression(Expression::infix(
                Expression::infix(Expression::Int(1), Expression::Int(2), InfixOperator::Sub),
                Expression::Int(3),
                InfixOperator::Add
            ))
        );
        assert_eq!(
            program.statements[2],
            Statement::Expression(Expression::infix(
                Expression::infix(Expression::Int(5), Expression::Int(7), InfixOperator::Mult),
                Expression::Int(10),
                InfixOperator::Add
            ))
        );
    }

    #[test]
    fn test_new_with_empty_input() {
        let parser = make_parser("");

        assert_eq!(parser.current_token, Token::EOF);
        assert_eq!(parser.peeking_token, Token::EOF);
    }

    #[test]
    fn test_new_with_single_token_input() {
        let parser = make_parser(";");

        assert_eq!(parser.current_token, Token::Semicolon);
        assert_eq!(parser.peeking_token, Token::EOF);
    }

    #[test]
    fn test_new_with_multiple_tokens_input() {
        let parser = make_parser("let five = 5;");

        assert_eq!(parser.current_token, Token::Let);
        assert_eq!(
            parser.peeking_token,
            Token::Identifier(String::from("five"))
        );
    }

    #[test]
    fn test_next_token() {
        let mut parser = make_parser("let five = 5;");

        assert_eq!(parser.current_token, Token::Let);
        assert_eq!(parser.peeking_token, Token::identifier("five"));

        parser.next_token();
        assert_eq!(parser.current_token, Token::identifier("five"));
        assert_eq!(parser.peeking_token, Token::Assign);

        parser.next_token();
        assert_eq!(parser.current_token, Token::Assign);
        assert_eq!(parser.peeking_token, Token::integer("5"));

        parser.next_token();
        assert_eq!(parser.current_token, Token::integer("5"));
        assert_eq!(parser.peeking_token, Token::Semicolon);
    }

    #[test]
    fn test_if_expression() {
        let mut parser = make_parser(indoc! {"
            if (x < y) { x }
        "});
        let program = parser.parse_program();

        assert_eq!(parser.errors.len(), 0);
        assert_eq!(program.statements.len(), 1);

        assert_eq!(
            program.statements[0],
            Statement::Expression(Expression::r#if(
                Expression::infix(
                    Expression::identifier("x"),
                    Expression::identifier("y"),
                    InfixOperator::LessThan,
                ),
                vec![Statement::Expression(Expression::identifier("x"))],
                None
            ))
        )
    }

    #[test]
    fn test_if_else_expression() {
        let mut parser = make_parser(indoc! {"
            if (x < y) { x } else { y }
        "});
        let program = parser.parse_program();

        assert_eq!(parser.errors.len(), 0);
        assert_eq!(program.statements.len(), 1);

        assert_eq!(
            program.statements[0],
            Statement::Expression(Expression::r#if(
                Expression::infix(
                    Expression::identifier("x"),
                    Expression::identifier("y"),
                    InfixOperator::LessThan,
                ),
                vec![Statement::Expression(Expression::identifier("x"))],
                Some(vec![Statement::Expression(Expression::identifier("y"))])
            ))
        )
    }

    #[test]
    fn test_parse_let_statement() {
        let mut parser = make_parser(indoc! {"
            let x = 5;
            let y = 10;
            let banana = 123456;
        "});

        let program = parser.parse_program();

        assert_eq!(parser.errors.len(), 0);
        assert_eq!(program.statements.len(), 3);

        assert_eq!(
            program.statements[0],
            Statement::r#let("x", Expression::Int(5))
        );
        assert_eq!(
            program.statements[1],
            Statement::r#let("y", Expression::Int(10))
        );
        assert_eq!(
            program.statements[2],
            Statement::r#let("banana", Expression::Int(123456))
        );
    }

    #[test]
    fn test_parse_return_statement() {
        let mut parser = make_parser(indoc! {"
            return banana;
            return 69 + 420;
        "});

        let program = parser.parse_program();

        assert_eq!(program.statements.len(), 2);
        assert_eq!(parser.errors.len(), 0);

        assert_eq!(
            program.statements[0],
            Statement::r#return(Expression::identifier("banana"))
        );
        assert_eq!(
            program.statements[1],
            Statement::r#return(Expression::infix(
                Expression::Int(69),
                Expression::Int(420),
                InfixOperator::Add
            ))
        );
    }

    #[test]
    fn test_identifier_expression() {
        let mut parser = make_parser(indoc! {"
            banana;
            apple;
        "});
        let program = parser.parse_program();

        assert_eq!(parser.errors.len(), 0);
        assert_eq!(program.statements.len(), 2);
        assert_eq!(
            program.statements[0],
            Statement::expression(Expression::identifier("banana"))
        );
        assert_eq!(
            program.statements[1],
            Statement::expression(Expression::identifier("apple"))
        );
    }

    #[test]
    fn test_integer_literal_expression() {
        let mut parser = make_parser(indoc! {"
            123;
            456;
        "});
        let program = parser.parse_program();

        assert_eq!(parser.errors.len(), 0);
        assert_eq!(program.statements.len(), 2);
        assert_eq!(
            program.statements[0],
            Statement::expression(Expression::Int(123))
        );
        assert_eq!(
            program.statements[1],
            Statement::expression(Expression::Int(456))
        );
    }

    #[test]
    fn test_prefix_operators() {
        let mut parser = make_parser(indoc! {"
            !5;
            -15;
        "});
        let program = parser.parse_program();

        assert_eq!(parser.errors.len(), 0);
        assert_eq!(program.statements.len(), 2);
        assert_eq!(
            program.statements[0],
            Statement::expression(Expression::prefix(Expression::Int(5), PrefixOperator::Not))
        );
        assert_eq!(
            program.statements[1],
            Statement::expression(Expression::prefix(
                Expression::Int(15),
                PrefixOperator::Negative
            ))
        );
    }

    fn make_parser(input: impl Into<String>) -> Parser {
        let input = input.into();
        let lexer = Lexer::new(&input);
        let parser = Parser::new(lexer);
        return parser;
    }

    #[test]
    fn test_precedences() {
        let tests = vec![
            ("-a * b", "((-a) * b)"),
            ("!-a", "(!(-a))"),
            ("a + b + c", "((a + b) + c)"),
            ("a + b - c", "((a + b) - c)"),
            ("a * b * c", "((a * b) * c)"),
            ("a * b / c", "((a * b) / c)"),
            ("a + b / c", "(a + (b / c))"),
            ("a + b * c + d / e - f", "(((a + (b * c)) + (d / e)) - f)"),
            ("3 + 4; -5 * 5", "(3 + 4)\n((-5) * 5)"),
            ("5 > 4 == 3 < 4", "((5 > 4) == (3 < 4))"),
            ("5 < 4 != 3 > 4", "((5 < 4) != (3 > 4))"),
            (
                "3 + 4 * 5 == 3 * 1 + 4 * 5",
                "((3 + (4 * 5)) == ((3 * 1) + (4 * 5)))",
            ),
            ("true", "true"),
            ("false", "false"),
            ("3 > 5 == false", "((3 > 5) == false)"),
            ("3 < 5 == true", "((3 < 5) == true)"),
            ("1 + (2 + 3) + 4", "((1 + (2 + 3)) + 4)"),
            ("(5 + 5) * 2", "((5 + 5) * 2)"),
            ("2 / (5 + 5)", "(2 / (5 + 5))"),
            ("-(5 + 5)", "(-(5 + 5))"),
            ("!(true == true)", "(!(true == true))"),
        ];

        for test in tests {
            let mut parser = make_parser(test.0);
            let program = parser.parse_program();
            assert_eq!(parser.errors.len(), 0);
            assert_eq!(program.to_string().trim(), test.1);
        }
    }
}
