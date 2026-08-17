//! Tool execution guardrails.
//!
//! Guardrails stop tight failure loops before they reach the concrete tool
//! executor again. They operate on the model transcript and return a normal tool
//! result so the model can recover with a different strategy.

