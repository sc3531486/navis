# navis-code（产品壳扩展）

Navis Code 产品壳：在通用 Navis 框架上组合 Agent IDE 形态。

本扩展不再作为业务扩展的物理父目录（目录已展平，业务扩展全部位于 `extensions/` 根下）。产品装配清单见根目录 `navis-code.json`。

## 角色

- 注册产品级布局：`StudioLayout`（root 插槽）与 `DialogHost`（overlay 插槽）
- 发布子插槽：`navis-code.sidebar.left`、`navis-code.viewport.main`、`navis-code.statusbar`，供业务扩展挂载

## 结构

```text
ExtensionUI/src/index.tsx   # NavisPlugin：绑定具名组件 + 注册子插槽内容
extension.json              # 清单（声明 slots/providesSlots）
```

## 装配

由根目录 `navis-code.json` 声明本产品包含的扩展清单，宿主按配置装载。