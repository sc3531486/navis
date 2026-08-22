import { Component, JSX } from 'solid-js';

export interface IconProps extends JSX.SvgSVGAttributes<SVGSVGElement> {
  size?: number;
  class?: string;
  color?: string;
}

const defaultProps = (props: IconProps) => ({
  width: props.size ?? 16,
  height: props.size ?? 16,
  viewBox: '0 0 24 24',
  fill: 'none',
  stroke: props.color ?? 'currentColor',
  'stroke-width': 2,
  'stroke-linecap': 'round' as const,
  'stroke-linejoin': 'round' as const,
  ...props,
});

// 设置图标
export const IconSettings: Component<IconProps> = (props) => (
  <svg {...defaultProps(props)}>
    <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" />
    <circle cx="12" cy="12" r="3" />
  </svg>
);

// 模型/CPU 图标
export const IconCpu: Component<IconProps> = (props) => (
  <svg {...defaultProps(props)}>
    <rect x="4" y="4" width="16" height="16" rx="2" />
    <rect x="9" y="9" width="6" height="6" />
    <path d="M9 1v3M15 1v3M9 20v3M15 20v3M20 9h3M20 14h3M1 9h3M1 14h3" />
  </svg>
);

// 沙箱与权限图标
export const IconShield: Component<IconProps> = (props) => (
  <svg {...defaultProps(props)}>
    <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
  </svg>
);

// 提示词/文档图标
export const IconPrompt: Component<IconProps> = (props) => (
  <svg {...defaultProps(props)}>
    <path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z" />
    <polyline points="14 2 14 8 20 8" />
    <line x1="16" y1="13" x2="8" y2="13" />
    <line x1="16" y1="17" x2="8" y2="17" />
    <line x1="10" y1="9" x2="8" y2="9" />
  </svg>
);

// 外观/调色盘图标
export const IconPalette: Component<IconProps> = (props) => (
  <svg {...defaultProps(props)}>
    <circle cx="13.5" cy="6.5" r=".5" fill="currentColor" />
    <circle cx="17.5" cy="10.5" r=".5" fill="currentColor" />
    <circle cx="8.5" cy="7.5" r=".5" fill="currentColor" />
    <circle cx="6.5" cy="12.5" r=".5" fill="currentColor" />
    <path d="M12 2C6.5 2 2 6.5 2 12s4.5 10 10 10c.926 0 1.648-.746 1.648-1.688 0-.437-.18-.835-.437-1.125-.29-.289-.438-.652-.438-1.125a1.64 1.64 0 0 1 1.668-1.668h1.996c3.051 0 5.555-2.503 5.555-5.554C21.965 6.012 17.461 2 12 2z" />
  </svg>
);

// 闪光/灵感图标
export const IconSparkles: Component<IconProps> = (props) => (
  <svg {...defaultProps(props)}>
    <path d="m12 3-1.912 5.813a2 2 0 0 1-1.275 1.275L3 12l5.813 1.912a2 2 0 0 1 1.275 1.275L12 21l1.912-5.813a2 2 0 0 1 1.275-1.275L21 12l-5.813-1.912a2 2 0 0 1-1.275-1.275L12 3Z" />
    <path d="M5 3v4M3 5h4M19 17v4M17 19h4" />
  </svg>
);

// 闪电/测速图标
export const IconZap: Component<IconProps> = (props) => (
  <svg {...defaultProps(props)}>
    <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2" />
  </svg>
);

// 刷新/同步图标
export const IconRefresh: Component<IconProps> = (props) => (
  <svg {...defaultProps(props)}>
    <path d="M21.5 2v6h-6M21.34 15.57a10 10 0 1 1-.57-8.38l5.67-5.67" />
  </svg>
);

// 勾选图标
export const IconCheck: Component<IconProps> = (props) => (
  <svg {...defaultProps(props)}>
    <polyline points="20 6 9 17 4 12" />
  </svg>
);

// 关闭/叉图标
export const IconClose: Component<IconProps> = (props) => (
  <svg {...defaultProps(props)}>
    <line x1="18" y1="6" x2="6" y2="18" />
    <line x1="6" y1="6" x2="18" y2="18" />
  </svg>
);

// 垃圾桶/删除图标
export const IconTrash: Component<IconProps> = (props) => (
  <svg {...defaultProps(props)}>
    <polyline points="3 6 5 6 21 6" />
    <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
  </svg>
);

// 眼睛/显示图标
export const IconEye: Component<IconProps> = (props) => (
  <svg {...defaultProps(props)}>
    <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" />
    <circle cx="12" cy="12" r="3" />
  </svg>
);

// 闭眼/隐藏图标
export const IconEyeOff: Component<IconProps> = (props) => (
  <svg {...defaultProps(props)}>
    <path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24" />
    <line x1="1" y1="1" x2="23" y2="23" />
  </svg>
);

