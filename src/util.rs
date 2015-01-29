use std::old_io::IoResult;

pub fn handle_io<T>(result: IoResult<T>, message: &str) -> Option<T>
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
