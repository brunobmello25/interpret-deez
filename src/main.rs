use repl::Repl;

mod ast;
mod lexer;
mod repl;
mod token;

fn main() {
    println!("Monkey repl! enter empty string to exit");

    Repl::start(std::io::stdin());
}
