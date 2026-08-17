//! Text-form Agent tool call parsing.
//!
//! Some providers or degraded adapters may emit `<tool_call>` XML-like text
//! instead of native function-call parts. This module normalizes that shape back
//! into the same Gateway `ToolCall` model used by native tool calls.

