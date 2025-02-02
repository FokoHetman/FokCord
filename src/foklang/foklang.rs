use crate::{foklang::core};
use std::{
  cmp, env, fs, io::{self, Read, Write}
};
use crate::Fokcord;

#[derive(Clone,Debug,PartialEq)]
pub struct Foklang {
  tokenizer: core::tokenizer::Tokenizer,
  parser: core::parser::Parser,
  interpreter: core::interpreter::Interpreter,
  pub env: core::env::Environment,
}
impl Foklang {
  pub fn new() -> Self {
    let tokenizer = core::tokenizer::Tokenizer {};
    let mut parser = core::parser::Parser {};
    let error_handler = core::error_handler::ErrorHandler {};
    let mut env = core::env::Environment{ error_handler, ..Default::default() };
    core::builtins::declare_builtins(&mut env);
    let mut interpreter = core::interpreter::Interpreter {error_handler, tokenizer, parser};

    return Foklang{tokenizer, parser, interpreter, env}
  }
  pub fn run(&mut self, input: String, program: Fokcord) -> (Fokcord,String) {
    let mut tokenized_input = self.tokenizer.tokenize(input);
    let mut parsed_input = self.parser.parse(tokenized_input);
    let mut interpreted_input = self.interpreter.evaluate(parsed_input, &mut self.env, program.clone());

    let value = interpreted_input.value;
    match value {
      core::AST::Fructa::FokcordModifier(nprogram) => {
        (nprogram.clone(), nprogram.io)
      }
      _ => {
        (program,value.display())
      }
    }
  }
  pub fn raw_run(&mut self, input: String, program: Fokcord) -> core::AST::Proventus {
    self.interpreter.evaluate(self.parser.parse(self.tokenizer.tokenize(input)), &mut self.env, program.clone())
  }
}

pub fn run(input: String, program: crate::Fokcord) -> String {
  let tokenizer = core::tokenizer::Tokenizer {};
  let mut parser = core::parser::Parser {};
  let error_handler = core::error_handler::ErrorHandler {};
  let mut env = core::env::Environment{ error_handler, ..Default::default() };
  core::builtins::declare_builtins(&mut env);
  let mut interpreter = core::interpreter::Interpreter {error_handler, tokenizer, parser};


  let mut tokenized_input = tokenizer.tokenize(input);
  let mut parsed_input = parser.parse(tokenized_input);
  let mut interpreted_input = interpreter.evaluate(parsed_input, &mut env, program);

  interpreted_input.value.display()
}
