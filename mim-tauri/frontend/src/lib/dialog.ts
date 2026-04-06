import { writable, get } from 'svelte/store';

export interface DialogConfig {
  show: boolean;
  title: string;
  message: string;
  type: 'confirm' | 'prompt' | 'password' | 'alert';
  placeholder: string;
  defaultValue: string;
  confirmText: string;
  cancelText: string;
  dangerous: boolean;
  resolve: (value: string | boolean | null) => void;
}

export const dialogState = writable<DialogConfig>({
  show: false,
  title: '',
  message: '',
  type: 'confirm',
  placeholder: '',
  defaultValue: '',
  confirmText: 'OK',
  cancelText: 'Cancel',
  dangerous: false,
  resolve: () => {},
});

function showDialog(config: Partial<DialogConfig>): Promise<string | boolean | null> {
  return new Promise((resolve) => {
    dialogState.set({
      show: true,
      title: config.title || 'Confirm',
      message: config.message || '',
      type: config.type || 'confirm',
      placeholder: config.placeholder || '',
      defaultValue: config.defaultValue || '',
      confirmText: config.confirmText || 'OK',
      cancelText: config.cancelText || 'Cancel',
      dangerous: config.dangerous || false,
      resolve,
    });
  });
}

/** Themed replacement for window.confirm() */
export async function appConfirm(message: string, title = 'Confirm'): Promise<boolean> {
  const result = await showDialog({ type: 'confirm', title, message });
  return result === true;
}

/** Themed replacement for window.prompt() */
export async function appPrompt(message: string, defaultValue = '', title = 'Input'): Promise<string | null> {
  const result = await showDialog({ type: 'prompt', title, message, placeholder: message, defaultValue });
  return typeof result === 'string' ? result : null;
}

/** Themed replacement for window.alert() */
export async function appAlert(message: string, title = 'Notice'): Promise<void> {
  await showDialog({ type: 'alert', title, message });
}

/** Password prompt */
export async function appPassword(title: string, message = ''): Promise<string | null> {
  const result = await showDialog({ type: 'password', title, message, placeholder: 'Password' });
  return typeof result === 'string' ? result : null;
}

/** Dangerous confirm (red button) */
export async function appDangerConfirm(message: string, title = 'Warning'): Promise<boolean> {
  const result = await showDialog({ type: 'confirm', title, message, dangerous: true, confirmText: 'Delete' });
  return result === true;
}
