# 白板容器规范

## 底座职责

白板容器（WhiteboardShell）是 Navis 底座的唯一 UI 入口。它不包含任何业务逻辑、不预设任何物理分区。

## 插槽体系

### 根级插槽
- `root`：主视口，扩展自主决定布局形式
- `overlay`：全局浮层，供弹窗、悬浮菜单使用

### 扩展子插槽
扩展可在 root 内开辟任意命名的子插槽，支持 Slot-in-Slot 递归：
```
root
├── navis-code.sidebar.left
├── navis-code.viewport.main
│   ├── navis-code.editor.tabs
│   └── navis-code.terminal.panel
└── navis-code.statusbar
```

## 无扩展状态

当没有任何扩展挂载时，白板显示品牌占位卡片：
- Logo + 标题 + 描述
- 提示可通过插件动态注入 UI

## CSS 规范

底座使用暗色主题（#0f1117），使用 flexbox 布局，无固定尺寸约束。
