export interface ConfirmOptions {
  title: string;
  message: string;
  targetName?: string;
  confirmText?: string;
  cancelText?: string;
  variant?: 'danger' | 'warning' | 'primary';
}

interface ActiveConfirm extends ConfirmOptions {
  resolve: (value: boolean) => void;
}

let active = $state<ActiveConfirm | null>(null);

export function confirmDialog(options: ConfirmOptions): Promise<boolean> {
  return new Promise((resolve) => {
    active = { ...options, resolve };
  });
}

export function resolveConfirm(result: boolean) {
  if (active) {
    const fn = active.resolve;
    active = null;
    fn(result);
  }
}

export const confirmStore = {
  get current() {
    return active;
  },
};
