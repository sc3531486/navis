use std::sync::Arc;

use tauri::Emitter;

use crate::kernel;

pub fn publish_kernel_events_to_tauri(
    app_handle: tauri::AppHandle,
    event_bus: Arc<dyn kernel::EventBus>,
) {
    let handler: kernel::EventHandler = Arc::new(move |envelope| {
        if let Err(error) = app_handle.emit(envelope.topic.as_str(), envelope.clone()) {
            tracing::warn!(
                event_id = %envelope.id,
                topic = %envelope.topic,
                error = %error,
                "Failed to publish kernel event to Tauri"
            );
        }
    });

    if let Err(error) = event_bus.subscribe(None, None, handler) {
        tracing::warn!(error = %error, "Failed to subscribe Tauri event publisher to kernel events");
    }
}
