use crate::session::AgentSession;
use anyhow::Result;
use tester_providers::{ChatMessage, Llm};
use tracing::info;

pub fn maybe_compact(llm: &dyn Llm, session: &mut AgentSession, max_chars: usize) -> Result<()> {
    let total: usize = session
        .messages
        .iter()
        .map(|m| m.content.as_deref().unwrap_or("").len())
        .sum();
    if total < max_chars.saturating_mul(9) / 10 || session.messages.len() < 6 {
        return Ok(());
    }

    info!(total, max_chars, "compaction истории агента");
    let system = session
        .messages
        .first()
        .cloned()
        .unwrap_or_else(|| ChatMessage::system("Ты - tester agent."));
    let recent: Vec<_> = session
        .messages
        .iter()
        .rev()
        .take(4)
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    let blob: String = session
        .messages
        .iter()
        .skip(1)
        .take(session.messages.len().saturating_sub(5))
        .map(|m| {
            format!(
                "{}: {}",
                m.role,
                m.content
                    .as_deref()
                    .unwrap_or("")
                    .chars()
                    .take(400)
                    .collect::<String>()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let summary = llm.complete(
        "Сожми историю диалога агента в краткое резюме на русском. Сохрани факты о проекте, файлах и ошибках.",
        &blob,
    )?;

    let mut next = vec![
        system,
        ChatMessage::user(format!("[сжатая история]\n{summary}")),
    ];
    next.extend(recent);
    session.messages = next;
    Ok(())
}
