/**
 * 通知渠道分发器
 *
 * 管理扩展渠道的注册、初始化和通知分发。
 * 对应 design/25-notification.md「扩展扩展支持」章节。
 *
 * 分发机制：
 * 任意模块发出通知
 *      |
 *      v
 * Notification Dispatcher
 *      |
 *      ├── 内置渠道：Toast（即时显示）
 *      ├── 内置渠道：Notification Center（持久化存储）
 *      ├── 内置渠道：System Notification（系统原生通知）
 *      |
 *      └── 扩展渠道（并行分发）：
 *          ├── Slack -> channel.send(payload)
 *          ├── Webhook -> channel.send(payload)
 *          ├── 邮件 -> channel.send(payload)
 *          └── ...（任何已注册的渠道）
 */

import type {
  NotificationChannel,
  NotificationPayload,
  ChannelFilterConfig,
  NotificationLevel,
} from './types'

// ============================================================
// 一、渠道存储
// ============================================================

/** 已注册渠道条目 */
interface RegisteredChannelEntry {
  /** 渠道实例 */
  channel: NotificationChannel
  /** 渠道过滤配置 */
  filter: ChannelFilterConfig
  /** 是否已初始化 */
  initialized: boolean
}

/**
 * 全局渠道存储
 *
 * 存储所有已接入的扩展通知渠道。
 * 使用 Map 保证渠道 ID 的唯一性。
 */
const channelStore = new Map<string, RegisteredChannelEntry>()

// ============================================================
// 二、渠道注册/注销
// ============================================================

/**
 * 注册扩展通知渠道
 *
 * 扩展通过此函数注册自定义通知渠道（如 Slack、Webhook、邮件等）。
 * 注册后需要调用 initializeChannel() 进行初始化才能使用。
 *
 * @param channel - 实现 NotificationChannel 接口的渠道实例
 * @param filter  - 级别过滤配置（可选，默认接收所有级别）
 *
 * @example
 * ```ts
 * const slackChannel: NotificationChannel = {
 *   id: 'slack',
 *   name: 'Slack',
 *   initialize: async (config) => { ... },
 *   send: async (payload) => { ... },
 *   dispose: async () => { ... },
 * }
 *
 * registerNotificationChannel(slackChannel, {
 *   channelId: 'slack',
 *   levels: ['warning', 'error'], // 只接收 warning 和 error
 * })
 * ```
 */
export function registerNotificationChannel(
  channel: NotificationChannel,
  filter?: ChannelFilterConfig,
): void {
  // 防止重复注册
  if (channelStore.has(channel.id)) {
    return
  }

  const defaultFilter: ChannelFilterConfig = {
    channelId: channel.id,
    levels: [], // 空数组 = 接收所有级别
  }

  channelStore.set(channel.id, {
    channel,
    filter: filter ?? defaultFilter,
    initialized: false,
  })
}

/**
 * 注销扩展通知渠道
 *
 * 注销前会调用渠道的 dispose() 清理资源。
 *
 * @param channelId - 要注销的渠道 ID
 */
export async function unregisterNotificationChannel(
  channelId: string,
): Promise<void> {
  const entry = channelStore.get(channelId)
  if (!entry) return

  // 如果已初始化，先清理
  if (entry.initialized) {
    try {
      await entry.channel.dispose()
    } catch {
      // dispose 失败不阻止注销
    }
  }

  channelStore.delete(channelId)
}

// ============================================================
// 三、渠道初始化
// ============================================================

/**
 * 初始化指定渠道
 *
 * 扩展启用时调用，传入用户在设置页面配置的参数。
 *
 * @param channelId - 渠道 ID
 * @param config    - 用户配置（对应 configSchema 定义的字段）
 * @throws 初始化失败时抛出异常
 */
export async function initializeChannel(
  channelId: string,
  config: Record<string, unknown>,
): Promise<void> {
  const entry = channelStore.get(channelId)
  if (!entry) {
    throw new Error(`通知渠道 "${channelId}" 未注册`)
  }

  await entry.channel.initialize(config)
  entry.initialized = true
}

/**
 * 初始化所有已注册但未初始化的渠道
 *
 * @param configProvider - 配置提供函数，根据渠道 ID 返回用户配置
 */
export async function initializeAllChannels(
  configProvider: (channelId: string) => Record<string, unknown>,
): Promise<void> {
  const tasks: Promise<void>[] = []

  for (const [channelId, entry] of channelStore) {
    if (!entry.initialized) {
      const config = configProvider(channelId)
      tasks.push(
        entry.channel.initialize(config).then(() => {
          entry.initialized = true
        }),
      )
    }
  }

  await Promise.allSettled(tasks)
}

// ============================================================
// 四、通知分发
// ============================================================

/**
 * 检查通知是否匹配渠道的级别过滤器
 *
 * @param level  - 通知级别
 * @param filter - 渠道过滤配置
 * @returns true 表示应该发送到该渠道
 */
function matchesFilter(level: NotificationLevel, filter: ChannelFilterConfig): boolean {
  // 空数组 = 接收所有级别
  if (filter.levels.length === 0) return true
  return filter.levels.includes(level)
}

/**
 * 将通知分发到所有匹配的扩展渠道
 *
 * 分发是异步并行的，单个渠道失败不影响其他渠道。
 * 返回每个渠道的发送结果。
 *
 * @param payload - 通知载荷
 * @returns 各渠道的发送结果
 */
export async function dispatchToAllChannels(
  payload: NotificationPayload,
): Promise<Array<{ channelId: string; success: boolean; error?: string }>> {
  const results: Array<{ channelId: string; success: boolean; error?: string }> = []

  // 筛选匹配的渠道
  const targets = Array.from(channelStore.values()).filter(
    (entry) => entry.initialized && matchesFilter(payload.level, entry.filter),
  )

  // 并行分发
  const tasks = targets.map(async (entry) => {
    try {
      await entry.channel.send(payload)
      results.push({ channelId: entry.channel.id, success: true })
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err)
      results.push({
        channelId: entry.channel.id,
        success: false,
        error: errorMessage,
      })
    }
  })

  await Promise.allSettled(tasks)
  return results
}

// ============================================================
// 五、查询
// ============================================================

/**
 * 获取所有已注册渠道的信息
 *
 * @returns 渠道信息数组（不含内部引用，安全用于展示）
 */
export function getRegisteredChannels(): Array<{
  id: string
  name: string
  initialized: boolean
  filterLevels: NotificationLevel[]
}> {
  return Array.from(channelStore.values()).map((entry) => ({
    id: entry.channel.id,
    name: entry.channel.name,
    initialized: entry.initialized,
    filterLevels: entry.filter.levels,
  }))
}

/**
 * 更新渠道的级别过滤配置
 *
 * @param channelId - 渠道 ID
 * @param levels    - 新的级别过滤列表
 */
export function updateChannelFilter(
  channelId: string,
  levels: NotificationLevel[],
): void {
  const entry = channelStore.get(channelId)
  if (entry) {
    entry.filter.levels = levels
  }
}

/**
 * 清空所有注册的渠道
 *
 * 用于应用退出时清理。
 */
export async function disposeAllChannels(): Promise<void> {
  const tasks = Array.from(channelStore.values()).map(async (entry) => {
    if (entry.initialized) {
      try {
        await entry.channel.dispose()
      } catch {
        // dispose 失败不影响其他渠道
      }
    }
  })

  await Promise.allSettled(tasks)
  channelStore.clear()
}