// 加号图标
export const IconPlus: Component<IconProps> = (props) => (
  <svg {...defaultProps(props)}>
    <line x1="12" y1="5" x2="12" y2="19" />
    <line x1="5" y1="12" x2="19" y2="12" />
  </svg>
);

// 搜索图标
export const IconSearch: Component<IconProps> = (props) => (
  <svg {...defaultProps(props)}>
    <circle cx="11" cy="11" r="8" />
    <line x1="21" y1="21" x2="16.65" y2="16.65" />
  </svg>
);

// 分屏视图图标
export const IconSplit: Component<IconProps> = (props) => (
  <svg {...defaultProps(props)}>
    <rect x="3" y="3" width="18" height="18" rx="2" />
    <line x1="12" y1="3" x2="12" y2="21" />
  </svg>
);

// 侧边栏汉堡菜单图标
export const IconMenu: Component<IconProps> = (props) => (
  <svg {...defaultProps(props)}>
    <line x1="3" y1="12" x2="21" y2="12" />
    <line x1="3" y1="6" x2="21" y2="6" />
    <line x1="3" y1="18" x2="21" y2="18" />
  </svg>
);

// 下拉箭头
export const IconChevronDown: Component<IconProps> = (props) => (
  <svg {...defaultProps(props)}>
    <polyline points="6 9 12 15 18 9" />
  </svg>
);

// 右箭头
export const IconChevronRight: Component<IconProps> = (props) => (
  <svg {...defaultProps(props)}>
    <polyline points="9 18 15 12 9 6" />
  </svg>
);

// 左箭头
export const IconArrowLeft: Component<IconProps> = (props) => (
  <svg {...defaultProps(props)}>
    <line x1="19" y1="12" x2="5" y2="12" />
    <polyline points="12 19 5 12 12 5" />
  </svg>
);

// 右箭头
export const IconArrowRight: Component<IconProps> = (props) => (
  <svg {...defaultProps(props)}>
    <line x1="5" y1="12" x2="19" y2="12" />
    <polyline points="12 5 19 12 12 19" />
  </svg>
);

// 复制图标
export const IconCopy: Component<IconProps> = (props) => (
  <svg {...defaultProps(props)}>
    <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
    <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
  </svg>
);

// 文件夹图标
export const IconFolder: Component<IconProps> = (props) => (
  <svg {...defaultProps(props)}>
    <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
  </svg>
);

// Git 分支图标
export const IconGitBranch: Component<IconProps> = (props) => (
  <svg {...defaultProps(props)}>
    <line x1="6" y1="3" x2="6" y2="15" />
    <circle cx="18" cy="6" r="3" />
    <circle cx="6" cy="18" r="3" />
    <path d="M18 9a9 9 0 0 1-9 9" />
  </svg>
);

// 终端图标
export const IconTerminal: Component<IconProps> = (props) => (
  <svg {...defaultProps(props)}>
    <polyline points="4 17 10 11 4 5" />
    <line x1="12" y1="19" x2="20" y2="19" />
  </svg>
);

// 思考灯泡图标
export const IconLightbulb: Component<IconProps> = (props) => (
  <svg {...defaultProps(props)}>
    <path d="M15 14c.2-1 .7-1.7 1.5-2.5 1-.9 1.5-2.2 1.5-3.5A6 6 0 0 0 6 8c0 1 .2 2.2 1.5 3.5.7.7 1.3 1.5 1.5 2.5" />
    <path d="M9 18h6M10 22h4" />
  </svg>
);

// 工具扳手图标
export const IconWrench: Component<IconProps> = (props) => (
  <svg {...defaultProps(props)}>
    <path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z" />
  </svg>
);

// 吉祥物 / 徽标星号
export const IconAsterisk: Component<IconProps> = (props) => (
  <svg {...defaultProps(props)}>
    <path d="M12 2v20M17 5H9.5M17 19H9.5M4.93 4.93l14.14 14.14M4.93 19.07l14.14-14.14" />
  </svg>
);

// 发送箭头图标
export const IconSend: Component<IconProps> = (props) => (
  <svg {...defaultProps(props)}>
    <line x1="22" y1="2" x2="11" y2="13" />
    <polygon points="22 2 15 22 11 13 2 9 22 2" />
  </svg>
);

// 医疗诊断图标
export const IconActivity: Component<IconProps> = (props) => (
  <svg {...defaultProps(props)}>
    <polyline points="22 12 18 12 15 21 9 3 6 12 2 12" />
  </svg>
);

// 钱币/费用图标
export const IconDollarSign: Component<IconProps> = (props) => (
  <svg {...defaultProps(props)}>
    <line x1="12" y1="1" x2="12" y2="23" />
    <path d="M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6" />
  </svg>
);

// 插件/MCP 图标
export const IconPlug: Component<IconProps> = (props) => (
  <svg {...defaultProps(props)}>
    <path d="M12 2v6M9 2v4M15 2v4M18 8v5a6 6 0 0 1-12 0V8h12zM12 19v3" />
  </svg>
);
