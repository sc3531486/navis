/* @refresh reload */
import { FrameworkLifecycle } from './bootstrap';
import { HostViewSurface } from './components/HostView';

export default function App() {
  return (
    <FrameworkLifecycle>
      <div class="flex h-screen w-screen items-center justify-center bg-white text-[#242424]">
        <HostViewSurface zone="main" title="Extensions" />
      </div>
    </FrameworkLifecycle>
  );
}
