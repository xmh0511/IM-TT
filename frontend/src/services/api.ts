import { AuthResponse, User, Message, Group, GroupMember } from '../types';

const API_BASE_URL = 'http://localhost:8080/api';

class ApiService {
  private token: string | null = null;

  setToken(token: string) {
    this.token = token;
    localStorage.setItem('authToken', token);
  }

  getToken(): string | null {
    if (!this.token) {
      this.token = localStorage.getItem('authToken');
    }
    return this.token;
  }

  clearToken() {
    this.token = null;
    localStorage.removeItem('authToken');
  }

  private async request(endpoint: string, options: RequestInit = {}) {
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
      ...(options.headers as Record<string, string>),
    };

    if (this.token) {
      headers['Authorization'] = `Bearer ${this.token}`;
    }

    const response = await fetch(`${API_BASE_URL}${endpoint}`, {
      ...options,
      headers,
    });

    if (!response.ok) {
      const error = await response.json().catch(() => ({ error: 'Request failed' }));
      throw new Error(error.error || 'Request failed');
    }

    return response.json();
  }

  async register(username: string, email: string, password: string): Promise<AuthResponse> {
    const data = await this.request('/auth/register', {
      method: 'POST',
      body: JSON.stringify({ username, email, password }),
    });
    this.setToken(data.token);
    return data;
  }

  async login(email: string, password: string): Promise<AuthResponse> {
    const data = await this.request('/auth/login', {
      method: 'POST',
      body: JSON.stringify({ email, password }),
    });
    this.setToken(data.token);
    return data;
  }

  async getCurrentUser(): Promise<User> {
    return this.request('/user/me');
  }

  async sendMessage(receiverId: number | null, groupId: number | null, content: string): Promise<Message> {
    return this.request('/messages/send', {
      method: 'POST',
      body: JSON.stringify({
        receiver_id: receiverId,
        group_id: groupId,
        content,
        message_type: 'text',
      }),
    });
  }

  async getMessages(receiverId?: number, groupId?: number): Promise<Message[]> {
    const params = new URLSearchParams();
    if (receiverId) params.append('receiver_id', receiverId.toString());
    if (groupId) params.append('group_id', groupId.toString());
    return this.request(`/messages/list?${params}`);
  }

  async createGroup(name: string, description?: string): Promise<Group> {
    return this.request('/groups/create', {
      method: 'POST',
      body: JSON.stringify({ name, description }),
    });
  }

  async getUserGroups(): Promise<Group[]> {
    return this.request('/groups/list');
  }

  async getGroupMembers(groupId: number): Promise<GroupMember[]> {
    return this.request(`/groups/${groupId}/members`);
  }
}

export const apiService = new ApiService();
