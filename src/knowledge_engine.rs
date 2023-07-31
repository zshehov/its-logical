use std::{cell::RefCell, rc::Rc};

pub(crate) trait Engine {
    fn ask(&mut self, name: &str, args: &[String]) -> Rc<RefCell<ConsultResult>>;
}
pub(crate) struct ConsultResult {
    size: usize,
}

impl ConsultResult {
    pub(crate) fn new(size: usize) -> Self {
        Self { size }
    }
}

impl ConsultResult {
    pub(crate) fn more(&mut self) -> Option<Vec<String>> {
        let mut res = Vec::with_capacity(self.size);
        for i in 0..self.size {
            res.push(format!("Eval numero {}", i));
        }
        Some(res)
    }
}

pub(crate) struct DummyEngine {}

impl Engine for DummyEngine {
    fn ask(&mut self, _: &str, args: &[String]) -> Rc<RefCell<ConsultResult>> {
        Rc::new(RefCell::new(ConsultResult::new(args.len())))
    }
}
