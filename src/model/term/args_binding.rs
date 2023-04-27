use nom::{
    bytes::complete::{tag, take_till1},
    error::VerboseError,
    multi::separated_list1,
    IResult,
};

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct ArgsBinding {
    pub(crate) binding: Vec<Option<String>>,
}

pub(crate) fn parse_args_binding<'a>(
    i: &'a str,
) -> IResult<&'a str, ArgsBinding, VerboseError<&str>> {
    separated_list1(tag(","), take_till1(is_end_of_args))(i).map(|(leftover, args)| {
        (
            leftover,
            ArgsBinding {
                binding: args
                    .iter()
                    .map(|&s| {
                        if s == "_" {
                            return None;
                        }
                        return Some(s.to_string());
                    })
                    .collect(),
            },
        )
    })
}

fn is_end_of_args(c: char) -> bool {
    c == ',' || c == ')'
}
