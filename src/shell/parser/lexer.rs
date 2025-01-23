use std::iter::Peekable;
use std::str::Chars;

#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    Word(String),
    Pipe,
    Redirect(RedirectOp),
    Background,
    Semi,
    EOF,
}

#[derive(Debug, PartialEq, Clone)]
pub enum RedirectOp {
    Input,  // <
    Output, // >
    Append, // >>
}

pub struct Lexer<'a> {
    input: Peekable<Chars<'a>>,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input: input.chars().peekable(),
        }
    }

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace();

        match self.peek_char() {
            None => Token::EOF,
            Some(c) => match c {
                '|' => {
                    self.read_char();
                    Token::Pipe
                }
                ';' => {
                    self.read_char();
                    Token::Semi
                }
                '&' => {
                    self.read_char();
                    Token::Background
                }
                '<' => {
                    self.read_char();
                    Token::Redirect(RedirectOp::Input)
                }
                '>' => {
                    self.read_char();
                    if self.peek_char() == Some('>') {
                        self.read_char();
                        Token::Redirect(RedirectOp::Append)
                    } else {
                        Token::Redirect(RedirectOp::Output)
                    }
                }
                '"' => self.read_quoted_string(),
                '\'' => self.read_quoted_string(),
                _ => self.read_word(),
            },
        }
    }

    fn read_char(&mut self) -> Option<char> {
        self.input.next()
    }

    fn peek_char(&mut self) -> Option<char> {
        self.input.peek().copied()
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek_char() {
            if !c.is_whitespace() {
                break;
            }
            self.read_char();
        }
    }

    fn read_word(&mut self) -> Token {
        let mut word = String::new();

        while let Some(c) = self.peek_char() {
            if c.is_whitespace() || ";<>|&".contains(c) {
                break;
            }
            word.push(self.read_char().unwrap_or_default());
        }

        Token::Word(word)
    }

    fn read_quoted_string(&mut self) -> Token {
        let quote = self.read_char().unwrap_or_default();
        let mut string = String::new();
        let mut escaped = false;

        while let Some(c) = self.read_char() {
            match (escaped, c) {
                (true, _) => {
                    string.push(c);
                    escaped = false;
                }
                (false, '\\') => escaped = true,
                (false, c) if c == quote => break,
                (false, c) => string.push(c),
            }
        }

        Token::Word(string)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_command() {
        let mut lexer = Lexer::new("ls -l");
        assert_eq!(lexer.next_token(), Token::Word("ls".to_string()));
        assert_eq!(lexer.next_token(), Token::Word("-l".to_string()));
        assert_eq!(lexer.next_token(), Token::EOF);
    }

    #[test]
    fn test_pipe() {
        let mut lexer = Lexer::new("ls | grep foo");
        assert_eq!(lexer.next_token(), Token::Word("ls".to_string()));
        assert_eq!(lexer.next_token(), Token::Pipe);
        assert_eq!(lexer.next_token(), Token::Word("grep".to_string()));
        assert_eq!(lexer.next_token(), Token::Word("foo".to_string()));
        assert_eq!(lexer.next_token(), Token::EOF);
    }

    #[test]
    fn test_redirections() {
        let mut lexer = Lexer::new("echo hello > output.txt");
        assert_eq!(lexer.next_token(), Token::Word("echo".to_string()));
        assert_eq!(lexer.next_token(), Token::Word("hello".to_string()));
        assert_eq!(lexer.next_token(), Token::Redirect(RedirectOp::Output));
        assert_eq!(lexer.next_token(), Token::Word("output.txt".to_string()));
        assert_eq!(lexer.next_token(), Token::EOF);
    }

    #[test]
    fn test_quoted_strings() {
        let mut lexer = Lexer::new(r#"echo "hello world" 'foo bar'"#);
        assert_eq!(lexer.next_token(), Token::Word("echo".to_string()));
        assert_eq!(lexer.next_token(), Token::Word("hello world".to_string()));
        assert_eq!(lexer.next_token(), Token::Word("foo bar".to_string()));
        assert_eq!(lexer.next_token(), Token::EOF);
    }
}
