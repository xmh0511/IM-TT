export interface User {
  id: number;
  username: string;
  email: string;
  avatar?: string;
  status: string;
  created_at: string;
  updated_at: string;
}

export interface Message {
  id: number;
  sender_id: number;
  receiver_id?: number;
  group_id?: number;
  content: string;
  message_type: string;
  created_at: string;
  is_read: boolean;
}

export interface Group {
  id: number;
  name: string;
  description?: string;
  avatar?: string;
  owner_id: number;
  created_at: string;
  updated_at: string;
}

export interface GroupMember {
  id: number;
  group_id: number;
  user_id: number;
  role: string;
  joined_at: string;
}

export interface AuthResponse {
  token: string;
  user: User;
}

export interface WsEvent {
  event_type: string;
  user_id: number;
  receiver_id?: number;
  group_id?: number;
  content?: string;
  data?: any;
}
