
pub fn run_main(main: fn() -> Result<i32>) {
    match main() {
        Err(message) => {
            eprintln!("{}", message);
            std::process::exit(1);
        }
        Ok(code) => std::process::exit(code),
    }
}

pub struct Error {
    message: String,
}

pub type Result<T> = std::result::Result<T, Error>;

impl<'a> From<&'a str> for Error {
    fn from(message: &'a str) -> Error {
        Error { message: message.to_owned() }
    }
}

impl From<String> for Error {
    fn from(message: String) -> Error {
        Error { message }
    }
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}
