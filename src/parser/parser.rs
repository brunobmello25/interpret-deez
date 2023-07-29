use crate::{
    ast::{expression::Expression, program::Program, statement::Statement},
    lexer::Lexer,
    parser::precedence::Precedence,
    token::Token,
};

pub struct ParserError {}

pub struct Parser {
    lexer: Lexer,
    current_token: Token,
    peeking_token: Token,
    errors: Vec<ParserError>,
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
            let stmt = match self.current_token {
                Token::Let => self.parse_let_statement(),
                Token::Return => self.parse_return_statement(),
                _ => self.parse_expression_statement(),
            };

            match stmt {
                Ok(stmt) => program.statements.push(stmt),
                Err(err) => self.errors.push(err),
            }

            self.advance_tokens();
        }

        program
    }

    fn parse_expression_statement(&mut self) -> Result<Statement, ParserError> {
        let expression = self.parse_expression(Precedence::LOWEST)?;

        if self.peeking_token == Token::Semicolon {
            self.next_token();
        };

        Ok(Statement::expression(expression))
    }

    fn parse_expression(&mut self, precedence: Precedence) -> Result<Expression, ParserError> {
        let prefix = self.parse_prefix()?;

        Ok(prefix)
    }

    fn advance_tokens(&mut self) {
        while self.current_token != Token::Semicolon && self.current_token != Token::EOF {
            self.next_token();
        }

        if self.current_token == Token::Semicolon {
            self.next_token();
        }
    }

    fn parse_prefix(&self) -> Result<Expression, ParserError> {
        let expression = match &self.current_token {
            Token::Identifier(identifier) => Expression::identifier(identifier),
            _ => todo!(),
        };

        Ok(expression)
    }

    fn parse_let_statement(&mut self) -> Result<Statement, ParserError> {
        let let_token = self.current_token.clone();

        self.next_token();

        let identifier = match &self.current_token {
            Token::Identifier(identifier) => identifier.clone(),
            _ => return Err(ParserError {}),
        };

        self.next_token();

        // TODO: parse expression here
        while self.current_token != Token::Semicolon && self.current_token != Token::EOF {
            self.next_token();
        }

        Ok(Statement::Let { name: identifier })
    }

    fn parse_return_statement(&mut self) -> Result<Statement, ParserError> {
        let return_token = self.current_token.clone();

        self.next_token();

        // TODO: parse expressions here
        while self.current_token != Token::Semicolon && self.current_token != Token::EOF {
            self.next_token();
        }

        Ok(Statement::r#return(return_token))
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
        ast::{expression::Expression, statement::Statement},
        lexer::Lexer,
        token::Token,
    };

    use super::Parser;

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
        assert_eq!(parser.peeking_token, Token::Integer(5));

        parser.next_token();
        assert_eq!(parser.current_token, Token::Integer(5));
        assert_eq!(parser.peeking_token, Token::Semicolon);
    }

    #[test]
    fn test_parse_single_let_statement() {
        let mut parser = make_parser("let x = banana;");

        let program = parser.parse_program();

        assert_eq!(program.statements.len(), 1);
        assert_eq!(parser.errors.len(), 0);

        assert_eq!(program.statements[0], Statement::r#let(Token::Let, "x"))
    }

    #[test]
    fn test_parse_let_statement() {
        let mut parser = make_parser(indoc! {"
            let x = 5;
            let y = 10;
            let banana = 123456;
        "});

        let program = parser.parse_program();

        assert_eq!(program.statements.len(), 3);
        assert_eq!(parser.errors.len(), 0);

        //TODO: assert that the statement expressions are correct
        assert_eq!(program.statements[0], Statement::r#let(Token::Let, "x"));
        assert_eq!(program.statements[1], Statement::r#let(Token::Let, "y"));
        assert_eq!(
            program.statements[2],
            Statement::r#let(Token::Let, "banana")
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

        // TODO: assert that the statement expressions are correct
        assert_eq!(program.statements[0], Statement::r#return(Token::Return));
        assert_eq!(program.statements[1], Statement::r#return(Token::Return));
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

    fn make_parser(input: impl Into<String>) -> Parser {
        let input = input.into();
        let lexer = Lexer::new(&input);
        let parser = Parser::new(lexer);
        return parser;
    }
}