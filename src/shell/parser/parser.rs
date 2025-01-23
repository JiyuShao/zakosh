use super::ast::{Command, Node, Redirection};
use super::lexer::{Lexer, RedirectOp, Token};

pub struct Parser<'a> {
    lexer: Lexer<'a>,
    current_token: Token,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        let mut lexer = Lexer::new(input);
        let current_token = lexer.next_token();
        Parser {
            lexer,
            current_token,
        }
    }

    fn next_token(&mut self) {
        self.current_token = self.lexer.next_token();
    }

    pub fn parse_command(&mut self) -> Result<Node, String> {
        let mut commands = Vec::new();

        while self.current_token != Token::EOF {
            let cmd = self.parse_simple_command()?;
            commands.push(cmd);

            match self.current_token {
                Token::Pipe => {
                    self.next_token();
                    continue;
                }
                Token::Semi => {
                    self.next_token();
                    break;
                }
                _ => break,
            }
        }

        Ok(if commands.len() == 1 {
            Node::Command(commands.pop().unwrap_or_default())
        } else {
            Node::Pipeline(commands)
        })
    }

    fn parse_simple_command(&mut self) -> Result<Command, String> {
        let mut command = Command {
            program: String::new(),
            arguments: Vec::new(),
            redirections: Vec::new(),
            background: false,
        };

        // 解析命令名
        match &self.current_token {
            Token::Word(word) => {
                command.program = word.clone();
                self.next_token();
            }
            _ => return Err("Expected command name".to_string()),
        }

        // 解析参数和重定向
        loop {
            match &self.current_token {
                Token::EOF | Token::Pipe | Token::Semi => break,
                Token::Background => {
                    command.background = true;
                    self.next_token();
                    break;
                }
                Token::Redirect(op) => {
                    let redirection = self.parse_redirection(op.clone())?;
                    command.redirections.push(redirection);
                }
                Token::Word(word) => {
                    command.arguments.push(word.clone());
                    self.next_token();
                }
            }
        }

        Ok(command)
    }

    fn parse_redirection(&mut self, operator: RedirectOp) -> Result<Redirection, String> {
        self.next_token(); // 跳过重定向操作符

        match &self.current_token {
            Token::Word(filename) => {
                let redirection = Redirection {
                    operator,
                    filename: filename.clone(),
                };
                self.next_token();
                Ok(redirection)
            }
            _ => Err("Expected filename after redirection operator".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::unwrap_used)]
    #[test]
    fn test_simple_command() {
        let mut parser = Parser::new("ls -l");
        let node = parser.parse_command().unwrap();

        match node {
            Node::Command(cmd) => {
                assert_eq!(cmd.program, "ls");
                assert_eq!(cmd.arguments, vec!["-l"]);
                assert!(cmd.redirections.is_empty());
                assert!(!cmd.background);
            }
            _ => panic!("Expected simple command"),
        }
    }

    #[allow(clippy::unwrap_used)]
    #[test]
    fn test_pipeline() {
        let mut parser = Parser::new("ls -l | grep foo");
        let node = parser.parse_command().unwrap();

        match node {
            Node::Pipeline(cmds) => {
                assert_eq!(cmds.len(), 2);
                assert_eq!(cmds[0].program, "ls");
                assert_eq!(cmds[0].arguments, vec!["-l"]);
                assert_eq!(cmds[1].program, "grep");
                assert_eq!(cmds[1].arguments, vec!["foo"]);
            }
            _ => panic!("Expected pipeline"),
        }
    }

    #[allow(clippy::unwrap_used)]
    #[test]
    fn test_redirection() {
        let mut parser = Parser::new("echo hello > output.txt");
        let node = parser.parse_command().unwrap();

        match node {
            Node::Command(cmd) => {
                assert_eq!(cmd.program, "echo");
                assert_eq!(cmd.arguments, vec!["hello"]);
                assert_eq!(cmd.redirections.len(), 1);
                assert_eq!(cmd.redirections[0].filename, "output.txt");
                assert!(matches!(cmd.redirections[0].operator, RedirectOp::Output));
            }
            _ => panic!("Expected command with redirection"),
        }
    }

    #[allow(clippy::unwrap_used)]
    #[test]
    fn test_background() {
        let mut parser = Parser::new("sleep 10 &");
        let node = parser.parse_command().unwrap();

        match node {
            Node::Command(cmd) => {
                assert_eq!(cmd.program, "sleep");
                assert_eq!(cmd.arguments, vec!["10"]);
                assert!(cmd.background);
            }
            _ => panic!("Expected background command"),
        }
    }
}
