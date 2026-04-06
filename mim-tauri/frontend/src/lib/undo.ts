import { writable, derived } from 'svelte/store';

export interface UndoAction {
  type: string;
  forward: () => Promise<void>;
  backward: () => Promise<void>;
  description: string;
}

const MAX_STACK_SIZE = 50;

export const undoStack = writable<UndoAction[]>([]);
export const redoStack = writable<UndoAction[]>([]);

export const canUndo = derived(undoStack, ($stack) => $stack.length > 0);
export const canRedo = derived(redoStack, ($stack) => $stack.length > 0);
export const lastUndoDescription = derived(undoStack, ($stack) =>
  $stack.length > 0 ? $stack[$stack.length - 1].description : ''
);

export function pushAction(action: UndoAction) {
  undoStack.update((stack) => {
    const next = [...stack, action];
    if (next.length > MAX_STACK_SIZE) {
      next.shift();
    }
    return next;
  });
  // Clear redo stack when a new action is pushed
  redoStack.set([]);
}

export async function undo() {
  let action: UndoAction | undefined;
  undoStack.update((stack) => {
    const next = [...stack];
    action = next.pop();
    return next;
  });
  if (action) {
    try {
      await action.backward();
    } catch (e) {
      console.error('Undo failed:', e);
    }
    redoStack.update((stack) => {
      const next = [...stack, action!];
      if (next.length > MAX_STACK_SIZE) {
        next.shift();
      }
      return next;
    });
  }
}

export async function redo() {
  let action: UndoAction | undefined;
  redoStack.update((stack) => {
    const next = [...stack];
    action = next.pop();
    return next;
  });
  if (action) {
    try {
      await action.forward();
    } catch (e) {
      console.error('Redo failed:', e);
    }
    undoStack.update((stack) => {
      const next = [...stack, action!];
      if (next.length > MAX_STACK_SIZE) {
        next.shift();
      }
      return next;
    });
  }
}
