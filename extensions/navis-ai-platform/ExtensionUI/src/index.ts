/**
 * ============================================================
 * navis-ai-platform 扩展前端 — 迁移过渡期 re-export 桥
 * ============================================================
 *
 * 实际文件仍保留在 src/stores/gateway.ts、
 * src/stores/gateway-menu.ts
 * 后续 Phase 执行物理搬迁时，此处的 re-export 路径将同步更新。
 *
 * 归属说明：
 *   - gateway store → 扩展 navis-ai-platform
 *   - gateway-menu store → 扩展 navis-ai-platform
 * ============================================================
 */

// ── Gateway Store ────────────────────────────────────────
export {
  gatewayState,
  setGatewayState,
  loadGatewayCatalog,
} from '@/stores/gateway';

export type {
  GatewayProvider,
  GatewayModel,
} from '@/stores/gateway';

// ── Gateway Menu Store ──────────────────────────────────
export {
  gatewayMenuState,
  setGatewayMenuState,
} from '@/stores/gateway-menu';

export type {
  GatewayMenuState,
} from '@/stores/gateway-menu';
