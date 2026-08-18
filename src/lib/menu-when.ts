import { hostState } from '@/stores/host';

/**
 * 宿主可识别的菜单上下文。
 *
 * 这里只保留桌面白板的通用投影信息；产品领域变量由产品通过提供器按需注入。
 */
export interface MenuWhenContext {
  activeView: string;
  sidebarVisible: boolean;
  rightWorkspaceVisible: boolean;
  platform: string;
  /** 当前菜单/命令所属扩展标识。 */
  extensionId?: string | null;
  /** 产品提供的附加上下文，未知变量保持 fail-closed。 */
  [key: string]: string | number | boolean | null | undefined;
}

/** 产品注册菜单表达式所需的领域上下文，宿主不解释具体字段。 */
export type MenuWhenContextProvider = () => Record<string, string | number | boolean | null | undefined>;

let menuWhenContextProvider: MenuWhenContextProvider | undefined;

/** 注册当前产品的菜单领域上下文提供器。 */
export function registerMenuWhenContextProvider(provider: MenuWhenContextProvider): void {
  menuWhenContextProvider = provider;
}

type Token =
  | { kind: 'value'; value: string | number | boolean | null; resolved?: boolean }
  | { kind: 'operator'; value: '&&' | '||' | '==' | '!=' | '!' }
  | { kind: 'lparen' }
  | { kind: 'rparen' };

function contextValue(name: string, context: MenuWhenContext): string | number | boolean | null | undefined {
  return context[name.replace(/^\{|\}$/g, '')];
}

function tokenize(input: string): Token[] | null {
  const tokens: Token[] = [];
  let index = 0;
  while (index < input.length) {
    const char = input[index];
    if (/\s/.test(char)) {
      index += 1;
      continue;
    }
    if (char === '(') { tokens.push({ kind: 'lparen' }); index += 1; continue; }
    if (char === ')') { tokens.push({ kind: 'rparen' }); index += 1; continue; }
    if (char === '{') {
      const end = input.indexOf('}', index + 1);
      if (end < 0) return null;
      const value = input.slice(index + 1, end).trim();
      if (!value) return null;
      tokens.push({ kind: 'value', value });
      index = end + 1;
      continue;
    }
    const two = input.slice(index, index + 2);
    if (two === '&&' || two === '||' || two === '==' || two === '!=') {
      tokens.push({ kind: 'operator', value: two });
      index += 2;
      continue;
    }
    if (char === '!') { tokens.push({ kind: 'operator', value: '!' }); index += 1; continue; }
    if (char === "'" || char === '"') {
      const quote = char;
      let end = index + 1;
      let value = '';
      while (end < input.length && input[end] !== quote) {
        if (input[end] === '\\' && end + 1 < input.length) {
          value += input[end + 1];
          end += 2;
        } else {
          value += input[end];
          end += 1;
        }
      }
      if (end >= input.length) return null;
      tokens.push({ kind: 'value', value });
      index = end + 1;
      continue;
    }
    const number = input.slice(index).match(/^-?\d+(?:\.\d+)?/);
    if (number) {
      tokens.push({ kind: 'value', value: Number(number[0]), resolved: true });
      index += number[0].length;
      continue;
    }
    const identifier = input.slice(index).match(/^[A-Za-z_][A-Za-z0-9_.:-]*/);
    if (!identifier) return null;
    const raw = identifier[0];
    if (raw === 'true') tokens.push({ kind: 'value', value: true, resolved: true });
    else if (raw === 'false') tokens.push({ kind: 'value', value: false, resolved: true });
    else if (raw === 'null') tokens.push({ kind: 'value', value: null, resolved: true });
    else tokens.push({ kind: 'value', value: raw });
    index += raw.length;
  }
  return tokens;
}

class Parser {
  private index = 0;
  constructor(private readonly tokens: Token[], private readonly context: MenuWhenContext) {}

  parse(): boolean | null {
    const result = this.parseOr();
    return result !== null && this.index === this.tokens.length ? result : null;
  }

  private parseOr(): boolean | null {
    let result = this.parseAnd();
    while (this.matchOperator('||')) {
      const right = this.parseAnd();
      if (result === null || right === null) return null;
      result = result || right;
    }
    return result;
  }

  private parseAnd(): boolean | null {
    let result = this.parseUnary();
    while (this.matchOperator('&&')) {
      const right = this.parseUnary();
      if (result === null || right === null) return null;
      result = result && right;
    }
    return result;
  }

  private parseUnary(): boolean | null {
    if (this.matchOperator('!')) {
      const value = this.parseUnary();
      return value === null ? null : !value;
    }
    return this.parsePrimary();
  }

  private parsePrimary(): boolean | null {
    if (this.match('lparen')) {
      const value = this.parseOr();
      return this.match('rparen') && value !== null ? value : null;
    }
    const left = this.parseValue();
    if (left === undefined) return null;
    const operator = this.peekOperator();
    if (operator === '==' || operator === '!=') {
      this.index += 1;
      const right = this.parseValue();
      if (right === undefined) return null;
      return operator === '==' ? left === right : left !== right;
    }
    return typeof left === 'boolean' ? left : null;
  }

  private parseValue(): string | number | boolean | null | undefined {
    const token = this.tokens[this.index];
    if (!token || token.kind !== 'value') return undefined;
    this.index += 1;
    if (typeof token.value === 'string' && !token.resolved) return contextValue(token.value, this.context);
    return token.value;
  }

  private peekOperator(): '==' | '!=' | undefined {
    const token = this.tokens[this.index];
    return token?.kind === 'operator' && (token.value === '==' || token.value === '!=') ? token.value : undefined;
  }

  private match(kind: Token['kind']): boolean {
    if (this.tokens[this.index]?.kind !== kind) return false;
    this.index += 1;
    return true;
  }

  private matchOperator(value: '&&' | '||' | '==' | '!=' | '!'): boolean {
    const token = this.tokens[this.index];
    if (token?.kind !== 'operator' || token.value !== value) return false;
    this.index += 1;
    return true;
  }
}

export function getMenuWhenContext(): MenuWhenContext {
  return {
    ...menuWhenContextProvider?.(),
    activeView: hostState.activeView,
    sidebarVisible: hostState.sidebarVisible,
    rightWorkspaceVisible: hostState.rightWorkspaceVisible,
    platform: typeof navigator === 'undefined' ? 'unknown' : navigator.platform,
  };
}

export function evaluateMenuWhen(expression: string | null | undefined, context = getMenuWhenContext()): boolean {
  if (expression === undefined || expression === null || expression.trim() === '') return true;
  const tokens = tokenize(expression);
  if (!tokens) return false;
  return new Parser(tokens, context).parse() === true;
}

/**
 * 带扩展上下文的 when 求值。菜单/命令投影按 item.extensionId 传入，
 * 使 `{extensionId}` 可求值；未提供的上下文变量（如 `{worktreeId}`）保持 fail-closed（隐藏）。
 */
export function evaluateMenuWhenForExtension(
  expression: string | null | undefined,
  extensionId: string | null | undefined,
): boolean {
  return evaluateMenuWhen(expression, { ...getMenuWhenContext(), extensionId: extensionId ?? null });
}
