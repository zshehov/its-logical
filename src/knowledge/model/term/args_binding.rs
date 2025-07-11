use nom::{
    bytes::complete::{tag, take_till1},
    error::VerboseError,
    multi::separated_list1,
    IResult,
};

#[derive(Debug, PartialEq, Clone, Eq, Hash)]
pub struct ArgsBinding {
    pub binding: Vec<String>,
}

impl ArgsBinding {
    pub fn new(binding: &[String]) -> Self {
        Self {
            binding: binding.to_vec(),
        }
    }
    pub fn encode(&self) -> String {
        self.binding.join(",")
    }
}

pub fn parse_args_binding(i: &str) -> IResult<&str, ArgsBinding, VerboseError<&str>> {
    separated_list1(tag(","), take_till1(is_end_of_args))(i).map(|(leftover, args)| {
        (
            leftover,
            ArgsBinding {
                binding: args.into_iter().map(|x| x.to_string()).collect(),
            },
        )
    })
}

fn is_end_of_args(c: char) -> bool {
    c == ',' || c == ')'
}
