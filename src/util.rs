use std::io;

pub fn handle_io<T>(result: io::Result<T>, message: &str) -> Option<T>
{
    match result
    {
        Ok(ok) => { Some(ok) },
        Err(ref error) =>
        {
            error!("{}: {}", message, error);
            None
        },
    }
}
