export interface ComposerInstructionFlags {
  planModeEnabled: boolean;
  multiAgentEnabled: boolean;
}

export function planModeInstruction(): string {
  return [
    '<navis_plan_mode enabled="true">',
    'Planning gate: do not execute the requested work yet. If key details are unclear, ask concise clarification questions and provide concrete choices. Also allow the user to give custom text. If enough information is available, write a concise plan document and immediately update the session todo list with one todo per phase, using status pending/in_progress/completed. Stop after the plan is written and wait for the user to start execution.',
    '</navis_plan_mode>',
  ].join('\n');
}

export function planExecutionInstruction(requestText: string, customText: string): string {
  const custom = customText.trim();
  return [
    '<navis_plan_execution approved="true">',
    'The user approved the plan. Execute the approved plan now. Keep the session todo phase list current as phases start and complete.',
    custom ? `User custom instruction: ${custom}` : '',
    '</navis_plan_execution>',
    '',
    `Original request:\n${requestText}`,
  ].filter(Boolean).join('\n');
}

export function multiAgentInstruction(): string {
  return [
    '<navis_multi_agent enabled="true">',
    'Before executing, decompose the request into independent subtasks. Assign suitable independent work to child agents with the task tool. Keep dependent or final integration work in the main agent. Do not ask the user for approval to create child agents.',
    '</navis_multi_agent>',
  ].join('\n');
}

export function modeInstructionMessage(text: string, flags: ComposerInstructionFlags): string {
  const instructions = [
    ...(flags.planModeEnabled ? [planModeInstruction()] : []),
    ...(flags.multiAgentEnabled ? [multiAgentInstruction()] : []),
  ];
  if (instructions.length === 0) return text;
  return [...instructions, '', text].join('\n');
}

export function goalTaskMessage(
  goalText: string | null | undefined,
  taskText: string,
  flags: ComposerInstructionFlags,
): string {
  const goal = goalText?.trim();
  if (!goal) return modeInstructionMessage(taskText, flags);

  const guidance = taskText.trim();
  const text = [
    `Goal: ${goal}`,
    guidance && guidance !== goal ? `User guidance: ${guidance}` : '',
    'Continue pursuing this goal now. Decide and execute the next useful step. Do not ask the user for the next step unless blocked.',
  ].filter(Boolean).join('\n\n');
  return modeInstructionMessage(text, flags);
}

export function goalTaskMessageForGoal(
  goalText: string,
  taskText: string,
  flags: ComposerInstructionFlags,
): string {
  const goal = goalText.trim();
  if (!goal) return modeInstructionMessage(taskText, flags);
  const guidance = taskText.trim();
  return modeInstructionMessage([
    `Goal: ${goal}`,
    guidance && guidance !== goal ? `User guidance: ${guidance}` : '',
    'Continue pursuing this goal now. Decide and execute the next useful step. Do not ask the user for the next step unless blocked.',
  ].filter(Boolean).join('\n\n'), flags);
}
