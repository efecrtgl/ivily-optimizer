// Bildirim merkezi: uygulama içi bildirimlerin kısa geçmişi.
// Yalnızca bu bilgisayarda (tarayıcı deposunda) tutulur; en fazla 50 kayıt.

export interface NotificationItem {
  id: number;
  ts: number; // unix saniye
  kind: "success" | "error" | "info" | "warning";
  title: string;
  body?: string;
  read: boolean;
}

const KEY = "ivily.notifications.v1";

function load(): NotificationItem[] {
  try {
    const raw = localStorage.getItem(KEY);
    const parsed = raw ? JSON.parse(raw) : [];
    return Array.isArray(parsed) ? parsed.slice(0, 50) : [];
  } catch {
    return [];
  }
}

export const notifications = $state({ items: load(), unread: 0 });
notifications.unread = notifications.items.filter((n) => !n.read).length;

function persist() {
  try {
    localStorage.setItem(KEY, JSON.stringify(notifications.items.slice(0, 50)));
  } catch {
    /* depolama kullanılamıyorsa geçmiş yalnızca bu oturumda kalır */
  }
}

export function pushNotification(kind: NotificationItem["kind"], title: string, body?: string) {
  const id = Date.now() * 1000 + Math.floor(Math.random() * 1000);
  notifications.items.unshift({ id, ts: Math.floor(Date.now() / 1000), kind, title, body, read: false });
  if (notifications.items.length > 50) notifications.items.length = 50;
  notifications.unread = notifications.items.filter((n) => !n.read).length;
  persist();
}

export function markAllRead() {
  notifications.items.forEach((n) => (n.read = true));
  notifications.unread = 0;
  persist();
}

export function clearNotifications() {
  notifications.items.length = 0;
  notifications.unread = 0;
  persist();
}
