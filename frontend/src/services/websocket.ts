import { WsEvent } from '../types';

const WS_URL = import.meta.env.VITE_WS_URL || 'ws://localhost:8080/api/ws';

export class WebSocketService {
  private ws: WebSocket | null = null;
  private listeners: Array<(event: WsEvent) => void> = [];
  private onlineUsers: Set<number> = new Set();
  private onlineListeners: Array<(users: Set<number>) => void> = [];

  connect(token: string) {
    if (this.ws) {
      this.ws.close();
    }

    this.ws = new WebSocket(`${WS_URL}?token=${token}`);

    this.ws.onopen = () => {
      console.log('WebSocket connected');
    };

    this.ws.onmessage = (event) => {
      try {
        const data: WsEvent = JSON.parse(event.data);

        if (data.event_type === 'online') {
          this.onlineUsers.add(data.user_id);
          this.notifyOnlineUsers();
        } else if (data.event_type === 'offline') {
          this.onlineUsers.delete(data.user_id);
          this.notifyOnlineUsers();
        }

        this.listeners.forEach(listener => listener(data));
      } catch (error) {
        console.error('Error parsing WebSocket message:', error);
      }
    };

    this.ws.onclose = () => {
      console.log('WebSocket disconnected');
      this.onlineUsers.clear();
      this.notifyOnlineUsers();
      setTimeout(() => this.connect(token), 3000);
    };

    this.ws.onerror = (error) => {
      console.error('WebSocket error:', error);
    };
  }

  disconnect() {
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
    this.onlineUsers.clear();
  }

  send(event: WsEvent) {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(event));
    }
  }

  addListener(listener: (event: WsEvent) => void) {
    this.listeners.push(listener);
  }

  removeListener(listener: (event: WsEvent) => void) {
    this.listeners = this.listeners.filter(l => l !== listener);
  }

  isUserOnline(userId: number): boolean {
    return this.onlineUsers.has(userId);
  }

  addOnlineListener(listener: (users: Set<number>) => void) {
    this.onlineListeners.push(listener);
  }

  removeOnlineListener(listener: (users: Set<number>) => void) {
    this.onlineListeners = this.onlineListeners.filter(l => l !== listener);
  }

  private notifyOnlineUsers() {
    this.onlineListeners.forEach(listener => listener(new Set(this.onlineUsers)));
  }
}

export const wsService = new WebSocketService();
