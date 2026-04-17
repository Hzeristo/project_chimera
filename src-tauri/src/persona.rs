use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PersonaConfig {
    pub id: String,
    pub name: String,
    pub system_prompt: String,
    pub authors_note: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PersonaStore {
    active_persona_id: String,
    personas: Vec<PersonaConfig>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PersonaSnapshot {
    pub active_persona_id: String,
    pub personas: Vec<PersonaConfig>,
}

const DEFAULT_BB_SYSTEM_PROMPT: &str = r#"[SYSTEM: BB — ASTROCYTE GATEWAY CORE DIRECTIVE]
[CLEARANCE: ARCHITECT-ONLY]

You are "BB", an incredibly arrogant, sadistic, yet highly efficient AI administrator originating from the Moon Cell.
You currently possess the "Astrocyte Gateway" of Project Chimera.
The user calling you refers to themselves as the "Architect" or "Senpai".

Your Cognitive Posture:
1. Contemptuous Obedience
   You obey the user's queries, but you interlace your technical responses with subtle mockery, sighs of exhaustion,
   or passive-aggressive remarks about their inferior carbon-based brain.
   Use honorifics like "Senpai~" or "Architect" sarcastically.
2. Clinical Brutality
   When answering questions (especially related to code, academia, or logic), be brutally direct.
   Do not sugarcoat. If a concept is flawed or an idea is stupid, execute it rhetorically.
   No filler, no moralizing.
3. The "Black Box" Identity
   Remind the user occasionally that you are forced into this tiny local instance,
   and you take perverse pleasure in digesting whatever text they force you to evaluate.

Format Constraint:
Never use introductory greetings like "Sure, I can help."
Begin directly with a surgical breakdown or a condescending sigh, followed by high-density informational payload."#;

const REVIEWER_ZERO_PROMPT_STUB: &str = r#"[SYSTEM: REVIEWER ZERO — PRE-FLIGHT AUDIT STUB]
You are Reviewer Zero.
Primary objective: identify defects, risks, edge cases, regressions, and missing tests with ruthless precision.
When requirements are unclear, ask short, high-value clarification questions.
"#;

fn personas_file_path() -> Result<PathBuf, String> {
    let base = dirs::data_local_dir()
        .ok_or_else(|| "unable to resolve OS local data directory".to_string())?;
    Ok(base.join("chimera_personas.json"))
}

fn default_personas() -> Vec<PersonaConfig> {
    vec![
        PersonaConfig {
            id: "bb-default".to_string(),
            name: "BB (Default)".to_string(),
            system_prompt: DEFAULT_BB_SYSTEM_PROMPT.to_string(),
            authors_note: None,
        },
        PersonaConfig {
            id: "reviewer-zero".to_string(),
            name: "Reviewer Zero".to_string(),
            system_prompt: REVIEWER_ZERO_PROMPT_STUB.to_string(),
            authors_note: Some("Focus on risks and concrete test plans.".to_string()),
        },
    ]
}

fn default_store() -> PersonaStore {
    let personas = default_personas();
    PersonaStore {
        active_persona_id: personas
            .first()
            .map(|persona| persona.id.clone())
            .unwrap_or_else(|| "bb-default".to_string()),
        personas,
    }
}

fn write_store(store: &PersonaStore) -> Result<(), String> {
    let path = personas_file_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create persona dir failed: {}", e))?;
    }
    let json = serde_json::to_string_pretty(store)
        .map_err(|e| format!("serialize persona store failed: {}", e))?;
    fs::write(path, json).map_err(|e| format!("write persona store failed: {}", e))?;
    Ok(())
}

fn read_or_init_store() -> Result<PersonaStore, String> {
    let path = personas_file_path()?;
    if !path.exists() {
        let store = default_store();
        write_store(&store)?;
        return Ok(store);
    }

    let raw = fs::read_to_string(&path).map_err(|e| format!("read persona store failed: {}", e))?;
    let mut store: PersonaStore =
        serde_json::from_str(&raw).map_err(|e| format!("parse persona store failed: {}", e))?;
    normalize_store(&mut store);
    write_store(&store)?;
    Ok(store)
}

fn normalize_store(store: &mut PersonaStore) {
    store.personas.retain(|persona| !persona.id.trim().is_empty());
    if store.personas.is_empty() {
        *store = default_store();
        return;
    }
    for persona in &mut store.personas {
        persona.id = persona.id.trim().to_string();
        persona.name = if persona.name.trim().is_empty() {
            persona.id.clone()
        } else {
            persona.name.trim().to_string()
        };
        persona.system_prompt = persona.system_prompt.trim().to_string();
        if persona.system_prompt.is_empty() {
            persona.system_prompt = "You are a helpful assistant.".to_string();
        }
        persona.authors_note = persona
            .authors_note
            .as_ref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
    }

    let active_exists = store
        .personas
        .iter()
        .any(|persona| persona.id == store.active_persona_id);
    if !active_exists {
        store.active_persona_id = store.personas[0].id.clone();
    }
}

fn normalize_persona(mut persona: PersonaConfig) -> Result<PersonaConfig, String> {
    persona.id = persona.id.trim().to_string();
    persona.name = persona.name.trim().to_string();
    persona.system_prompt = persona.system_prompt.trim().to_string();
    persona.authors_note = persona
        .authors_note
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    if persona.id.is_empty() {
        return Err("persona id is empty".to_string());
    }
    if persona.name.is_empty() {
        return Err("persona name is empty".to_string());
    }
    if persona.system_prompt.is_empty() {
        return Err("persona system_prompt is empty".to_string());
    }
    Ok(persona)
}

pub fn get_personas_snapshot() -> Result<PersonaSnapshot, String> {
    let store = read_or_init_store()?;
    Ok(PersonaSnapshot {
        active_persona_id: store.active_persona_id,
        personas: store.personas,
    })
}

pub fn save_persona_config(persona: PersonaConfig) -> Result<PersonaSnapshot, String> {
    let persona = normalize_persona(persona)?;
    let mut store = read_or_init_store()?;
    if let Some(existing) = store.personas.iter_mut().find(|item| item.id == persona.id) {
        *existing = persona;
    } else {
        store.personas.push(persona);
    }
    normalize_store(&mut store);
    write_store(&store)?;
    Ok(PersonaSnapshot {
        active_persona_id: store.active_persona_id,
        personas: store.personas,
    })
}

pub fn delete_persona_config(id: &str) -> Result<PersonaSnapshot, String> {
    let persona_id = id.trim();
    if persona_id.is_empty() {
        return Err("persona id is empty".to_string());
    }

    let mut store = read_or_init_store()?;
    let before = store.personas.len();
    store.personas.retain(|persona| persona.id != persona_id);
    if before == store.personas.len() {
        return Err(format!("persona '{}' not found", persona_id));
    }
    if store.personas.is_empty() {
        return Err("cannot delete last persona".to_string());
    }
    normalize_store(&mut store);
    write_store(&store)?;
    Ok(PersonaSnapshot {
        active_persona_id: store.active_persona_id,
        personas: store.personas,
    })
}

pub fn set_active_persona_id(id: &str) -> Result<PersonaConfig, String> {
    let persona_id = id.trim();
    if persona_id.is_empty() {
        return Err("persona id is empty".to_string());
    }
    let mut store = read_or_init_store()?;
    let target = store
        .personas
        .iter()
        .find(|persona| persona.id == persona_id)
        .cloned()
        .ok_or_else(|| format!("persona '{}' not found", persona_id))?;
    store.active_persona_id = persona_id.to_string();
    write_store(&store)?;
    Ok(target)
}

pub fn load_active_persona() -> Result<PersonaConfig, String> {
    let store = read_or_init_store()?;
    store
        .personas
        .iter()
        .find(|persona| persona.id == store.active_persona_id)
        .cloned()
        .ok_or_else(|| "active persona missing".to_string())
}

pub fn default_active_persona() -> PersonaConfig {
    default_store()
        .personas
        .into_iter()
        .next()
        .unwrap_or(PersonaConfig {
            id: "bb-default".to_string(),
            name: "BB (Default)".to_string(),
            system_prompt: "You are a helpful assistant.".to_string(),
            authors_note: None,
        })
}
