export type ToastVariant = 'success' | 'error' | 'warning' | 'info';

export interface ToastItem {
  id: number;
  variant: ToastVariant;
  title: string;
  description?: string;
}

let nextId = 1;
let items = $state<ToastItem[]>([]);

function push(variant: ToastVariant, title: string, description?: string, durationMs = 4000) {
  const id = nextId++;
  items = [...items, { id, variant, title, description }];
  if (durationMs > 0) {
    setTimeout(() => {
      dismiss(id);
    }, durationMs);
  }
}

export function dismiss(id: number) {
  items = items.filter((t) => t.id !== id);
}

export const toast = {
  get items() {
    return items;
  },
  dismiss,
  success: (title: string, description?: string) => push('success', title, description, 3500),
  error: (title: string, description?: string) => push('error', title, description, 5500),
  warning: (title: string, description?: string) => push('warning', title, description, 4500),
  info: (title: string, description?: string) => push('info', title, description, 3500),
};
