import { WsEvent } from '../types';

const WS_URL = 'ws://localhost:8080/api/ws';

export class WebSocketService {
  private ws: WebSocket | null = null;
  private listeners: Array<(event: WsEvent) => void> = [];

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
        this.listeners.forEach(listener => listener(data));
      } catch (error) {
        console.error('Error parsing WebSocket message:', error);
      }
    };

    this.ws.onclose = () => {
      console.log('WebSocket disconnected');
      // Reconnect after 3 seconds
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
}

export const wsService = new WebSocketService();
