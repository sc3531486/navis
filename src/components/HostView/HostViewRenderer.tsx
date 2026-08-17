import { Component, Show, lazy } from 'solid-js';
import type { RightWorkspacePanel } from '../../stores/app';
import { getHostViewRendererDescriptor } from './registry';
import type { HostViewRendererProps } from './types';

const BackgroundTasksPanel = lazy(() => import('../WorkspacePanel/BackgroundTasksPanel'));

/**
 * 宿主内置视图投影（业务即扩展路径的宿主侧承接）：
 * 扩展 manifest 以 host:panel 声明视图后，宿主按 viewId 白名单映射到内置组件；
 * 未命中的视图仍走 registry 渲染器（host:panel / html:sandbox）。
 */
const BUILTIN_VIEW_PROJECTIONS: Record<string, Component<HostViewRendererProps>> = {
  'navis-task.panel': (props) => {
    const panel: RightWorkspacePanel = {
      id: props.view.viewId,
      title: props.view.name,
      viewId: props.view.viewId,
      config: props.view.config,
      extensionView: props.view,
    };
    return <BackgroundTasksPanel panel={panel} />;
  },
};

const resolveHostViewRenderer = (props: HostViewRendererProps): Component<HostViewRendererProps> | undefined =>
  BUILTIN_VIEW_PROJECTIONS[props.view.viewId]
  ?? getHostViewRendererDescriptor(props.view.renderer)?.component;

const HostViewRenderer: Component<HostViewRendererProps> = (props) => (
  <Show
    when={resolveHostViewRenderer(props)}
    fallback={<div class="navis-host-view-empty"><strong>Renderer unavailable</strong><span>{props.view.renderer}</span></div>}
  >
    {(descriptor) => {
      const Renderer = descriptor();
      return <Renderer view={props.view} surface={props.surface} />;
    }}
  </Show>
);

export default HostViewRenderer;
