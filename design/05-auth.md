# 05 - Auth 身份认证 详细设计

> 模块编号：05 | 层级：核心服务层
> 依赖：01-Logger, 02-Event+IPC, 03-Config, 04-Storage
> 被依赖：06-Sandbox, 12-Gateway, 21-Git, Extension lifecycle

---

## 一、模块概述

### 1.1 定位

Auth 是 Navis Go 的凭据事实源，负责所有需要保密的外部凭据：Gateway Provider 的模型调用密钥、Git 凭证、第三方服务 Token 以及 Extension 运行所需的私有凭据。Auth 只管理凭据的身份、加密存储、解析、轮转和校验状态，不负责协议字段转换，也不负责 Provider 或 Extension 生命周期。

Gateway 和 Extension manifest 只保存不透明的 `secret_ref`。只有受控的后端能力端口可以在请求执行的最小范围内解析临时 secret；UI、日志、manifest、配置投影和 Extension 运行时均不得取得明文 secret。

### 1.2 职责边界

负责：
- 凭据创建、加密存储、元数据查询、轮转和删除。
- 为 Gateway 提供受控的 `SecretResolver`，按 opaque reference 解析短生命周期 secret。
- 为 Git、第三方服务和 Extension 提供同一套凭据生命周期能力。
- 维护校验状态、过期状态和审计事件。
- 对 secret 的用途、所有者、访问范围和最小暴露时间进行约束。

不负责：
- Provider 协议转换、请求体模板、响应路径或流式 framing；这些属于 Gateway Adapter。
- Provider、Model 或 Extension 的注册与启停；这些属于 Gateway / Extension lifecycle。
- UI 中的 Provider 分支、协议下拉或凭据明文展示。
- 加密算法实现；具体算法由安全存储实现和密码库负责。

---

## 二、架构设计

    auth/
    ├── mod.rs              # Auth facade、SecretResolver 与公开数据类型
    ├── key_store.rs        # 加密 secret 存储与零化
    ├── key_validator.rs    # 通用校验状态与验证端口
    ├── credential.rs       # Git / 第三方凭据模型
    ├── provider_keys.rs    # Provider / Extension scope 的 reference 管理
    └── schema_check.rs     # Auth 数据与 reference 合同校验

依赖方向：

    auth storage -> SecretStore -> SecretResolver port -> Gateway / Extension host
                                       └-> Git / third-party capability ports

Gateway 只能依赖 `SecretResolver` 端口，不能依赖具体数据库表、加密实现或 Auth facade 的内部存储结构。Extension 只能声明凭据需求并引用用户选择的 reference，不能直接调用底层 key store。

---

## 三、数据模型

~~~rust
struct SecretReference {
    id: String,                 // opaque ID，不携带 secret 内容
    owner_scope: String,        // provider:<id> / extension:<id> / git:<id>
    kind: String,               // bearer / api-token / ssh-key / custom
    label: String,              // UI 展示名称，不包含 secret
}

struct SecretMetadata {
    reference: SecretReference,
    created_at: DateTime,
    updated_at: DateTime,
    expires_at: Option<DateTime>,
    validation: ValidationStatus,
}

enum ValidationStatus {
    Unknown,     // 尚未校验、校验超时或校验器未提供
    Reachable,   // 目标可达，但凭据有效性未被确认
    Valid,       // 目标接受该凭据
    Invalid,     // 目标明确拒绝该凭据
}
~~~

约束：
- `SecretReference.id` 是唯一路由标识，显示名称不能参与运行时查找。
- Secret 内容只存在于加密存储和受控的短生命周期内存对象中。
- Provider、Model、Adapter 和 Extension manifest 不内嵌 secret；它们只保存 `secret_ref`。
- Auth 返回给 UI 的只能是 `SecretMetadata`；返回给 Gateway 的临时值必须限定用途、调用方和生命周期。
- 删除或轮转 reference 时，所有引用它的运行配置必须进入可诊断状态，不能静默回退到另一个 secret。

---

## 四、接口定义

### 4.1 Rust API

~~~rust
Auth::create_secret(scope: SecretScope, kind: SecretKind, value: SecretInput) -> Result<SecretMetadata>
Auth::list_secrets(scope: Option<&SecretScope>) -> Result<Vec<SecretMetadata>>
Auth::remove_secret(reference_id: &str) -> Result<()>
Auth::rotate_secret(reference_id: &str, value: SecretInput) -> Result<SecretMetadata>
Auth::validate_secret(reference_id: &str, validator: &dyn SecretValidator) -> Result<ValidationStatus>
SecretResolver::resolve(reference: &SecretReference, purpose: SecretPurpose) -> Result<ResolvedSecret>
~~~

