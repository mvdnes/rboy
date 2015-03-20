use std::io;

pub fn handle_io<T>(result: io::Result<T>, message: &'static str) -> ::StrResult<T>
{
    result.map_err(|_| message)
}
