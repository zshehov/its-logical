use std::{cell::RefCell, rc::Rc};

use scryer_prolog::machine::Machine;

use crate::knowledge::model::term::args_binding::ArgsBinding;
use crate::knowledge::model::term::bound_term::BoundTerm;

pub trait Engine {
    fn ask(&mut self, name: &str, args: &[String]) -> Rc<RefCell<ConsultResult>>;
}

pub struct ConsultResult {
    size: usize,
}

impl ConsultResult {
    pub fn new(size: usize) -> Self {
        Self { size }
    }
}

impl ConsultResult {
    pub fn more(&mut self) -> Option<Vec<String>> {
        let mut res = Vec::with_capacity(self.size);
        for i in 0..self.size {
            res.push(format!("Eval numero {}", i));
        }
        Some(res)
    }
}

impl Engine for Machine {
    fn ask(&mut self, name: &str, args: &[String]) -> Rc<RefCell<ConsultResult>> {
        let term = BoundTerm::new(name, ArgsBinding::new(args));
        let result = self.run_query(term.encode());

        Rc::new(RefCell::new(ConsultResult::new(args.len())))
    }
}
