use std::{error, fmt};

// TODO: remove ugliness of this workaround ...
type ArgsError1 = ::hlua51::LuaFunctionCallError<
    ::hlua51::TuplePushError<
        ::hlua51::Void,
        ::hlua51::TuplePushError<::hlua51::Void, ::hlua51::Void>,
    >,
>;
type ArgsError2 =
    ::hlua51::LuaFunctionCallError<::hlua51::TuplePushError<::hlua51::Void, ::hlua51::Void>>;

#[derive(Debug)]
pub enum Error {
    Lua(::hlua51::LuaError),
    LuaFunctionCall(::hlua51::LuaFunctionCallError<::hlua51::Void>),
    ArgsPush(ArgsError1),
    GetPluginArgs(ArgsError2),
    // TODO: improve by including information about the global/key that was not defined
    Undefined(String),
    GcloudAccessKeyMissing,
    AmazonAccessKeyMissing,
    AmazonSecretKeyMissing,
    AmazonRegionMissing,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::Error::*;
        use std::error::Error;

        match self {
            Undefined(key) => write!(
                f,
                "Error: Trying to access undefined lua global or table key: {}",
                key
            )?,
            _ => write!(f, "Error: {}", self.description())?,
        }

        let mut cause: Option<&dyn error::Error> = self.source();
        while let Some(err) = cause {
            write!(f, "  -> {}", err)?;
            cause = err.source();
        }

        Ok(())
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        use self::Error::*;

        match *self {
            Lua(_) => "Lua error",
            LuaFunctionCall(_) => "Error calling Lua function",
            ArgsPush(_) => "Error pushing Lua function arguments",
            GetPluginArgs(_) => "Error pushing Lua function arguments for OptionsData.getPlugin",
            Undefined(_) => "Trying to access lua gobal or table key that does not exist",
            GcloudAccessKeyMissing => "Google Cloud Access key is not set",
            AmazonAccessKeyMissing => "Amazon access key is not set",
            AmazonSecretKeyMissing => "Amazon secret key is not set",
            AmazonRegionMissing => "Amazon region is not set",
        }
    }

    fn cause(&self) -> Option<&dyn error::Error> {
        use self::Error::*;

        match *self {
            Lua(ref err) => Some(err),
            LuaFunctionCall(ref err) => Some(err),
            _ => None,
        }
    }
}

impl From<::hlua51::LuaError> for Error {
    fn from(err: ::hlua51::LuaError) -> Self {
        Error::Lua(err)
    }
}

impl From<::hlua51::LuaFunctionCallError<::hlua51::Void>> for Error {
    fn from(err: ::hlua51::LuaFunctionCallError<::hlua51::Void>) -> Self {
        Error::LuaFunctionCall(err)
    }
}

impl From<ArgsError1> for Error {
    fn from(err: ArgsError1) -> Self {
        Error::ArgsPush(err)
    }
}

impl From<ArgsError2> for Error {
    fn from(err: ArgsError2) -> Self {
        Error::GetPluginArgs(err)
    }
}
