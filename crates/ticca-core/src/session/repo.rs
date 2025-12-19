//! Repository trait for session storage

use anyhow::Result;

use crate::session::database::SessionDatabase;
use crate::session::models::{Session, SessionMessage};

pub trait SessionRepo {
    fn create_session(&self, session: &Session) -> Result<()>;
    fn get_session(&self, id: &str) -> Result<Option<Session>>;
    fn list_sessions(&self) -> Result<Vec<Session>>;
    fn delete_session(&self, id: &str) -> Result<bool>;
    fn add_message(&self, message: &SessionMessage) -> Result<i64>;
    fn get_messages(&self, session_id: &str) -> Result<Vec<SessionMessage>>;
    fn clear_session_messages(&self, session_id: &str) -> Result<usize>;
}

impl SessionRepo for SessionDatabase {
    fn create_session(&self, session: &Session) -> Result<()> {
        SessionDatabase::create_session(self, session)
    }

    fn get_session(&self, id: &str) -> Result<Option<Session>> {
        SessionDatabase::get_session(self, id)
    }

    fn list_sessions(&self) -> Result<Vec<Session>> {
        SessionDatabase::list_sessions(self)
    }

    fn delete_session(&self, id: &str) -> Result<bool> {
        SessionDatabase::delete_session(self, id)
    }

    fn add_message(&self, message: &SessionMessage) -> Result<i64> {
        SessionDatabase::add_message(self, message)
    }

    fn get_messages(&self, session_id: &str) -> Result<Vec<SessionMessage>> {
        SessionDatabase::get_messages(self, session_id)
    }

    fn clear_session_messages(&self, session_id: &str) -> Result<usize> {
        SessionDatabase::clear_session_messages(self, session_id)
    }
}
