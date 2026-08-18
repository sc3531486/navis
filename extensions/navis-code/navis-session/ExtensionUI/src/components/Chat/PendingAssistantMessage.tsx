import { Component } from 'solid-js';

const PendingAssistantMessage: Component = () => (
  <article class="navis-message is-navis is-pending-assistant self-start" aria-live="polite">
    <div class="navis-message-thinking" role="status">
      <span class="navis-message-thinking-copy navis-shimmer-text">正在思考</span>
    </div>
  </article>
);

export default PendingAssistantMessage;
