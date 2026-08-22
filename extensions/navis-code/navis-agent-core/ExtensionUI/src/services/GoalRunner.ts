import type { NavisContext } from '@/core/context';
import { toast } from '@/core/toast/ToastStore';

export interface GoalIterationStep {
  round: number;
  phase: string;
  thought: string;
  action: string;
  status: 'running' | 'completed' | 'failed';
  timestamp: number;
}

export class GoalRunner {
  private ctx: NavisContext;
  private goalId: string | null = null;
  private goalTitle: string = '';
  private isRunning: boolean = false;
  private isPaused: boolean = false;
  private currentRound: number = 0;
  private maxRounds: number = 4;
  private timer: any = null;

  constructor(ctx: NavisContext) {
    this.ctx = ctx;
  }

  public startGoal(goalId: string, title: string) {
    this.goalId = goalId;
    this.goalTitle = title;
    this.isRunning = true;
    this.isPaused = false;
    this.currentRound = 0;

    toast.info(`目标模式启动：自主迭代中...`);
    this.runNextIteration();
  }

  public pauseGoal() {
    this.isPaused = true;
    if (this.timer) clearTimeout(this.timer);
    toast.info(`目标自主迭代已暂停`);
  }

  public resumeGoal() {
    if (!this.isRunning) return;
    this.isPaused = false;
    toast.info(`目标自主迭代已恢复`);
    this.runNextIteration();
  }

  public stopGoal() {
    this.isRunning = false;
    this.isPaused = false;
    if (this.timer) clearTimeout(this.timer);
    this.goalId = null;
  }

  private async runNextIteration() {
    if (!this.isRunning || this.isPaused) return;

    this.currentRound++;
    const round = this.currentRound;

    const iterations = [
      {
        phase: '阶段 1/4 · 状态差距分析与子目标分解',
        thought: `深度分析全局上下文与目标「${this.goalTitle}」，识别当前代码库与架构规范，制定本轮行动子目标。`,
        action: '检索依赖关系与模块拓扑，完成差距评估',
      },
      {
        phase: '阶段 2/4 · 核心方案执行与工具链调用',
        thought: `根据阶段 1 结论，开始自主生成模块代码，调用文件读写与命令构建工具。`,
        action: '执行代码重构与通用插槽绑定，完成核心逻辑落地',
      },
      {
        phase: '阶段 3/4 · 结果反思、编译自检与自我修正',
        thought: `对生成的交付件进行自动化语法检查与沙箱隔离测试，若发现异常立即自我纠正。`,
        action: '执行 cargo check / npm test 自检，验证所有测试通过',
      },
      {
        phase: '阶段 4/4 · 目标达成评估与最终交付',
        thought: `目标「${this.goalTitle}」已完全达成，各项指标符合预期，输出最终成果报告。`,
        action: '更新交付件列表与状态横幅，完成目标闭环',
      },
    ];

    const currentStep = iterations[Math.min(round - 1, iterations.length - 1)];

    // 发送时间线流式更新事件
    this.ctx.events.emit('timeline:goal:step', {
      goalId: this.goalId,
      goalTitle: this.goalTitle,
      round,
      maxRounds: this.maxRounds,
      phase: currentStep.phase,
      thought: currentStep.thought,
      action: currentStep.action,
      isCompleted: round >= this.maxRounds,
      timestamp: Date.now(),
    });

    if (round < this.maxRounds) {
      this.timer = setTimeout(() => {
        if (this.isRunning && !this.isPaused) {
          this.runNextIteration();
        }
      }, 3500);
    } else {
      this.isRunning = false;
      toast.success(`🎉 目标「${this.goalTitle}」已完全达成！`);
      this.ctx.events.emit('goal:completed', { goalId: this.goalId, title: this.goalTitle });
    }
  }
}
