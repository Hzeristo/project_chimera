use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::persona::{self, PersonaConfig};
use crate::settings::{load_astrocyte_config, AstrocyteConfig};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Clone, Debug)]
pub struct Session {
    pub history: Vec<Message>,
}

pub struct AstrocyteState {
    pub sessions: RwLock<HashMap<String, Session>>,
    pub config: RwLock<AstrocyteConfig>,
    pub active_persona: RwLock<PersonaConfig>,
}

impl AstrocyteState {
    pub fn new() -> Self {
        let active_persona =
            persona::load_active_persona().unwrap_or_else(|_| persona::default_active_persona());
        Self {
            sessions: RwLock::new(HashMap::new()),
            config: RwLock::new(load_astrocyte_config()),
            active_persona: RwLock::new(active_persona),
        }
    }

    pub async fn create_session(&self, id: String) {
        let mut sessions = self.sessions.write().await;
        sessions
            .entry(id)
            .or_insert(Session { history: Vec::new() });
    }

    pub async fn get_history(&self, session_id: &str) -> Option<Vec<Message>> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).map(|s| s.history.clone())
    }

    pub async fn append_message_to_session(
        &self,
        session_id: &str,
        role: &str,
        content: String,
    ) -> Result<(), String> {
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("session '{}' not found", session_id))?;
        session.history.push(Message {
            role: role.to_string(),
            content,
        });
        Ok(())
    }

    pub async fn set_history_for_session(
        &self,
        session_id: &str,
        history: Vec<Message>,
    ) -> Result<(), String> {
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("session '{}' not found", session_id))?;
        session.history = history;
        Ok(())
    }
}
