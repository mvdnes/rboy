use std::io::{IoResult,IoError};

pub fn handle_io<T>(result: IoResult<T>, message: &str, fail: bool) -> Result<T, IoError>
{
	match result
	{
		Ok(_) => {},
		Err(ref error) =>
		{
			if fail { fail!("{:s}: {}", message, error) }
			else { error!("{:s}: {}", message, error) }
		},
	};
	result
}
