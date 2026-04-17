//! Tauri commands: scratchpad CRUD with `RwLock` + atomic persist (no nested awaits on the same lock).

use serde::Deserialize;
use tauri::State;

use crate::scratchpad::{
    new_scratchpad_note_id, save_scratchpad_notes, scratchpad_now_iso8601, ScratchpadNote,
};
use crate::state::AstrocyteState;

/// WebView `invoke('add_note', { payload: { … } })` 内层为 camelCase；`alias` 仍接受 snake_case。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddNotePayload {
    pub content: String,
    #[serde(default, alias = "context_id")]
    pub context_id: Option<String>,
    #[serde(default, alias = "focus_duration")]
    pub focus_duration: Option<u32>,
}

#[tauri::command]
pub async fn get_notes(
    state: State<'_, AstrocyteState>,
) -> Result<Vec<ScratchpadNote>, String> {
    Ok(state.scratchpad_notes.read().await.clone())
}

#[tauri::command]
pub async fn add_note(
    payload: AddNotePayload,
    state: State<'_, AstrocyteState>,
) -> Result<ScratchpadNote, String> {
    let AddNotePayload {
        content,
        context_id,
        focus_duration,
    } = payload;
    let context_id = context_id.and_then(|s| {
        let t = s.trim();
        if t.is_empty() {
            None
        } else {
            Some(t.to_string())
        }
    });
    let focus_duration_mins = context_id.as_ref().and_then(|_| focus_duration);
    let now = scratchpad_now_iso8601();
    let note = ScratchpadNote {
        id: new_scratchpad_note_id(),
        content,
        created_at: now.clone(),
        updated_at: now,
        context_id,
        focus_duration_mins,
    };
    let mut notes = state.scratchpad_notes.write().await;
    notes.push(note.clone());
    match save_scratchpad_notes(notes.as_slice()) {
        Ok(()) => Ok(note),
        Err(e) => {
            notes.pop();
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn update_note(
    id: String,
    content: String,
    state: State<'_, AstrocyteState>,
) -> Result<(), String> {
    let mut notes = state.scratchpad_notes.write().await;
    let Some(idx) = notes.iter().position(|n| n.id == id) else {
        return Err(format!("note '{}' not found", id));
    };
    let old_content = notes[idx].content.clone();
    let old_updated_at = notes[idx].updated_at.clone();
    notes[idx].content = content;
    notes[idx].updated_at = scratchpad_now_iso8601();
    match save_scratchpad_notes(notes.as_slice()) {
        Ok(()) => Ok(()),
        Err(e) => {
            notes[idx].content = old_content;
            notes[idx].updated_at = old_updated_at;
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn delete_note(id: String, state: State<'_, AstrocyteState>) -> Result<(), String> {
    let mut notes = state.scratchpad_notes.write().await;
    let Some(idx) = notes.iter().position(|n| n.id == id) else {
        return Err(format!("note '{}' not found", id));
    };
    let removed = notes.remove(idx);
    match save_scratchpad_notes(notes.as_slice()) {
        Ok(()) => Ok(()),
        Err(e) => {
            notes.insert(idx, removed);
            Err(e)
        }
    }
}
