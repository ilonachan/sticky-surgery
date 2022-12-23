use std::error::Error as StdError;
use std::fmt;
use std::result::Result as StdResult;
use sea_orm::DbErr;
use serenity::Error as SerenityError;
use tracing::instrument;

pub type Result<T> = StdResult<T, Error>;

#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// An error from the `serenity` crate
    Serenity(SerenityError),
    /// An error from the `sea-orm` crate
    SeaOrm(DbErr),
    /// Generic error message
    Other(String),
}

impl From<SerenityError> for Error {
  fn from(e: SerenityError) -> Self {
    Error::Serenity(e)
  }
}

impl From<DbErr> for Error {
  fn from(e: DbErr) -> Self {
    Error::SeaOrm(e)
  }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Other(msg) => f.write_str(msg),
            // Self::ExceededLimit(..) => f.write_str("Input exceeded a limit"),
            // Self::NotInRange(..) => f.write_str("Input is not in the specified range"),
            Self::Serenity(inner) => fmt::Display::fmt(&inner, f),
            Self::SeaOrm(inner) => fmt::Display::fmt(&inner, f),
        }
    }
}

impl StdError for Error {
    #[instrument]
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Serenity(inner) => Some(inner),
            Self::SeaOrm(inner) => Some(inner),
            _ => None,
        }
    }
}