`ResolvedSecret` 必须实现自动清零，并且不能实现可序列化、可调试输出或前端 DTO 转换。Gateway 请求完成、取消或失败后立即释放临时 secret。

### 4.2 IPC 命令

~~~typescript
auth.createSecret(scope, kind, input): Promise<SecretMetadata>
auth.listSecrets(scope?): Promise<SecretMetadata[]>
auth.removeSecret(referenceId): Promise<void>
auth.rotateSecret(referenceId, input): Promise<SecretMetadata>
auth.validateSecret(referenceId): Promise<ValidationStatus>
~~~

IPC 不返回 secret 内容。创建和轮转只接收一次性安全输入，前端不得把 secret 写入 localStorage、普通配置、事件 payload 或日志。

---

## 五、Gateway 与 Extension 集成

### 5.1 Gateway

Gateway Provider 的认证配置只允许声明 scheme、opaque `secret_ref` 和可选 header。Gateway 在请求执行前通过 `SecretResolver` 获取临时值，并由 Adapter 按认证合同注入请求。Adapter 不读取 Auth 存储，Gateway 也不把 secret 写入 Provider catalog、事件或错误文本。

### 5.2 Extension

Extension manifest 可以声明认证方案和可选 reference，但不能预置 secret，不能通过模板变量、模块路径、任意 IPC 或日志间接获取 secret。用户选择 reference 后，宿主只把 opaque ID 写入 Provider 配置。

### 5.3 校验端口

Auth 不按 Provider 名称写死校验分支。校验器由宿主能力端口提供，输入是目标、协议能力和临时 secret，输出只能是 `Unknown / Reachable / Valid / Invalid`。网络可达不等于凭据有效；没有明确验证结果时不得标记为 `Valid`。

---

## 六、错误处理

| 场景 | 处理策略 |
|------|----------|
| reference 不存在 | 返回 `SecretNotFound`，Gateway 不发送请求 |
| reference 不属于当前 scope | 返回 `SecretScopeDenied`，不尝试其他 reference |
| secret 已过期 | 返回 `SecretExpired`，保留 Provider / Extension 配置并要求用户轮转 |
| 校验目标可达但拒绝凭据 | 状态为 `Invalid`，响应正文不得写入日志 |
| 校验超时或结果不明确 | 状态为 `Unknown`，不能降级为 `Valid` |
| Adapter 请求失败 | Gateway 归一化错误，不包含 secret、Authorization header 或完整请求体 |
| Extension 禁用或卸载 | 先注销运行能力，再清理 runtime handle；是否删除 secret 由用户显式操作决定 |

---

## 七、事件定义

~~~typescript
type AuthEvents = {
  "auth.secret.created":   { referenceId: string; scope: string; kind: string }
  "auth.secret.rotated":   { referenceId: string; scope: string }
  "auth.secret.removed":   { referenceId: string; scope: string }
  "auth.secret.validated": { referenceId: string; status: ValidationStatus }
  "auth.secret.expired":   { referenceId: string; scope: string }
}
~~~

事件只包含 reference ID、scope、状态和非敏感元数据，不包含 secret 内容、请求头、响应正文或解密后的凭据。

---

## 八、安全考量

1. **加密存储**：secret 使用 AES-256-GCM 或等价的认证加密方案存储，主密钥由设备安全边界和用户保护机制管理。
2. **最小暴露**：只有受控后端 port 能解析 secret；ResolvedSecret 限定用途、生命周期和调用者。
3. **日志脱敏**：统一脱敏 token、Authorization、Cookie、请求模板展开值和错误响应中的敏感字段。
4. **输入校验**：scope、kind、reference、header 和 validator 目标必须经过 schema 与权限校验。
5. **备份保护**：备份和导出只允许加密形式，禁止把 secret 写入普通配置或 manifest。
6. **轮转隔离**：轮转只替换被指定的 reference，不自动修改其他 Provider、Model 或 Extension。
7. **零信任 Extension**：Extension 不获得 Auth 存储句柄、解密密钥、任意文件权限或任意网络代理。

---

## 九、测试策略

```text
单元测试：加密/解密、reference scope、零化、状态机、header 脱敏、过期判断
集成测试：Gateway 通过 SecretResolver 获取临时 secret；Extension Provider 使用 opaque reference；删除/轮转后的诊断状态
安全测试：secret 不出现在 UI DTO、事件、日志、错误、catalog、模板展开和备份文件
回归测试：Git / 第三方凭据与 Gateway secret 共用 Auth Store，但互不越权
```

文档合同：Auth 是 secret 的唯一事实源；Gateway、Extension、UI 和配置层只持有 reference 或脱敏 metadata。任何新增 Provider、协议或 Extension 都不得重新引入独立密钥存储或明文配置字段。